use crate::ops::Len;

use super::BitSrc;

pub trait AsBitSrc: Len {
    type BitSrc: BitSrc;

    /// `self.len()` minimum of 8 bytes.
    fn as_bit_src(self) -> Self::BitSrc;
}

impl<'a> AsBitSrc for &'a [u8] {
    type BitSrc = Self;

    #[inline(always)]
    fn as_bit_src(self) -> Self::BitSrc {
        assert!(self.len() >= 8);
        self
    }
}
