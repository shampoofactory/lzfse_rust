use crate::ops::Len;

use std::mem;

/// Constrained type: 0 <= u < size_of::<usize>()
#[derive(Copy, Clone, Debug)]
pub struct NPopBytes(usize);

impl NPopBytes {
    #[allow(dead_code)]
    #[inline(always)]
    pub fn new(n: usize) -> Self {
        assert!(n < mem::size_of::<usize>());
        Self(n)
    }

    #[inline(always)]
    pub unsafe fn new_unchecked(n: usize) -> Self {
        debug_assert!(n < mem::size_of::<usize>());
        Self(n)
    }

    #[inline(always)]
    pub fn get(self) -> usize {
        self.0
    }
}

/// BitReader source. Lazy underflow state management. 8 byte padded (undefined).
pub trait BitSrc: Len {
    /// Pops bytes, as little-endian `usize` packed to the right with any unused bytes undefined.
    /// Usage before initialization undefined not unsafe.
    fn pop_bytes(&mut self, n_bytes: NPopBytes) -> usize;

    /// Initialize and pop `size_of::<usize> - 1` bytes with unused bytes set to zero.
    fn init_1(&mut self) -> usize;

    /// Initialize and pop `size_of::<usize>` bytes.
    fn init_0(&mut self) -> usize;
}
