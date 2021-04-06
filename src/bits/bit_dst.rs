use crate::ops::{Allocate, Pos};

use std::io;
use std::mem;
use std::ptr;

/// BitWriter.
///
/// Memory must be allocated in advance via `Allocate`.
pub trait BitDst: Allocate + Pos {
    fn push_bytes(&mut self, bytes: usize, n_bytes: usize) {
        assert!(n_bytes <= mem::size_of::<usize>());
        unsafe { self.push_bytes_unchecked(bytes, n_bytes) }
    }

    /// Pushes bytes, as little-endian `usize` packed to the right with any unused bytes undefined.
    /// Usage after finalize undefined but not unsafe.
    ///
    /// # Panics
    ///
    /// Implementations may choose either to panic if insufficient memory is allocated or lazily
    /// throw an error on finalize.
    //
    /// # Safety
    ///
    /// * `n_bytes <= mem::size_of::<usize>()`
    unsafe fn push_bytes_unchecked(&mut self, bytes: usize, n_bytes: usize);

    fn finalize(&mut self) -> io::Result<()>;
}

impl<T: BitDst + ?Sized> BitDst for &mut T {
    #[inline(always)]
    unsafe fn push_bytes_unchecked(&mut self, bytes: usize, n_bytes: usize) {
        (**self).push_bytes_unchecked(bytes, n_bytes)
    }

    #[inline(always)]
    fn finalize(&mut self) -> io::Result<()> {
        (**self).finalize()
    }
}

impl BitDst for Vec<u8> {
    #[inline(always)]
    unsafe fn push_bytes_unchecked(&mut self, bytes: usize, n_bytes: usize) {
        let index = self.len();
        assert!(mem::size_of::<usize>() <= self.capacity() - self.len());
        let src = bytes.to_le_bytes().as_ptr();
        let dst = self.as_mut_ptr().add(index);
        ptr::copy_nonoverlapping(src, dst, mem::size_of::<usize>());
        self.set_len(index + n_bytes);
    }

    #[inline(always)]
    fn finalize(&mut self) -> io::Result<()> {
        Ok(())
    }
}
