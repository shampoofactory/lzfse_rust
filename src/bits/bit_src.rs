use crate::ops::Len;

use std::mem;

/// BitReader source.
///
/// Conceptually:
///
/// |PAD|DATA|
///       ^ index
///
/// PAD:  8 byte pad
/// DATA: data
pub trait BitSrc: Len {
    /// Reads data as little endian 'usize' from the specified index.
    ///
    /// Negative index values are permitted although the results are undefined.
    ///
    /// # Safety
    ///
    /// `index + size_of::<usize>() <= self.len()`
    unsafe fn read_bytes(&self, index: isize) -> usize;
}

impl<'a> BitSrc for &'a [u8] {
    #[inline(always)]
    unsafe fn read_bytes(&self, index: isize) -> usize {
        if index >= 0 {
            // Likely
            debug_assert!(index as usize + mem::size_of::<usize>() <= self.len());
            self.as_ptr().add(index as usize).cast::<usize>().read_unaligned().to_le()
        } else {
            // Unlikely
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn read_bytes_9() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(9) }, 0x3938_3736_3534_3332);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn read_bytes_8() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(8) }, 0x3837_3635_3433_3231);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn read_bytes_0() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(0) }, 0x2A2A_2A2A_2A2A_2A2A);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn read_bytes_neg() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(-1) }, 0x0000_0000_0000_0000);
    }
    // TODO 32 bit tests
}
