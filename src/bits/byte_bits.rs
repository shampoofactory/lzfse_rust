use crate::ops::Len;

use super::bit_src::{BitSrc, NPopBytes};

use std::mem;

/// `BitSrc` wrapper over `&[u8]`.
#[derive(Clone, Copy)]
pub struct ByteBits<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> ByteBits<'a> {
    #[inline(always)]
    pub fn new(bytes: &'a [u8]) -> Self {
        assert!(bytes.len() >= 8);
        Self { bytes, index: 0 }
    }

    #[inline(always)]
    unsafe fn init_n(&mut self, n: usize) -> usize {
        assert!(n <= 1);
        debug_assert!(self.bytes.len() >= mem::size_of::<usize>());
        let len = self.bytes.len();
        self.index = len - (mem::size_of::<usize>() - n);
        self.bytes
            .as_ptr()
            .add(len - mem::size_of::<usize>())
            .cast::<usize>()
            .read_unaligned()
            .to_le()
            >> (n * 8)
    }
}

impl<'a> BitSrc for ByteBits<'a> {
    #[inline(always)]
    fn pop_bytes(&mut self, n_bytes: NPopBytes) -> usize {
        debug_assert!(self.index <= self.bytes.len() - (mem::size_of::<usize>() - 1));
        let n_bytes = n_bytes.get();
        if n_bytes == 0 {
            0
        } else if n_bytes <= self.index {
            self.index -= n_bytes;
            debug_assert!(self.index + mem::size_of::<usize>() <= self.bytes.len());
            unsafe { self.bytes.as_ptr().add(self.index).cast::<usize>().read_unaligned().to_le() }
        } else {
            self.index = 0;
            0
        }
    }

    fn init_1(&mut self) -> usize {
        unsafe { self.init_n(1) }
    }

    fn init_0(&mut self) -> usize {
        unsafe { self.init_n(0) }
    }
}

impl<'a> Len for ByteBits<'a> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_pointer_width = "64")]
    #[allow(clippy::erasing_op)]
    #[test]
    fn init_1_pop() {
        let bytes = b"********ABCDEFGHIJKLMNOP1234567";
        let mut bs = ByteBits::new(bytes);
        assert_eq!(bs.init_1(), 0x0037_3635_3433_3231);
        assert_eq!(bs.len(), 24);
        assert_eq!(bs.pop_bytes(NPopBytes::new(0)) & 0x0000_0000_0000_0000, 0x0000_0000_0000_0000);
        assert_eq!(bs.pop_bytes(NPopBytes::new(7)) & 0x00FF_FFFF_FFFF_FFFF, 0x0050_4F4E_4D4C_4B4A);
        assert_eq!(bs.pop_bytes(NPopBytes::new(5)) & 0x0000_00FF_FFFF_FFFF, 0x0000_0049_4847_4645);
        assert_eq!(bs.pop_bytes(NPopBytes::new(3)) & 0x0000_0000_00FF_FFFF, 0x0000_0000_0044_4342);
        assert_eq!(bs.pop_bytes(NPopBytes::new(1)) & 0x0000_0000_0000_00FF, 0x0000_0000_0000_0041);
        assert_eq!(bs.len(), 8);
        bs.pop_bytes(NPopBytes::new(7));
        assert_eq!(bs.len(), 1);
        bs.pop_bytes(NPopBytes::new(1));
        assert_eq!(bs.len(), 0);
        bs.pop_bytes(NPopBytes::new(1));
        assert_eq!(bs.len(), 0);
    }

    #[cfg(target_pointer_width = "64")]
    #[allow(clippy::erasing_op)]
    #[rustfmt::skip]
    #[test]
    fn test_init_0_pop() {
        let bytes = b"********ABCDEFGHIJKLMNOP12345678";
        let mut bs = ByteBits::new(bytes);
        assert_eq!(bs.init_0(), 0x3837363534333231);
        assert_eq!(bs.len(), 24);
        assert_eq!(bs.pop_bytes(NPopBytes::new(0)) & 0x0000_0000_0000_0000, 0x0000_0000_0000_0000);
        assert_eq!(bs.pop_bytes(NPopBytes::new(7)) & 0x00FF_FFFF_FFFF_FFFF, 0x0050_4F4E_4D4C_4B4A);
        assert_eq!(bs.pop_bytes(NPopBytes::new(5)) & 0x0000_00FF_FFFF_FFFF, 0x0000_0049_4847_4645);
        assert_eq!(bs.pop_bytes(NPopBytes::new(3)) & 0x0000_0000_00FF_FFFF, 0x0000_0000_0044_4342);
        assert_eq!(bs.pop_bytes(NPopBytes::new(1)) & 0x0000_0000_0000_00FF, 0x0000_0000_0000_0041);
        assert_eq!(bs.len(), 8);
        bs.pop_bytes(NPopBytes::new(7));
        assert_eq!(bs.len(), 1);
        bs.pop_bytes(NPopBytes::new(1));
        assert_eq!(bs.len(), 0);
        bs.pop_bytes(NPopBytes::new(1));
        assert_eq!(bs.len(), 0);
    }

    #[cfg(target_pointer_width = "32")]
    #[allow(clippy::erasing_op)]
    #[test]
    fn test_init_1_pop() {
        let bytes = b"********ABCD123";
        let mut bs = ByteBits::new(bytes);
        assert_eq!(bs.init_1(), 0x00333231);
        assert_eq!(bs.len(), 12);
        assert_eq!(bs.pop_bytes(NPopBytes::new(0)) & 0x0000_0000, 0x0000_0000);
        assert_eq!(bs.pop_bytes(NPopBytes::new(3)) & 0x00FF_FFFF, 0x0044_4342);
        assert_eq!(bs.pop_bytes(NPopBytes::new(1)) & 0x0000_00FF, 0x0000_0041);
        assert_eq!(bs.len(), 8);
        bs.pop_bytes(NPopBytes::new(3));
        assert_eq!(bs.len(), 5);
        bs.pop_bytes(NPopBytes::new(3));
        assert_eq!(bs.len(), 2);
        bs.pop_bytes(NPopBytes::new(3));
        assert_eq!(bs.len(), 0);
        bs.pop_bytes(NPopBytes::new(1));
        assert_eq!(bs.len(), 0);
    }

    #[cfg(target_pointer_width = "32")]
    #[allow(clippy::erasing_op)]
    #[test]
    fn test_init_0_pop() {
        let bytes = b"********ABCD1234";
        let mut bs = ByteBits::new(bytes);
        assert_eq!(bs.init_0(), 0x34333231);
        assert_eq!(bs.len(), 12);
        assert_eq!(bs.pop_bytes(NPopBytes::new(0)) & 0x0000_0000, 0x0000_0000);
        assert_eq!(bs.pop_bytes(NPopBytes::new(3)) & 0x00FF_FFFF, 0x0044_4342);
        assert_eq!(bs.pop_bytes(NPopBytes::new(1)) & 0x0000_00FF, 0x0000_0041);
        assert_eq!(bs.len(), 8);
        bs.pop_bytes(NPopBytes::new(3));
        assert_eq!(bs.len(), 5);
        bs.pop_bytes(NPopBytes::new(3));
        assert_eq!(bs.len(), 2);
        bs.pop_bytes(NPopBytes::new(3));
        assert_eq!(bs.len(), 0);
        bs.pop_bytes(NPopBytes::new(1));
        assert_eq!(bs.len(), 0);
    }
}
