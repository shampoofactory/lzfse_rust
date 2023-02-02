use super::flush_limit::FlushLimit;

pub trait Flush: FlushLimit {
    /// Flush and empty the internal buffer to the internal destination.
    /// Implementations may partially flush the internal buffer unless `hard` is specified, in which
    /// case the entire buffer must be emptied.
    fn flush(&mut self, hard: bool) -> crate::Result<()>;
}

impl Flush for Vec<u8> {
    #[inline(always)]
    fn flush(&mut self, _: bool) -> crate::Result<()> {
        Ok(())
    }
}

impl<T: Flush + ?Sized> Flush for &mut T {
    #[inline(always)]
    fn flush(&mut self, hard: bool) -> crate::Result<()> {
        (**self).flush(hard)
    }
}
