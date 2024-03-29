use crate::base::MagicBytes;
use crate::fse::FseBackend;
use crate::lmd::{DMax, MatchDistance};
use crate::match_kit;
use crate::raw::{self, RAW_HEADER_SIZE};
use crate::types::{Idx, ShortWriter};
use crate::vn::VnBackend;

use super::backend::Backend;
use super::backend_type::BackendType;
use super::constants::*;
use super::history::{History, HistoryTable, Item};
use super::match_object::Match;
use super::match_unit::MatchUnit;

use std::io;
use std::mem;

// Fixed constant. Do NOT change.
const SLACK: u32 = 0x1000_0000;

// Fixed constant. Do NOT change.
const BLOCK_GUIDE: u32 = 0x7FFF_FFFF;

pub struct FrontendBytes<'a> {
    table: &'a mut HistoryTable,
    src: &'a [u8],
    block: &'a [u8],
    pending: Match,
    literal_index: u32,
    index: u32,
}

impl<'a> FrontendBytes<'a> {
    #[inline(always)]
    pub fn new(table: &'a mut HistoryTable, src: &'a [u8]) -> Self {
        Self { table, src, block: &[], pending: Match::default(), literal_index: 0, index: 0 }
    }

    #[inline(always)]
    pub fn execute<O>(&mut self, backend: &mut FseBackend, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        self.init();
        self.flush(backend, dst)?;
        Ok(())
    }

    fn flush<O>(&mut self, backend: &mut FseBackend, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        // Select.
        self.flush_select(backend, dst)?;
        debug_assert_eq!(self.literal_index as usize, self.src.len());
        // Eos.
        dst.write_short_u32(MagicBytes::Eos.into())?;
        dst.flush(true)?;
        Ok(())
    }

    fn flush_select<O>(&mut self, backend: &mut FseBackend, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        let len = self.src.len();
        if len > VN_CUTOFF as usize {
            // Fse
            self.flush_backend::<_, _, false>(backend, dst)
        } else if len > RAW_CUTOFF as usize {
            // Vn
            self.flush_backend::<_, _, true>(&mut VnBackend::default(), dst)
        } else {
            self.flush_raw(dst)
        }
    }

