use crate::bits::{BitDst, BitReader, BitSrc, BitWriter};
use crate::kit::{CopyTypeIndex, WIDE};
use crate::lmd::LMax;
use crate::types::ShortBuffer;

use super::block::LiteralParam;
use super::constants::*;
use super::decoder::{self, Decoder};
use super::encoder::{self, Encoder};
use super::error::Error;
use super::Fse;

use std::io;
use std::usize;

const BUF_LEN: usize = LITERALS_PER_BLOCK as usize + MAX_L_VALUE as usize + WIDE;

#[repr(C)]
pub struct Literals(Box<[u8]>, pub usize);

impl Literals {
    #[inline(always)]
    pub unsafe fn push_unchecked_max<I>(&mut self, literals: &mut I)
    where
        I: ShortBuffer,
    {
        assert!(Fse::MAX_LITERAL_LEN as u32 <= I::SHORT_LIMIT);
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        debug_assert!(self.1 + Fse::MAX_LITERAL_LEN as usize <= LITERALS_PER_BLOCK as usize);
        let ptr = self.0.as_mut_ptr().add(self.1);
        literals.read_short_raw::<CopyTypeIndex>(ptr, Fse::MAX_LITERAL_LEN as usize);
        self.1 += Fse::MAX_LITERAL_LEN as usize;
    }

    #[inline(always)]
    pub unsafe fn push_unchecked<I>(&mut self, literals: &mut I, n_literals: u32)
    where
        I: ShortBuffer,
    {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        debug_assert!(self.1 + n_literals as usize <= LITERALS_PER_BLOCK as usize);
        debug_assert!(n_literals <= I::SHORT_LIMIT);
        let ptr = self.0.as_mut_ptr().add(self.1);
        literals.read_short_raw::<CopyTypeIndex>(ptr, n_literals as usize);
        self.1 += n_literals as usize;
    }

    #[allow(clippy::clippy::identity_op)]
    pub fn load<T>(&mut self, src: T, decoder: &Decoder, param: &LiteralParam) -> crate::Result<()>
    where
        T: BitSrc,
    {
        let mut reader = BitReader::new(src, param.bits() as usize)?;
        let state = param.state();
        let mut state = (
            unsafe { decoder::U::new_unchecked(state[0] as usize) },
            unsafe { decoder::U::new_unchecked(state[1] as usize) },
            unsafe { decoder::U::new_unchecked(state[2] as usize) },
            unsafe { decoder::U::new_unchecked(state[3] as usize) },
        );
        let ptr = self.0.as_mut_ptr().cast::<u8>();
        let n_literals = param.num() as usize;
        debug_assert!(n_literals <= LITERALS_PER_BLOCK as usize);
        let mut i = 0;
        while i != n_literals {
            // `flush` constraints:
            // 32 bit systems: maximum of x2 10 bit pushes.
            // 64 bit systems: maximum of x5 10 bit pushes (although we only push 4 for simplicity).
            unsafe { *ptr.add(i + 0) = decoder.u(&mut reader, &mut state.0) };
            unsafe { *ptr.add(i + 1) = decoder.u(&mut reader, &mut state.1) };
            #[cfg(target_pointer_width = "32")]
            unsafe {
                reader.flush()
            };
            unsafe { *ptr.add(i + 2) = decoder.u(&mut reader, &mut state.2) };
            unsafe { *ptr.add(i + 3) = decoder.u(&mut reader, &mut state.3) };
            reader.flush();
            i += 4;
        }
        reader.finalize()?;
        if state
            != (
                decoder::U::default(),
                decoder::U::default(),
                decoder::U::default(),
                decoder::U::default(),
            )
        {
            return Err(Error::BadLmdPayload.into());
        }
        self.1 = n_literals;
        Ok(())
    }

    pub fn store<T>(&self, dst: &mut T, encoder: &Encoder) -> io::Result<LiteralParam>
    where
        T: BitDst,
    {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        let mark = dst.pos();
        let n_literals = (self.1 + 3) / 4 * 4;
        let n_bytes = (n_literals * MAX_U_BITS as usize + 7) / 8;
        let mut writer = BitWriter::new(dst, n_bytes)?;
        let mut state = (
            encoder::U::default(),
            encoder::U::default(),
            encoder::U::default(),
            encoder::U::default(),
        );
        let ptr = self.0.as_ptr();
        let mut i = n_literals;
        while i != 0 {
            // `flush` constraints:
            // 32 bit systems: maximum of x2 10 bit pushes.
            // 64 bit systems: maximum of x5 10 bit pushes (although we only push 4 for simplicity).
            unsafe { encoder.u(&mut writer, &mut state.3, *ptr.add(i - 1)) };
            unsafe { encoder.u(&mut writer, &mut state.2, *ptr.add(i - 2)) };
            #[cfg(target_pointer_width = "32")]
            writer.flush();
            unsafe { encoder.u(&mut writer, &mut state.1, *ptr.add(i - 3)) };
            unsafe { encoder.u(&mut writer, &mut state.0, *ptr.add(i - 4)) };
            writer.flush();
            i -= 4;
        }
        let state = [
            u32::from(state.0) as u16,
            u32::from(state.1) as u16,
            u32::from(state.2) as u16,
            u32::from(state.3) as u16,
        ];
        let bits = writer.finalize()? as u32;
        let n_payload_bytes = (dst.pos() - mark) as u32;
        let n_literals = (self.1 as u32 + 3) / 4 * 4;
        Ok(LiteralParam::new(n_literals, n_payload_bytes, bits, state).expect("internal error"))
    }

    #[inline(always)]
    pub fn pad(&mut self) {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        self.pad_u(unsafe { *self.0.get_unchecked(0) });
    }

    #[inline(always)]
    pub fn pad_u(&mut self, u: u8) {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        unsafe { self.0.get_unchecked_mut(self.1..).get_unchecked_mut(..4) }.fill(u);
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        self.1
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        self.1 = 0;
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
}

impl AsRef<[u8]> for Literals {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        debug_assert!(self.1 <= LITERALS_PER_BLOCK as usize);
        unsafe { self.0.get_unchecked(..self.1) }
    }
}

impl Default for Literals {
    fn default() -> Self {
        Self(vec![0u8; BUF_LEN].into_boxed_slice(), 0)
    }
}
