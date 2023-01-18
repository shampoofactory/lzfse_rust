use crate::Error;

use super::bit_mask;
use super::bit_src::BitSrc;

use std::mem;

pub const ACCUM_MAX: isize = mem::size_of::<usize>() as isize * 8;

#[cfg(target_pointer_width = "32")]
const MASK: [usize; 5] = [0x0000_0000, 0x0000_00FF, 0x0000_FFFF, 0x00FF_FFFF, 0xFFFF_FFFF];

pub struct BitReader<T: BitSrc> {
    accum_data: usize,
    accum_bits: isize,
    index: isize,
    inner: T,
}

impl<T: BitSrc> BitReader<T> {
    #[inline(always)]
    pub fn new(inner: T, off: usize) -> crate::Result<Self> {
        assert!(off <= 7);
        assert!(8 <= inner.len());
        assert!(inner.len() <= isize::MAX as usize);
        let index = inner.len() as isize - mem::size_of::<usize>() as isize;
        let accum_data = unsafe { inner.read_bytes(index) };
        let accum_bits = mem::size_of::<usize>() as isize * 8 - off as isize;
        if off != 0 && accum_data >> accum_bits != 0 {
            Err(Error::BadBitStream)
        } else {
            Ok(Self { accum_data, accum_bits, inner, index })
        }
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.inner
    }

    #[inline(always)]
    pub fn flush(&mut self) {
        debug_assert!(0 <= self.accum_bits);
        debug_assert!(self.accum_bits <= ACCUM_MAX);
        let n_bytes = (ACCUM_MAX - self.accum_bits) as usize / 8;
        let n_bits = n_bytes * 8;
        debug_assert!(n_bytes < mem::size_of::<usize>());
        self.index -= n_bytes as isize;
        self.accum_data = unsafe { self.inner.read_bytes(self.index) };
        self.accum_bits += n_bits as isize;
        debug_assert!(0 <= self.accum_bits);
        debug_assert!(self.accum_bits <= ACCUM_MAX);
    }

    /// # Safety
    ///
    /// * No more than `ACCUM_MAX` bits in total are pulled without flushing.
    #[inline(always)]
    pub unsafe fn pull(&mut self, n_bits: usize) -> usize {
        debug_assert!(n_bits <= 32);
        self.accum_bits -= n_bits as isize;
        // TODO consider `unchecked_shr` when stable.
        let accum_shift = self.accum_data >> (self.accum_bits & (ACCUM_MAX - 1));
        bit_mask::mask(accum_shift, n_bits)
    }

    #[inline(always)]
    pub fn finalize(mut self) -> crate::Result<()> {
        self.flush();
        if self.accum_bits + self.index * 8 < 64 {
            return Err(Error::PayloadUnderflow);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use test_kit::Fibonacci;

    use super::*;

    // Bit stream of the first 32 Fibonacci numbers.
    const FIB_32_BS: [u8; 49] = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7B, 0xB1, 0xAB, 0x78, 0x67, 0x21, 0xD3,
        0xF3, 0x8A, 0xB9, 0x7D, 0x8F, 0x31, 0xB4, 0x0A, 0xB6, 0x69, 0x61, 0xF5, 0xA5, 0x18, 0xFF,
        0x06, 0xA9, 0x8D, 0x28, 0x19, 0xA3, 0x5D, 0xE8, 0xDF, 0xB9, 0x6C, 0xD6, 0x62, 0x1F, 0x45,
        0x96, 0xBB, 0x15, 0x29,
    ];

    const FIB_32_OFF: usize = 2;

    #[test]
    fn fibonacci() -> crate::Result<()> {
        let src = FIB_32_BS.as_ref();
        let mut rdr = BitReader::new(src, FIB_32_OFF)?;
        let fib: Vec<u32> = Fibonacci::default().take(32).collect();
        for &v in fib.iter().rev() {
            rdr.flush();
            let u = unsafe { rdr.pull(32 - v.leading_zeros() as usize) as u32 };
            assert_eq!(v, u);
        }
        assert_eq!(rdr.index * 8 + rdr.accum_bits, 64);
        rdr.finalize()?;
        Ok(())
    }

    #[test]
    fn fibonacci_interleave_zero() -> crate::Result<()> {
        let src = FIB_32_BS.as_ref();
        let mut rdr = BitReader::new(src, FIB_32_OFF)?;
        let fib: Vec<u32> = Fibonacci::default().take(32).collect();
        for &v in fib.iter().rev() {
            rdr.flush();
            let u = unsafe { rdr.pull(32 - v.leading_zeros() as usize) as u32 };
            assert_eq!(v, u);
            let u = unsafe { rdr.pull(0) };
            assert_eq!(0, u);
        }
        assert_eq!(rdr.index * 8 + rdr.accum_bits, 64);
        rdr.finalize()?;
        Ok(())
    }

    #[test]
    fn overflow() -> crate::Result<()> {
        let bytes = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
        let src = bytes.as_ref();
        for off in 0..7 {
            let mut rdr = BitReader::new(src, off)?;
            for _ in 0..8 - off {
                assert_eq!(unsafe { rdr.pull(1) }, 0);
            }
            assert_eq!(rdr.index * 8 + rdr.accum_bits, 64);
            assert_eq!(unsafe { rdr.pull(1) }, 1);
            assert_eq!(rdr.index * 8 + rdr.accum_bits, 63);
        }
        Ok(())
    }
}