    fn flush_backend<B, O, const VN: bool>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
    ) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        let src_len = self.src.len();
        let mark = dst.pos();
        backend.init(dst, Some(src_len))?;
        self.finalize(backend, dst)?;
        if VN && src_len < RAW_LIMIT as usize {
            let dst_len = (dst.pos() - mark) as usize;
            if src_len + RAW_HEADER_SIZE as usize <= dst_len && dst.truncate(mark) {
                // The compressed length is NOT shorter than raw block length AND we have a
                // successful truncate, so we proceed to rework as a raw block.
                self.flush_raw(dst)?;
            }
        }
        Ok(())
    }

    fn flush_raw<O>(&mut self, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        assert!(self.src.len() <= i32::MAX as usize);
        raw::raw_compress(dst, self.src)?;
        self.literal_index = self.src.len() as u32;
        Ok(())
    }

    fn init(&mut self) {
        self.table.reset();
        self.block = &[];
        self.pending = Match::default();
        self.literal_index = 0;
        self.index = 0;
    }

    fn finalize<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        self.match_blocks(backend, dst)?;
        self.flush_pending(backend, dst)?;
        self.flush_literals(backend, dst)?;
        backend.finalize(dst)?;
        Ok(())
    }

    fn match_blocks<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        debug_assert!(self.is_init());
        while self.match_block(backend, dst)? {}
        Ok(())
    }

    fn match_block<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<bool>
    where
        B: Backend,
        O: ShortWriter,
    {
        Ok({
            if self.match_any(backend, dst)? {
                false
            } else {
                self.reposition(backend, dst)?;
                true
            }
        })
    }

    #[allow(clippy::absurd_extreme_comparisons)]
    #[allow(clippy::assertions_on_constants)]
    fn match_any<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<bool>
    where
        B: Backend,
        O: ShortWriter,
    {
        assert!(BLOCK_GUIDE <= i32::MAX as u32);
        assert!(256 <= SLACK);
        assert!(SLACK * 2 <= BLOCK_GUIDE);
        assert!(self.src.len() >= 4);
        debug_assert!(self.is_any::<B::Type>());
        let mut index = self.index;
        let is_short = if self.src.len() <= BLOCK_GUIDE as usize + 3 {
            self.block = &self.src[..self.src.len()];
            self.index = self.block.len() as u32 - 3;
            true
        } else {
            self.block = &self.src[..BLOCK_GUIDE as usize];
            self.index = self.block.len() as u32 - SLACK - 3;
            false
        };
        assert!(index < self.index);
        assert!(self.index <= BLOCK_GUIDE);
        assert!(self.index as usize + mem::size_of::<u32>() <= self.block.len() + 1);
        loop {
            // Hot loop.
            let val = unsafe { get_u32(self.block, index) };
            let item = Item::new(val, index.into());
            let queue = self.table.push::<B::Type>(item);
            let incoming = unsafe { self.find_match::<B::Type>(queue, item) };
            if let Some(select) = self.pending.select::<GOOD_MATCH_LEN>(incoming) {
                unsafe { self.push_match(backend, dst, select)? };
                if self.literal_index >= self.index {
                    // Unlikely.
                    break;
                }
                index += 1;
                index = unsafe { self.sync_history::<B::Type>(index) };
                if index >= self.index {
                    // Unlikely
                    break;
                }
            } else {
                index += 1;
                if index == self.index {
                    // Unlikely
                    break;
                }
            }
        }
        debug_assert!(self.is_any_post::<B::Type>());
        Ok(is_short)
    }

    #[inline(always)]
    unsafe fn find_match<B>(&self, queue: History, item: Item) -> Match
    where
        B: BackendType,
    {
        let mut m = Match::default();
        for &match_idx_val in queue.iter() {
            let distance = (item.idx - match_idx_val.idx) as u32;
            debug_assert!(distance <= Q2);
            if distance > B::MAX_MATCH_DISTANCE {
                break;
            }
            let match_len_inc = self.match_unit::<B>(item, match_idx_val);
            if match_len_inc > m.match_len {
                m.match_len = match_len_inc;
                m.match_idx = match_idx_val.idx;
            }
        }
        if m.match_len == 0 {
            // Likely.
            m
        } else {
            // Unlikely.
            m.idx = item.idx;
            let match_len_dec = self.match_dec::<B>(m.idx, m.match_idx);
            m.idx -= match_len_dec;
            m.match_idx -= match_len_dec;
            m.match_len += match_len_dec;
            debug_assert!(self.validate_match::<B>(m));
            m
        }
    }

    #[inline(always)]
    unsafe fn match_unit<M: MatchUnit>(&self, item: Item, match_item: Item) -> u32 {
        debug_assert!(self.validate_match_items::<M>(item, match_item));
        let len = M::match_us((item.val, match_item.val));
        if len == 4 {
            let index = usize::from(item.idx);
            let match_index = usize::from(match_item.idx);
            let max = self.block.len() - index;
            match_kit::fast_match_inc_unchecked(self.block, index, match_index, 4, max) as u32
        } else {
            len
        }
    }

    #[inline(always)]
    unsafe fn match_dec<M: MatchUnit>(&self, idx: Idx, match_idx: Idx) -> u32 {
        debug_assert!(self.validate_match_idxs::<M>(idx, match_idx));
        let index = usize::from(idx);
        let match_index = usize::from(match_idx);
        let literal_len = usize::from(idx) - self.literal_index as usize;
        let max = (literal_len).min(match_index);
        match_kit::fast_match_dec_unchecked(self.block, index, match_index, max) as u32
    }

    #[inline(always)]
    fn flush_pending<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
    ) -> io::Result<()> {
        debug_assert!(self.block.len() <= BLOCK_GUIDE as usize + 3);
        if self.pending.match_len != 0 {
            assert!(self.literal_index as usize <= usize::from(self.pending.idx));
            assert!(usize::from(self.pending.idx) <= self.block.len());
            unsafe { self.push_match(backend, dst, self.pending)? };
            self.pending.match_len = 0;
        }
        Ok(())
    }

    #[inline(always)]
    unsafe fn push_match<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
        m: Match,
    ) -> io::Result<()> {
        debug_assert!(self.validate_match::<B::Type>(m));
        let match_distance = MatchDistance::new_unchecked((m.idx - m.match_idx) as u32);
        let literal_index = self.literal_index as usize;
        let match_index = usize::from(m.idx);
        debug_assert!(literal_index <= self.block.len());
        debug_assert!(match_index <= self.block.len());
        let literals = self.block.get_unchecked(literal_index..match_index);
        self.literal_index = u32::from(m.idx) + m.match_len;
        backend.push_match(dst, literals, m.match_len, match_distance)
    }

    #[inline(always)]
    fn flush_literals<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
    ) -> io::Result<()> {
        debug_assert!(self.block.len() <= BLOCK_GUIDE as usize + 3);
        assert!(self.literal_index as usize <= self.block.len());
        let len = self.block.len() as u32 - self.literal_index;
        if len != 0 {
            unsafe { self.push_literals(backend, dst, len)? };
        }
        Ok(())
    }

    unsafe fn push_literals<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
        len: u32,
    ) -> io::Result<()> {
        debug_assert_ne!(len, 0);
        debug_assert_eq!(self.pending.match_len, 0);
        debug_assert!(self.literal_index as usize + len as usize <= self.block.len());
        let literal_index = self.literal_index as usize;
        let literals = self.block.get_unchecked(literal_index..literal_index + len as usize);
        self.literal_index += len;
        backend.push_literals(dst, literals)
    }

    #[inline(always)]
    #[must_use]
    unsafe fn sync_history<B: BackendType>(&mut self, mut index: u32) -> u32 {
        while index < self.literal_index {
            let val = get_u32(self.src, index);
            let item = Item::new(val, index.into());
            self.table.push::<B>(item);
            index += 1;
        }
        index
    }

    #[allow(clippy::absurd_extreme_comparisons)]
    #[allow(clippy::assertions_on_constants)]
    fn reposition<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        assert!(BLOCK_GUIDE <= i32::MAX as u32);
        assert!(self.literal_index <= BLOCK_GUIDE);
        assert!(self.literal_index as usize + mem::size_of::<u32>() <= self.src.len() + 1);
        self.index = unsafe { self.sync_history::<B::Type>(self.index) };
        assert!(B::Type::MAX_MATCH_DISTANCE <= self.index);
        let delta = self.index - B::Type::MAX_MATCH_DISTANCE;
        if self.literal_index < delta {
            // We have literals that have have passed our buffer head without a match, we'll
            // push them as is.
            // We could push pending, but we don't, we discard it. The compression loss is
            // negligible, at most we lose `GOOD_MATCH - 1` bytes in a situation with a low
            // probability of occurrence. Instead we take the reduction in code complexity/
            // size.
            self.pending.match_len = 0;
            unsafe { self.push_literals(backend, dst, delta - self.literal_index)? };
        }
        self.table.clamp_rebias(self.index.into(), delta);
        self.pending.rebias(delta);
        self.src = &self.src[delta as usize..];
        self.literal_index -= delta;
        self.index -= delta;
        Ok(())
    }

    fn is_init(&self) -> bool {
        self.block.is_empty()
            && self.pending == Match::default()
            && self.literal_index == 0
            && self.index == 0
    }

    fn is_any<B: BackendType>(&self) -> bool {
        self.literal_index <= self.index
            && (self.index == 0 || self.index == B::MAX_MATCH_DISTANCE)
            && self.src.len() >= 4 + self.index as usize
    }

    fn is_any_post<B: BackendType>(&self) -> bool {
        self.literal_index as usize <= self.block.len()
            && self.index as usize <= self.block.len() - 3
            && self.block.len() <= self.src.len()
    }

    fn validate_match<B: BackendType>(&self, m: Match) -> bool {
        m.match_len != 0
            && m.match_len >= B::MATCH_UNIT
            && self.literal_index <= m.idx.into()
            && m.match_idx < m.idx
            && (m.idx - m.match_idx) as u32 <= B::MAX_MATCH_DISTANCE
            && usize::from(m.idx + m.match_len) <= self.block.len()
    }

    unsafe fn validate_match_items<M: MatchUnit>(&self, item: Item, match_item: Item) -> bool {
        self.validate_match_idxs::<M>(item.idx, match_item.idx)
            && (item.val ^ get_u32(self.block, item.idx.into())) & M::MATCH_MASK == 0
            && (match_item.val ^ get_u32(self.block, match_item.idx.into())) & M::MATCH_MASK == 0
    }

    fn validate_match_idxs<M: MatchUnit>(&self, idx: Idx, match_idx: Idx) -> bool {
        match_idx < idx && usize::from(idx) <= self.block.len() - M::MATCH_UNIT as usize
    }
}

