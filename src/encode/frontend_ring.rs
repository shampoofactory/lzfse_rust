use crate::base::MagicBytes;
use crate::fse::{Fse, FseBackend};
use crate::kit::ReadExtFully;
use crate::lmd::DMax;
use crate::lmd::MatchDistance;
use crate::raw::{self, RAW_HEADER_SIZE};
use crate::ring::{self, Ring, RingBlock, RingType};
use crate::types::{Idx, ShortWriter};
use crate::vn::{BackendVn, Vn};

use super::backend::Backend;
use super::backend_type::BackendType;
use super::constants::*;
use super::history::{History, HistoryTable, UIdx};
use super::match_object::Match;
use super::match_unit::MatchUnit;

use std::io::{self, Read};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum Commit {
    Fse,
    Vn,
    None,
}

trait Flush {
    const FLUSH: bool;
}

#[derive(Copy, Clone)]
struct FlushTrue;

impl Flush for FlushTrue {
    const FLUSH: bool = true;
}

#[derive(Copy, Clone)]
struct FlushFalse;

impl Flush for FlushFalse {
    const FLUSH: bool = false;
}

pub struct FrontendRing<'a, T> {
    table: &'a mut HistoryTable,
    ring: Ring<'a, T>,
    pending: Match,
    literal_idx: Idx,
    idx: Idx,
    head: Idx,
    mark: Idx,
    tail: Idx,
    clamp: Idx,
    commit: Commit,
}

// Implementation notes:
//
// Built over `Ring`. It may be easier to visualize if we imagine a sliding window over flat input
// data as opposed to being overly bogged down in `Ring` internal mechanics.
//
// <------------------------------------ INPUT DATA--------------------------------------->
//              |--X--|---------------------- W ----------------------|--X--|
//                                           | ----- G ----- |
//                    ^H          ^L              ^I                  ^T    ^U
//                                               |---- M ----|
//                          |---- R ----|
//                                            |--- P ---|
//                       |--- Q ---|
//              <--------------------------- RING -------------------------->
//
//
// -------------------------------------------------------------------------------------------
// Tag | Description | Notes
// -------------------------------------------------------------------------------------------
// W   | Window      | Window/ working buffer.
// H   | Window head |
// T   | Window tail |
// U   | Mark        | Fill mark.
// X   | Undefined   | Undefined buffer data.
// L   | Literal idx | Data below this point has been pushed into the backend.
// I   | Working idx | Data below this point has been pushed into history but not the backend.
// G   | Goldilocks  | G = RING_SIZE / 2..RING_SIZE / 2 + RING_BLOCK_SIZE
// M   | Match       | Incoming match target.
// R   | Match       | Incoming match source.
// P   | Match       | Pending match target.
// Q   | Match       | Pending match source.
//
//
// Global constraints:
// H <= L <= I <= T <= U <= H + RING_SIZE
// H <= R < M <= T      R can overlap M
// H <= Q < P <= T      Q can overlap P
// P < M                P can overlap M
//
// Operational constraints:
// I == L == H == T     If and only if: init.
// I == L == H          If and only if: no blocks processed.
//
// Match constraints:
// source.idx <  target.idx
// source.len == target.len
//
// 'Goldilocks` zone is the optimal working index position with respect to matching. Within the zone
// we have RING_SIZE / 2 - RING_BLOCK_SIZE incremental matching and RING_SIZE /2 decremental
// matching.
//
// `commit` defines the type of history values we have stored in our `table`. More specifically the
// minimum match length and index hash method. Fse and Vn history types are incompatible in this
// regard. The initial `commit` value is `None` and the compression logic may commit to `Fse` or
// `Vn` but not both.
//
// `clamp` defines the `Idx` value, with 1GB leeway, at which distances should be clamped in our
// `table`. Failure to do this will result in old values eventually, that is after 3GB or so,
// wrapping back around resulting in data corruption. We shall keep well away from these limits.

impl<'a, T: Copy + RingBlock> FrontendRing<'a, T> {
    // Non flush max match len that doesn't overshoot our tail.
    const MAX_MATCH_LEN: u32 = T::RING_SIZE / 2 - T::RING_BLK_SIZE - ring::overmatch_len(4) as u32;

