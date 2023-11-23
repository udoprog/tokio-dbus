use crate::ty;
use crate::{Arguments, BodyBuf, Frame, Write};

use super::ArrayWriter;

/// Helper to write a struct into a buffer.
pub struct StructWriter<'a> {
    buf: &'a mut BodyBuf,
}

impl<'a> StructWriter<'a> {
    #[inline]
    pub(crate) fn new(buf: &'a mut BodyBuf) -> Self {
        buf.align_mut::<u64>();
        Self { buf }
    }

    /// Store a value in the struct.
    #[inline]
    pub(crate) fn store<T>(&mut self, value: T)
    where
        T: Frame,
    {
        self.buf.store_only(value);
    }

    /// Write a field in the struct.
    #[inline]
    pub(crate) fn write<T>(&mut self, value: &T)
    where
        T: ?Sized + Write,
    {
        value.write_to(self.buf);
    }

    /// Extend the current struct with the given arguments as fields.
    #[inline]
    pub(super) fn extend<T>(&mut self, value: T)
    where
        T: Arguments,
    {
        value.buf_to(self.buf);
    }

    /// Write an array in the struct.
    ///
    /// The type parameter `A` indicates the alignment of the elements stored in
    /// the array.
    #[inline]
    pub(super) fn write_array<A>(&mut self) -> ArrayWriter<'_, A>
    where
        A: ty::Aligned,
    {
        ArrayWriter::new(self.buf)
    }

    /// Write an struct in the struct.
    #[inline]
    pub(super) fn write_struct(&mut self) -> StructWriter<'_> {
        StructWriter::new(self.buf)
    }
}