#[inline(always)]
unsafe fn get_u32(bytes: &[u8], index: u32) -> u32 {
    debug_assert!(index as usize + mem::size_of::<u32>() <= bytes.len());
    bytes.as_ptr().add(index as usize).cast::<u32>().read_unaligned()
}

#[cfg(test)]
mod tests {
    use test_kit::Rng;

    use crate::lmd::Lmd;
    use crate::ops::PeekData;

    use super::super::dummy::{Dummy, DummyBackend};
    use super::*;

    use std::convert::TryFrom;

    fn compress(src: &[u8]) -> io::Result<Vec<u8>> {
        let mut table = HistoryTable::default();
        let mut backend = FseBackend::default();
        let mut frontend = FrontendBytes::new(&mut table, src);
        let mut dst = Vec::with_capacity(32 + src.len() * 2);
        frontend.execute(&mut backend, &mut dst)?;
        Ok(dst)
    }

    fn check_output(src: &[u8], expected: &[u8]) -> io::Result<()> {
        let dst = compress(src)?;
        assert_eq!(dst, expected);
        Ok(())
    }

    fn check_magic(src: &[u8], expected: MagicBytes) -> io::Result<()> {
        assert_eq!(MagicBytes::try_from(compress(src)?.peek_u32())?, expected);
        Ok(())
    }