    pub fn new(ring: Ring<'a, T>, table: &'a mut HistoryTable) -> Self {
        assert!(T::RING_BLK_SIZE.is_power_of_two());
        assert!(Vn::MAX_MATCH_DISTANCE < T::RING_SIZE / 2);
        assert!(Fse::MAX_MATCH_DISTANCE < T::RING_SIZE / 2);
        assert!(T::RING_SIZE <= CLAMP_INTERVAL / 4);
        assert!(0x100 < T::RING_BLK_SIZE as usize);
        assert!(T::RING_BLK_SIZE <= T::RING_SIZE / 4);
        assert!(ring::overmatch_len(4) < T::RING_LIMIT as usize);
        let zero = Idx::default();
        Self {
            table,
            ring,
            pending: Match::default(),
            literal_idx: zero,
            idx: zero,
            head: zero,
            mark: zero,
            tail: zero,
            clamp: zero,
            commit: Commit::None,
        }
    }

    /// Call immediately after init.
    #[inline(always)]
    pub fn copy<B, I, O>(&mut self, backend: &mut B, dst: &mut O, src: &mut I) -> io::Result<u64>
    where
        B: Backend,
        I: Read,
        O: ShortWriter,
    {
        debug_assert_eq!(self.pending.match_len, 0);
        debug_assert_eq!(self.literal_idx, Idx::default());
        debug_assert_eq!(self.idx, Idx::default());
        debug_assert_eq!(self.head, Idx::default());
        debug_assert_eq!(self.mark, Idx::default() + T::RING_BLK_SIZE);
        debug_assert_eq!(self.tail, Idx::default());
        debug_assert_eq!(self.clamp, Idx::default() + CLAMP_INTERVAL);
        debug_assert_eq!(self.commit, Commit::None);
        let mut n_raw_bytes = 0;
        loop {
            debug_assert!(self.head <= self.tail);
            debug_assert!(self.tail < self.mark);
            debug_assert!(self.mark <= self.head + T::RING_SIZE);
            debug_assert_eq!(self.tail % T::RING_BLK_SIZE, 0);
            debug_assert_eq!(self.mark % T::RING_BLK_SIZE, 0);
            // Load one block.
            let index = self.tail % T::RING_SIZE as usize;
            let bytes =
                unsafe { self.ring.get_unchecked_mut(index..index + T::RING_BLK_SIZE as usize) };
            let len = src.read_fully(bytes)?;
            self.tail += len as u32;
            n_raw_bytes += len as u64;
            // EOF, break.
            if len != T::RING_BLK_SIZE as usize {
                break;
            }
            // Manage head/ tail zones.
            debug_assert_eq!(self.tail, self.mark);
            if self.mark % T::RING_SIZE == T::RING_BLK_SIZE {
                self.ring.head_copy_out();
            } else if self.mark % T::RING_SIZE == 0 {
                self.ring.tail_copy_out();
            }
            // Not full, continue.
            if self.mark != self.head + T::RING_SIZE {
                self.mark += T::RING_BLK_SIZE;
                continue;
            }
            // Full call block, clamp and continue.
            self.block(backend, dst)?;
            self.mark = self.tail + T::RING_BLK_SIZE;
            self.clamp();
        }
        Ok(n_raw_bytes)
    }

    /// Call after init.    
    pub fn write<O>(
        &mut self,
        backend: &mut FseBackend,
        mut src: &[u8],
        dst: &mut O,
    ) -> io::Result<usize>
    where
        O: ShortWriter,
    {
        let total_len = src.len();
        loop {
            debug_assert!(self.head <= self.tail);
            debug_assert!(self.tail < self.mark);
            debug_assert!(self.mark <= self.head + T::RING_SIZE);
            debug_assert_eq!(self.mark % T::RING_BLK_SIZE, 0);
            // Load one block.
            let len = src.len();
            let index = self.tail % T::RING_SIZE as usize;
            let limit = (self.mark - self.tail) as usize;
            if len < limit {
                unsafe { self.ring.get_unchecked_mut(index..index + len) }.copy_from_slice(src);
                self.tail += len as u32;
                break;
            }
            unsafe {
                self.ring
                    .get_unchecked_mut(index..index + limit)
                    .copy_from_slice(src.get_unchecked(..limit))
            };
            src = unsafe { src.get_unchecked(limit..) };
            self.tail += limit as u32;
            // Manage head/ tail zones.
            debug_assert_eq!(self.tail, self.mark);
            if self.mark % T::RING_SIZE == T::RING_BLK_SIZE {
                self.ring.head_copy_out();
            } else if self.mark % T::RING_SIZE == 0 {
                self.ring.tail_copy_out();
            }
            // Not full, continue.
            if self.mark != self.head + T::RING_SIZE {
                self.mark += T::RING_BLK_SIZE;
                continue;
            }
            self.block(backend, dst)?;
            self.mark = self.tail + T::RING_BLK_SIZE;
            self.clamp();
        }
        Ok(total_len)
    }

