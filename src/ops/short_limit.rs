/// Length limit for short types.
///

///
/// * `SHORT_LIMIT <= i32::MAX`
pub unsafe trait ShortLimit {
    const SHORT_LIMIT: u32;
}

unsafe impl ShortLimit for &[u8] {
    const SHORT_LIMIT: u32 = i32::MAX as u32;
}

unsafe impl ShortLimit for Vec<u8> {
    const SHORT_LIMIT: u32 = i32::MAX as u32;
}

unsafe impl<T: ShortLimit + ?Sized> ShortLimit for &T {
    const SHORT_LIMIT: u32 = T::SHORT_LIMIT;
}

unsafe impl<T: ShortLimit + ?Sized> ShortLimit for &mut T {
    const SHORT_LIMIT: u32 = T::SHORT_LIMIT;
}