    // Raw, assumes the defaults (RAW_CUTOFF: 0x0014, VN_CUTOFF: 0x1000)
    #[test]
    fn zero_0() -> io::Result<()> {
        check_output(
            &[0; 0],
            &[0x62, 0x76, 0x78, 0x2D, 0x00, 0x00, 0x00, 0x00, 0x62, 0x76, 0x78, 0x24],
        )
    }

    // Raw, assumes the defaults (RAW_CUTOFF: 0x0014, VN_CUTOFF: 0x1000)
    #[test]
    fn zero_1() -> io::Result<()> {
        check_output(
            &[0; 1],
            &[0x62, 0x76, 0x78, 0x2D, 0x01, 0x00, 0x00, 0x00, 0x00, 0x62, 0x76, 0x78, 0x24],
        )
    }

    // Raw, assumes the defaults (RAW_CUTOFF: 0x0014, VN_CUTOFF: 0x1000)
    #[test]
    fn zero_20() -> io::Result<()> {
        check_output(
            &[0; 20],
            &[
                0x62, 0x76, 0x78, 0x2D, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x62, 0x76, 0x78, 0x24,
            ],
        )
    }

    // Raw, assumes the defaults (RAW_CUTOFF: 0x0014, VN_CUTOFF: 0x1000)
    #[test]
    fn zero_21() -> io::Result<()> {
        check_output(
            &[0; 21],
            &[
                0x62, 0x76, 0x78, 0x6E, 0x15, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x68, 0x01,
                0x00, 0xFC, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x62, 0x76, 0x78, 0x24,
            ],
        )
    }

    // Vxn, assumes the defaults (RAW_CUTOFF: 0x0014, VN_CUTOFF: 0x1000)
    #[test]
    fn zero_4096() -> io::Result<()> {
        check_output(
            &[0; 4096],
            &[
                0x62, 0x76, 0x78, 0x6E, 0x00, 0x10, 0x00, 0x00, 0x2B, 0x00, 0x00, 0x00, 0x68, 0x01,
                0x00, 0xF0, 0xFF, 0xF0, 0xFF, 0xF0, 0xFF, 0xF0, 0xFF, 0xF0, 0xFF, 0xF0, 0xFF, 0xF0,
                0xFF, 0xF0, 0xFF, 0xF0, 0xFF, 0xF0, 0xFF, 0xF0, 0xFF, 0xF0, 0xFF, 0xF0, 0xFF, 0xF0,
                0xFF, 0xF0, 0xFF, 0xF0, 0x06, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x62,
                0x76, 0x78, 0x24,
            ],
        )
    }

    // Vx2, assumes the defaults (RAW_CUTOFF: 0x0014, VN_CUTOFF: 0x1000)
    #[test]
    fn zero_4097() -> io::Result<()> {
        check_output(
            &[0; 4097],
            &[
                0x62, 0x76, 0x78, 0x32, 0x01, 0x10, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x02,
                0x00, 0x70, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x10, 0x83, 0x00, 0x00, 0x00,
                0x20, 0x00, 0x00, 0x08, 0x8F, 0xC0, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0xC0, 0xA3, 0xF0, 0x68, 0x3C, 0x1A, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0xE8, 0x03, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x22,
                0xCB, 0xFF, 0x01, 0x62, 0x76, 0x78, 0x24,
            ],
        )
    }