    #[inline(always)]
    fn block<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        debug_assert!(self.commit == Commit::None || self.commit == Commit::Fse);
        // Commit.
        if self.commit == Commit::None {
            self.commit = Commit::Fse;
            backend.init(dst, None)?;
        }
        // Process.
        self.process(backend, dst)?;
        // Manage head.
        let delta = (self.idx - self.head) as u32;
        let delta = (delta - T::RING_SIZE / 2) / T::RING_BLK_SIZE * T::RING_BLK_SIZE;
        self.head += delta;
        // Manage literal overflow.
        if self.literal_idx < self.head {
            // We have literals that have have passed our buffer head without a match, we'll push
            // them as is.
            // We could push pending, but we don't, we discard it. The compression loss is
            // negligible, at most we lose `GOOD_MATCH - 1` bytes in a situation with a low
            // probability of occurrence. Instead we take the reduction in code complexity/ size.
            self.pending.match_len = 0;
            self.push_literals(backend, dst, (self.head - self.literal_idx) as u32)?;
        }
        debug_assert!(self.head <= self.literal_idx);
        Ok(())
    }

    /// Call after init.
    pub fn flush<O>(&mut self, backend: &mut FseBackend, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        debug_assert!(self.head <= self.literal_idx);
        debug_assert!(self.literal_idx <= self.idx);
        debug_assert!(self.idx <= self.tail);
        debug_assert!(self.tail < self.mark);
        debug_assert!(self.mark <= self.head + T::RING_SIZE);
        // Manage head/ tail zones.
        if self.mark % T::RING_SIZE == T::RING_BLK_SIZE {
            self.ring.head_copy_out();
        } else if self.mark % T::RING_SIZE == 0 {
            self.ring.tail_copy_out();
        }
        // Select.
        match self.commit {
            Commit::Fse => {
                self.finalize(backend, dst)?;
            }
            Commit::Vn => {
                panic!("internal error: invalid commit state: {:?}", self.commit);
            }
            Commit::None => self.flush_select(backend, dst)?,
        };
        debug_assert!(-(CLAMP_INTERVAL as i32) <= self.idx - self.clamp);
        debug_assert!(self.idx - self.clamp < CLAMP_INTERVAL as i32 / 2);
        debug_assert_eq!(self.literal_idx, self.tail);
        // Eos.
        dst.write_short_u32(MagicBytes::Eos.into())?;
        Ok(())
    }

    fn flush_select<O>(&mut self, backend: &mut FseBackend, dst: &mut O) -> io::Result<()>
    where
        O: ShortWriter,
    {
        debug_assert!(self.head.is_zero());
        debug_assert!(self.literal_idx.is_zero());
        debug_assert!(self.idx.is_zero());
        debug_assert_eq!(self.pending.match_len, 0);
        debug_assert_eq!(self.commit, Commit::None);
        let len = (self.tail - self.idx) as u32;
        if len > VN_CUTOFF {
            self.commit = Commit::Fse;
            backend.init(dst, Some(len))?;
            self.finalize(backend, dst)?;
            return Ok(());
        }
        if len > RAW_CUTOFF {
            self.commit = Commit::Vn;
            let mut backend = BackendVn::default();
            backend.init(dst, Some(len))?;
            let mark = dst.pos();
            self.finalize(&mut backend, dst)?;
            let dst_len = (dst.pos() - mark) as u32;
            let src_len = (self.tail - Idx::default()) as u32;
            if dst_len < src_len + RAW_HEADER_SIZE {
                return Ok(());
            }
            dst.truncate(mark);
        }
        let bytes = self.ring.view(self.head, self.tail);
        raw::raw_compress(dst, bytes)?;
        self.literal_idx = self.tail;
        Ok(())
    }

    fn finalize<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        debug_assert!(self.tail < self.head + T::RING_SIZE);
        let len = (self.tail - self.idx) as u32;
        if len >= 4 {
            self.process_final(backend, dst)?;
        }
        if self.pending.match_len != 0 {
            unsafe { self.push_match(backend, dst, self.pending)? };
            self.pending.match_len = 0;
        }
        let len = (self.tail - self.literal_idx) as u32;
        if len != 0 {
            self.push_literals(backend, dst, len)?;
        }
        backend.finalize(dst)?;
        Ok(())
    }

    #[inline(always)]
    fn process<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        // This method assumes that we have a full buffer with correctly managed head and tail
        // zones, it will not operate correctly otherwise.
        // `idx` needs to be either in init position where `idx == head`, or within the 'Goldilocks'
        // zone.
        // We continue matching until we exit the right side of the 'Goldilocks' zone.
        debug_assert_eq!(self.head + T::RING_SIZE, self.tail);
        debug_assert!(self.head <= self.literal_idx);
        debug_assert!(self.literal_idx <= self.idx);
        debug_assert!(self.head == self.idx || self.head + T::RING_SIZE / 2 <= self.idx);
        debug_assert!(self.idx < self.head + T::RING_SIZE / 2 + T::RING_BLK_SIZE);
        let mut idx = self.idx;
        self.idx = self.head + T::RING_SIZE / 2 + T::RING_BLK_SIZE;
        loop {
            // Hot loop.
            let u = self.ring.get_u32(idx);
            let u_idx = UIdx::new(u, idx);
            let queue = self.table.push::<B::Type>(u_idx);
            let incoming = self.find_match::<B::Type, false>(queue, u_idx, Self::MAX_MATCH_LEN);
            if let Some(select) = self.pending.select::<GOOD_MATCH_LEN>(incoming) {
                unsafe { self.push_match(backend, dst, select)? };
                idx += 1;
                for _ in 0..(self.literal_idx - idx) {
                    let u = self.ring.get_u32(idx);
                    let u_idx = UIdx::new(u, idx);
                    self.table.push::<B::Type>(u_idx);
                    idx += 1;
                }
                if idx >= self.idx {
                    // Unlikely
                    self.idx = idx;
                    break;
                }
            } else {
                idx += 1;
                if idx == self.idx {
                    // Unlikely
                    break;
                }
            }
        }
        debug_assert!(self.head <= self.literal_idx);
        debug_assert!(self.literal_idx <= self.idx);
        debug_assert!(self.head + T::RING_SIZE / 2 + T::RING_BLK_SIZE <= self.idx);
        debug_assert!(self.idx <= self.tail);
        Ok(())
    }

    #[inline(always)]
    fn process_final<B, O>(&mut self, backend: &mut B, dst: &mut O) -> io::Result<()>
    where
        B: Backend,
        O: ShortWriter,
    {
        // This method assumes that we have a non-full buffer with correctly managed head and tail
        // zones, it will not operate correctly otherwise.
        // We continue matching until we hit the MATCH_UNIT limit.
        debug_assert!(4 <= (self.tail - self.idx) as u32);
        debug_assert!(self.head <= self.literal_idx);
        debug_assert!(self.literal_idx <= self.idx);
        debug_assert!(self.idx < self.tail);
        let mut idx = self.idx;
        self.idx = self.tail - B::Type::MATCH_UNIT + 1;
        loop {
            // Hot loop.
            let u = self.ring.get_u32(idx);
            let u_idx = UIdx::new(u, idx);
            let queue = self.table.push::<B::Type>(u_idx);
            let max = (self.tail - idx) as u32;
            let incoming = self.find_match::<B::Type, true>(queue, u_idx, max);
            if let Some(select) = self.pending.select::<GOOD_MATCH_LEN>(incoming) {
                unsafe { self.push_match(backend, dst, select)? };
                if self.literal_idx >= self.idx {
                    // Unlikely.
                    break;
                }
                idx += 1;
                for _ in 0..(self.literal_idx - idx) {
                    let u = self.ring.get_u32(idx);
                    let u_idx = UIdx::new(u, idx);
                    self.table.push::<B::Type>(u_idx);
                    idx += 1;
                }
                if idx >= self.idx {
                    // Unlikely
                    self.idx = idx;
                    break;
                }
            } else {
                idx += 1;
                if idx == self.idx {
                    // Unlikely
                    break;
                }
            }
        }
        debug_assert!(self.head <= self.literal_idx);
        debug_assert!(self.literal_idx <= self.tail);
        debug_assert!(self.idx <= self.tail);
        Ok(())
    }

    #[inline(always)]
    fn find_match<B, const F: bool>(&self, queue: History, item: UIdx, max: u32) -> Match
    where
        B: BackendType,
    {
        debug_assert!(max >= B::MATCH_UNIT);
        let mut m = Match::default();
        for &match_idx_val in queue.iter() {
            let distance = (item.idx - match_idx_val.idx) as u32;
            debug_assert!(distance < CLAMP_INTERVAL * 3);
            if distance > B::MAX_MATCH_DISTANCE {
                break;
            }
            let match_len_inc = self.match_unit_coarse::<B>(item, match_idx_val, max);
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
            let literal_len = (item.idx - self.literal_idx) as u32;
            m.idx = item.idx;
            if F {
                m.match_len = m.match_len.min(max);
            }
            let max = ((m.match_idx - self.head) as u32).min(literal_len);
            let match_len_dec = self.match_dec_coarse(m.idx, m.match_idx, max).min(max);
            m.idx -= match_len_dec;
            m.match_idx -= match_len_dec;
            m.match_len += match_len_dec;
            debug_assert_ne!(m.match_len, 0);
            debug_assert!(m.match_len >= B::MATCH_UNIT);
            debug_assert!(self.literal_idx <= m.idx);
            debug_assert!(m.match_idx < m.idx);
            debug_assert!((m.idx - m.match_idx) as u32 <= B::MAX_MATCH_DISTANCE);
            debug_assert!(m.idx + m.match_len <= self.tail);
            m
        }
    }

    #[inline(always)]
    fn match_unit_coarse<M: MatchUnit>(&self, item: UIdx, match_item: UIdx, max: u32) -> u32 {
        debug_assert!((item.u ^ self.ring.get_u32(item.idx)) & M::MATCH_MASK == 0);
        debug_assert!((match_item.u ^ self.ring.get_u32(match_item.idx)) & M::MATCH_MASK == 0);
        debug_assert!(self.head <= match_item.idx);
        debug_assert!(match_item.idx < item.idx);
        debug_assert!(item.idx < self.tail);
        let len = M::match_us((item.u, match_item.u));
        if len == 4 {
            self.ring.coarse_match_inc((item.idx, match_item.idx), 4, max as usize) as u32
        } else {
            len
        }
    }

    #[inline(always)]
    fn match_dec_coarse(&self, idx: Idx, match_idx: Idx, literal_len: u32) -> u32 {
        debug_assert!(self.head <= match_idx);
        debug_assert!(match_idx < idx);
        debug_assert!(idx < self.tail);
        self.ring.match_dec_coarse((idx, match_idx), 0, literal_len as usize) as u32
    }

    #[inline(always)]
    unsafe fn push_match<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
        m: Match,
    ) -> io::Result<()> {
        debug_assert_ne!(m.match_len, 0);
        debug_assert!(self.literal_idx <= m.idx);
        debug_assert!(m.match_idx < m.idx);
        debug_assert!((m.idx - m.match_idx) as u32 <= B::Type::MAX_MATCH_DISTANCE);
        debug_assert!(m.idx + m.match_len <= self.tail);
        let match_len = m.match_len;
        let match_distance = MatchDistance::new_unchecked((m.idx - m.match_idx) as u32);
        let literals = self.ring.view(self.literal_idx, m.idx);
        self.literal_idx = m.idx + m.match_len;
        backend.push_match(dst, literals, match_len, match_distance)
    }

    fn push_literals<B: Backend, O: ShortWriter>(
        &mut self,
        backend: &mut B,
        dst: &mut O,
        len: u32,
    ) -> io::Result<()> {
        debug_assert_ne!(len, 0);
        debug_assert_eq!(self.pending.match_len, 0);
        debug_assert!(self.literal_idx + len <= self.tail);
        let literals = self.ring.view(self.literal_idx, self.literal_idx + len);
        self.literal_idx += len;
        backend.push_literals(dst, literals)
    }

    pub fn init(&mut self) {
        let zero = Idx::default();
        self.table.reset_idx(zero - CLAMP_INTERVAL);
        self.pending = Match::default();
        self.literal_idx = zero;
        self.idx = zero;
        self.head = zero;
        self.mark = zero + T::RING_BLK_SIZE;
        self.tail = zero;
        self.clamp = zero + CLAMP_INTERVAL;
        self.commit = Commit::None;
    }

    fn clamp(&mut self) {
        let delta = self.idx - self.clamp;
        debug_assert!(delta < CLAMP_INTERVAL as i32 && delta >= -(CLAMP_INTERVAL as i32));
        if delta >= 0 {
            // Unlikely.
            assert!(delta < CLAMP_INTERVAL as i32);
            self.table.clamp(self.idx);
            self.clamp += CLAMP_INTERVAL;
        }
    }
}

