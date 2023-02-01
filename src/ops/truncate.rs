use crate::types::Idx;

use super::pos::Pos;

pub trait Truncate: Pos {
    /// Truncate to `idx` returning true if the action is successful.
    /// Truncating to greater than `i32::MAX` relative to `self.pos()` is undefined.
    #[must_use]
    fn truncate(&mut self, idx: Idx) -> bool;
}

impl Truncate for Vec<u8> {
    fn truncate(&mut self, idx: Idx) -> bool {
        let delta = self.pos() - idx;
        let index = (self.len() as isize - delta as isize) as usize;
        if index <= self.len() {
            unsafe { self.set_len(index) };
            true
        } else {
            false
        }
    }
}

impl<T: Truncate + ?Sized> Truncate for &mut T {
    #[inline(always)]
    fn truncate(&mut self, idx: Idx) -> bool {
        (**self).truncate(idx)
    }
}
