use crate::types::Idx;

pub trait Pos {
    /// Current read/ write position. Result returned as a wrapping Idx.
    ///
    /// - `size_of::<usize>() == 4`: Idx corresponds to position..
    /// - `size_of::<usize>() == 4`: Idx corresponds to a wrapped position. As such only the tail
    ///                            : 0x8000_0000 positions are accessible.
    fn pos(&self) -> Idx;
}

impl Pos for Vec<u8> {
    #[inline(always)]
    fn pos(&self) -> Idx {
        (self.len() as u32).into()
    }
}

impl<T: Pos + ?Sized> Pos for &T {
    #[inline(always)]
    fn pos(&self) -> Idx {
        (**self).pos()
    }
}

impl<T: Pos + ?Sized> Pos for &mut T {
    #[inline(always)]
    fn pos(&self) -> Idx {
        (**self).pos()
    }
}