impl<'a, T: RingType> AsMut<[u8]> for FrontendRing<'a, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.ring
    }
}

#[cfg(test)]
mod tests {
    use crate::ops::WriteShort;
    use crate::ring::RingBox;
    use crate::vn::BackendVn;

    use super::*;

    // Check flush terminal match is detected and correct against a manually constructed reference.
    #[test]
    #[ignore = "expensive"]
    fn sandwich() -> io::Result<()> {
        let mut ring_box = RingBox::<Input>::default();
        let mut table = HistoryTable::default();

        let mut fse = FseBackend::default();
        let mut vn = BackendVn::default();

        let mut frontend_enc = Vec::default();
        let mut frontend_dec = Vec::default();
        let mut backend_enc = Vec::default();
        let mut backend_dec = Vec::default();
        let mut master = Vec::default();

        for n in RAW_CUTOFF - 7..VN_CUTOFF - 7 {
            // Manually construct our encoded reference.
            vn.init(&mut backend_enc, Some(0x2000))?;
            if n < GOOD_MATCH_LEN {
                vn.push_match(&mut backend_enc, [0].as_ref(), 3, MatchDistance::new(1))?;
                vn.push_match(&mut backend_enc, [1].as_ref(), n, MatchDistance::new(1))?;
            } else {
                vn.push_match(
                    &mut backend_enc,
                    [0, 0, 0, 0, 1].as_ref(),
                    n,
                    MatchDistance::new(1),
                )?;
            }
            vn.push_match(&mut backend_enc, [].as_ref(), 3, MatchDistance::new(5 + n))?;
            vn.finalize(&mut backend_enc)?;
            backend_enc.write_short_u32(MagicBytes::Eos.into())?;

            // Manually construct our decoded reference and compress.
            master.resize(master.len() + 4, 0);
            master.resize(master.len() + n as usize + 1, 1);
            master.resize(master.len() + 3, 0);

            let ring = (&mut ring_box).into();
            let mut frontend = FrontendRing::new(ring, &mut table);
            frontend.ring.fill(0);
            frontend.init();
            frontend.copy(&mut fse, &mut frontend_enc, &mut master.as_slice())?;
            frontend.flush(&mut fse, &mut frontend_enc)?;
            crate::decode_bytes(&frontend_enc, &mut frontend_dec)?;
            crate::decode_bytes(&backend_enc, &mut backend_dec)?;

            // Validate.
            assert!(backend_dec == master);
            assert!(frontend_dec == master);
            assert!(frontend_enc == backend_enc);

            // Reset.
            frontend_enc.clear();
            frontend_dec.clear();
            backend_enc.clear();
            backend_dec.clear();
            master.clear();
        }
        Ok(())
    }
}
