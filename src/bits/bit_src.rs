use crate::ops::Len;
use crate::types::Idx;

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
    /// Negative idx values are permitted although the results are undefined.
    ///
    /// # Safety
    ///
    /// `base` has been called
    /// `idx + size_of::<usize>() <= self.len()`
    unsafe fn read_bytes(&self, idx: Idx) -> usize;

    /// Validate and return the base idx.
    /// Panic if the following conditions are not true:
    /// - `8 <= self.len())`
    /// - `self.len() <= u32::MAX as usize)`
    fn base(&self) -> Idx;
}

impl<'a> BitSrc for &'a [u8] {
    #[inline(always)]
    unsafe fn read_bytes(&self, idx: Idx) -> usize {
        let index = isize::from(idx);
        if index >= 0 {
            // Likely
            debug_assert!(index as usize + mem::size_of::<usize>() <= self.len());
            self.as_ptr().add(index as usize).cast::<usize>().read_unaligned().to_le()
        } else {
            // Unlikely
            0
        }
    }

    #[inline(always)]
    fn base(&self) -> Idx {
        assert!(8 <= self.len());
        assert!(self.len() <= u32::MAX as usize);
        Idx::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn read_bytes_9() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(Idx::new(9)) }, 0x3938_3736_3534_3332);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn read_bytes_8() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(Idx::new(8)) }, 0x3837_3635_3433_3231);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn read_bytes_0() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(Idx::new(0)) }, 0x2A2A_2A2A_2A2A_2A2A);
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn read_bytes_neg() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(Idx::default() - 1) }, 0x0000_0000_0000_0000);
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn read_bytes_9() {
        let src = b"********12345".as_ref();
        assert_eq!(unsafe { src.read_bytes(9) }, 0x3534_3332);
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn read_bytes_8() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(8) }, 0x3433_3231);
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn read_bytes_0() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(0) }, 0x2A2A_2A2A);
    }

    #[cfg(target_pointer_width = "32")]
    #[test]
    fn read_bytes_neg() {
        let src = b"********123456789".as_ref();
        assert_eq!(unsafe { src.read_bytes(-1) }, 0x0000_0000);
    }
}
