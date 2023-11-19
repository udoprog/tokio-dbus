use crate::{Frame, OwnedBuf, Write};

/// Helper to write a struct into a buffer.
pub struct StructWriter<'a> {
    buf: &'a mut OwnedBuf,
}

impl<'a> StructWriter<'a> {
    #[inline]
    pub(super) fn new(buf: &'a mut OwnedBuf) -> Self {
        buf.align_mut::<u64>();
        Self { buf }
    }

    /// Store a value in the struct.
    #[inline]
    pub fn store<T>(&mut self, value: T)
    where
        T: Frame,
    {
        self.buf.store(value);
    }

    /// Write a field in the struct.
    #[inline]
    pub fn write<T>(&mut self, value: &T)
    where
        T: ?Sized + Write,
    {
        self.buf.write(value);
    }
}
