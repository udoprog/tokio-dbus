use crate::ty;
use crate::{Arguments, Frame, Result, Write};

use crate::buf::BufMut;

use super::ArrayWriter;

/// Helper to write a struct into a buffer.
pub struct StructWriter<'a, O: ?Sized>
where
    O: BufMut,
{
    buf: &'a mut O,
}

impl<'a, O: ?Sized> StructWriter<'a, O>
where
    O: BufMut,
{
    #[inline]
    pub(crate) fn new(buf: &'a mut O) -> Self {
        buf.align_mut::<u64>();
        Self { buf }
    }

    /// Store a value in the struct.
    #[inline]
    pub(crate) fn store<T>(&mut self, value: T) -> Result<()>
    where
        T: Frame,
    {
        self.buf.store(value)
    }

    /// Write a field in the struct.
    #[inline]
    pub(crate) fn write<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Write,
    {
        value.write_to(self.buf)
    }

    /// Extend the current struct with the given arguments as fields.
    #[inline]
    pub(super) fn extend<T>(&mut self, value: T) -> Result<()>
    where
        T: Arguments,
    {
        value.buf_to(self.buf)
    }

    /// Write an array in the struct.
    ///
    /// The type parameter `A` indicates the alignment of the elements stored in
    /// the array.
    #[inline]
    pub(super) fn write_array<A>(&mut self) -> ArrayWriter<'_, O, A>
    where
        A: ty::Aligned,
    {
        ArrayWriter::new(self.buf)
    }

    /// Write an struct in the struct.
    #[inline]
    pub(super) fn write_struct(&mut self) -> StructWriter<'_, O> {
        StructWriter::new(self.buf)
    }
}
