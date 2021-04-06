use crate::bits::{BitSrc, NPopBytes};
use crate::ops::{Len, Pos};
use crate::types::Idx;

use super::ring_size::RingSize;
use super::ring_type::RingType;
use super::ring_view::RingView;

use std::mem;

#[derive(Copy, Clone)]
pub struct RingBits<'a, T> {
    view: RingView<'a, T>,
    idx: Idx,
}

impl<'a, T: RingType> RingBits<'a, T> {
    #[inline(always)]
    pub fn new(view: RingView<'a, T>) -> Self {
        assert!(T::RING_LIMIT as usize >= mem::size_of::<usize>());
        let idx = view.tail;
        Self { view, idx }
    }
}

impl<'a, T> Pos for RingBits<'a, T> {
    #[inline(always)]
    fn pos(&self) -> Idx {
        self.idx
    }
}

impl<'a, T: RingSize> RingBits<'a, T> {
    #[inline(always)]
    unsafe fn pop(&mut self, n: usize) -> usize {
        self.idx -= n as u32;
        let index = self.idx % T::RING_SIZE;
        debug_assert!(n <= 8);
        self.view.ring_ptr.add(index as usize).cast::<usize>().read_unaligned().to_le()
    }
}

impl<'a, T: RingSize> BitSrc for RingBits<'a, T> {
    #[inline(always)]
    fn pop_bytes(&mut self, n_bytes: NPopBytes) -> usize {
        debug_assert_ne!(self.idx, self.view.tail);
        unsafe { self.pop(n_bytes.get()) }
    }

    fn init_1(&mut self) -> usize {
        self.idx = self.view.tail;
        unsafe { (self.pop(mem::size_of::<usize>() - 1) << 8) >> 8 }
    }

    fn init_0(&mut self) -> usize {
        self.idx = self.view.tail;
        unsafe { self.pop(mem::size_of::<usize>()) }
    }
}

impl<'a, T: RingSize> Len for RingBits<'a, T> {
    #[inline(always)]
    fn len(&self) -> usize {
        let len = self.idx - self.view.head;
        if len < 0 {
            0
        } else {
            len as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ring::{Ring, RingBox, RingType};

    use super::*;

    struct T;

    unsafe impl RingSize for T {
        const RING_SIZE: u32 = 64;
    }

    unsafe impl RingType for T {
        const RING_LIMIT: u32 = mem::size_of::<usize>() as u32;
    }

    #[cfg(target_pointer_width = "64")]
    #[allow(clippy::erasing_op)]
    #[rustfmt::skip]
    #[test]
    fn test_init_0_pop() {
        let bytes = b"********ABCDEFGHIJKLMNOP12345678";
        let mut ring_box = RingBox::<T>::default();
        let mut ring = Ring::from(&mut ring_box);
        (&mut ring[..bytes.len()]).copy_from_slice(bytes);
        ring.head_copy_in();
        let view = ring.view(Idx::new(0), Idx::new(bytes.len() as u32));
        let mut bs = RingBits::new(view);
        assert_eq!(bs.init_0(), 0x3837363534333231);
        assert_eq!(bs.len(), 24);
        assert_eq!(bs.pop_bytes(NPopBytes::new(0)) & 0x0000000000000000, 0x0000000000000000);
        assert_eq!(bs.pop_bytes(NPopBytes::new(7)) & 0x00FFFFFFFFFFFFFF, 0x00504F4E4D4C4B4A);
        assert_eq!(bs.pop_bytes(NPopBytes::new(5)) & 0x000000FFFFFFFFFF, 0x0000004948474645);
        assert_eq!(bs.pop_bytes(NPopBytes::new(3)) & 0x0000000000FFFFFF, 0x0000000000444342);
        assert_eq!(bs.pop_bytes(NPopBytes::new(1)) & 0x00000000000000FF, 0x0000000000000041);
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
    fn test_init_1_pop() {
        let bytes = b"********ABCDEFGHIJKLMNOP1234567";
        let mut ring_box = RingBox::<T>::default();
        let mut ring = Ring::from(&mut ring_box);
        (&mut ring[..bytes.len()]).copy_from_slice(bytes);
        ring.head_copy_in();
        let view = ring.view(Idx::new(0), Idx::new(bytes.len() as u32));
        let mut bs = RingBits::new(view);
        assert_eq!(bs.init_1(), 0x0037363534333231);
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
        let mut ring_box = RingBox::<T>::default();
        let mut ring = Ring::from(&mut ring_box);
        (&mut ring[..bytes.len()]).copy_from_slice(bytes);
        ring.head_copy_in();
        let view = ring.view(Idx::new(0), Idx::new(bytes.len() as u32));
        let mut bs = RingBits::new(view);
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
        let mut ring_box = RingBox::<T>::default();
        let mut ring = Ring::from(&mut ring_box);
        (&mut ring[..bytes.len()]).copy_from_slice(bytes);
        ring.head_copy_in();
        let view = ring.view(Idx::new(0), Idx::new(bytes.len() as u32));
        let mut bs = RingBits::new(view);
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
