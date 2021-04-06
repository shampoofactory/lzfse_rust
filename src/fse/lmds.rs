use crate::bits::{BitDst, BitReader, BitSrc, BitWriter};
use crate::lmd::LmdPack;
use crate::ops::WriteShort;

use super::block::LmdParam;
use super::constants::*;
use super::decoder::{self, Decoder};
use super::encoder::{self, Encoder};
use super::error::Error;
use super::object::Fse;

use std::io;

const BUF_LEN: usize = LMDS_PER_BLOCK as usize;

#[repr(C)]
pub struct Lmds(Box<[LmdPack<Fse>]>, usize);

impl Lmds {
    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, lmd: LmdPack<Fse>) {
        debug_assert!(self.1 < LMDS_PER_BLOCK as usize);
        *self.0.get_unchecked_mut(self.1) = lmd;
        self.1 += 1;
    }

    pub fn load<T>(&mut self, src: T, decoder: &Decoder, param: &LmdParam) -> crate::Result<()>
    where
        T: BitSrc,
    {
        let mut reader = BitReader::new(src, param.bits() as usize)?;
        let state = param.state();
        let mut state = (
            unsafe { decoder::L::new_unchecked(state[0] as usize) },
            unsafe { decoder::M::new_unchecked(state[1] as usize) },
            unsafe { decoder::D::new_unchecked(state[2] as usize) },
        );
        let n_lmds = param.num() as usize;
        debug_assert!(n_lmds <= LMDS_PER_BLOCK as usize);
        for lmd in unsafe { self.0.get_unchecked_mut(..n_lmds) } {
            // `flush` constraints:
            // 32 bit systems: flush after each L, M, D component pull.
            // 64 bit systems: flush after all L, M, D components have been pulled.
            let literal_len = unsafe { decoder.l(&mut reader, &mut state.0) };
            #[cfg(target_pointer_width = "32")]
            reader.flush();
            let match_len = unsafe { decoder.m(&mut reader, &mut state.1) };
            #[cfg(target_pointer_width = "32")]
            reader.flush();
            let match_distance_zeroed = unsafe { decoder.d(&mut reader, &mut state.2) };
            reader.flush();
            *lmd = LmdPack(literal_len.into(), match_len.into(), match_distance_zeroed);
        }
        reader.finalize()?;
        if state != (decoder::L::default(), decoder::M::default(), decoder::D::default()) {
            return Err(Error::BadLmdPayload.into());
        }
        self.1 = n_lmds;
        Ok(())
    }

    pub fn store<T>(&self, dst: &mut T, encoder: &Encoder) -> io::Result<LmdParam>
    where
        T: BitDst + WriteShort,
    {
        debug_assert!(self.1 <= LMDS_PER_BLOCK as usize);
        let mark = dst.pos();
        // 8 byte pad.
        dst.write_short_u64(0)?;
        let n_bytes = (self.1 * MAX_LMD_BITS as usize + 7) / 8;
        let mut writer = BitWriter::new(dst, n_bytes)?;
        let mut state = (encoder::L::default(), encoder::M::default(), encoder::D::default());
        for &LmdPack(literal_len, match_len, match_distance_zeroed) in
            unsafe { self.0.get_unchecked(..self.1).iter().rev() }
        {
            // `flush` constraints:
            // 32 bit systems: flush after each L, M, D component pull.
            // 64 bit systems: flush after all L, M, D components have been pulled.
            unsafe { encoder.d(&mut writer, &mut state.2, match_distance_zeroed) };
            #[cfg(target_pointer_width = "32")]
            writer.flush();
            unsafe { encoder.m(&mut writer, &mut state.1, match_len.into()) };
            #[cfg(target_pointer_width = "32")]
            writer.flush();
            unsafe { encoder.l(&mut writer, &mut state.0, literal_len.into()) };
            writer.flush();
        }
        let state =
            [u32::from(state.0) as u16, u32::from(state.1) as u16, u32::from(state.2) as u16];
        let bits = writer.finalize()? as u32;
        let n_payload_bytes = (dst.pos() - mark) as u32;
        Ok(LmdParam::new(self.1 as u32, n_payload_bytes, bits, state).expect("internal error"))
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        debug_assert!(self.1 <= LMDS_PER_BLOCK as usize);
        self.1
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        debug_assert!(self.1 <= LMDS_PER_BLOCK as usize);
        self.1 = 0;
    }
}

impl AsRef<[LmdPack<Fse>]> for Lmds {
    #[inline(always)]
    fn as_ref(&self) -> &[LmdPack<Fse>] {
        debug_assert!(self.1 <= LMDS_PER_BLOCK as usize);
        &self.0[..self.1]
    }
}

impl Default for Lmds {
    fn default() -> Self {
        Self(vec![LmdPack::default(); BUF_LEN].into_boxed_slice(), 0)
    }
}