    // Random non-compressible data (<= VN_CUTOFF) should fall back to a Raw block (more efficient).
    #[test]
    fn rand_vn_cutoff() -> io::Result<()> {
        check_magic(Rng::default().gen_vec(VN_CUTOFF as usize).unwrap().as_ref(), MagicBytes::Raw)
    }

    // Random non-compressible data (> VN_CUTOFF) does not fall back to a Raw block.
    #[test]
    fn rand_vn_cutoff_add_1() -> io::Result<()> {
        check_magic(
            Rng::default().gen_vec(VN_CUTOFF as usize + 1).unwrap().as_ref(),
            MagicBytes::Vx2,
        )
    }

    // Match short: zero bytes, length 4. Lower limit.
    #[test]
    fn match_short_zero_4() -> io::Result<()> {
        let mut table = HistoryTable::default();
        let bytes = vec![0u8; 4];
        let mut frontend = FrontendBytes::new(&mut table, &bytes);
        let mut dst = Vec::default();
        let mut backend = DummyBackend::default();
        frontend.table.reset();
        frontend.match_blocks(&mut backend, &mut dst).unwrap();
        if frontend.pending.match_len != 0 {
            unsafe { frontend.push_match(&mut backend, &mut dst, frontend.pending)? };
        }
        let literal_len = frontend.src.len() as u32 - frontend.literal_index;
        if literal_len > 0 {
            unsafe { frontend.push_literals(&mut backend, &mut dst, literal_len)? };
        }
        assert_eq!(backend.literals, [0, 0, 0, 0]);
        assert_eq!(backend.lmds, vec![Lmd::<Dummy>::new(4, 0, 1)]);
        Ok(())
    }

    // Match short: zero bytes, length 5++.
    #[test]
    #[ignore = "expensive"]
    fn match_short_zero_n() -> io::Result<()> {
        let mut table = HistoryTable::default();
        let bytes = vec![0u8; 0x1000];
        let mut dst = Vec::default();
        let mut backend = DummyBackend::default();
        for n in 5..bytes.len() {
            backend.init(&mut dst, None)?;
            let mut frontend = FrontendBytes::new(&mut table, &bytes[..n]);
            frontend.table.reset();
            frontend.match_blocks(&mut backend, &mut dst)?;
            if frontend.pending.match_len != 0 {
                unsafe { frontend.push_match(&mut backend, &mut dst, frontend.pending)? };
            }
            assert_eq!(frontend.literal_index, frontend.src.len() as u32);
            assert_eq!(backend.literals, [0]);
            assert_eq!(backend.lmds, vec![Lmd::<Dummy>::new(1, n as u32 - 1, 1)]);
        }
        Ok(())
    }

    // Sandwich, incremental literals.
    #[allow(clippy::needless_range_loop)]
    #[test]
    #[ignore = "expensive"]
    fn sandwich_n_short() -> io::Result<()> {
        let mut table = HistoryTable::default();
        let mut bytes = vec![0u8; 0x1000];
        let mut dst = Vec::default();
        let mut backend = DummyBackend::default();
        for i in 0..4 {
            bytes[i] = i as u8 + 1;
        }
        for n in 12..bytes.len() {
            backend.init(&mut dst, None)?;
            for i in 0..4 {
                bytes[n - 4 + i] = i as u8 + 1;
            }
            let mut frontend = FrontendBytes::new(&mut table, &bytes[..n]);
            frontend.table.reset();
            frontend.match_blocks(&mut backend, &mut dst)?;
            if frontend.pending.match_len != 0 {
                unsafe { frontend.push_match(&mut backend, &mut dst, frontend.pending)? };
            }
            assert_eq!(frontend.literal_index, frontend.src.len() as u32);
            assert_eq!(backend.literals, [1, 2, 3, 4, 0]);
            assert_eq!(
                backend.lmds,
                vec![Lmd::<Dummy>::new(5, n as u32 - 9, 1), Lmd::<Dummy>::new(0, 4, n as u32 - 4),]
            );
            bytes[n - 4] = 0;
        }
        Ok(())
    }
}
