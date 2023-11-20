use std::mem::ManuallyDrop;

use crate::buf::{Alloc, BufMut, StructWriter};
use crate::{Frame, Write};

/// Write an array into a [`Buf`] where `E` is the type being written.
#[must_use = "Arrays must be finalized using ArrayWriter::finish"]
pub struct ArrayWriter<'a, O: ?Sized>
where
    O: BufMut,
{
    start: usize,
    len: Alloc<u32>,
    buf: &'a mut O,
}

impl<'a, O: ?Sized> ArrayWriter<'a, O>
where
    O: BufMut,
{
    pub(super) fn new(buf: &'a mut O) -> Self {
        let len = buf.alloc();
        let start = buf.len();

        Self { start, len, buf }
    }

    /// Finish writing the array.
    pub(crate) fn finish(self) {
        ManuallyDrop::new(self).finalize();
    }

    /// Store a [`Frame`] value into the array.
    #[inline]
    pub(crate) fn store<T>(&mut self, value: T)
    where
        T: Frame,
    {
        self.buf.store(value);
    }

    /// Write a value into the array.
    #[inline]
    pub(crate) fn write<T>(&mut self, value: &T)
    where
        T: ?Sized + Write,
    {
        value.write_to(self.buf);
    }

    /// Push an array inside of the array.
    #[inline]
    pub(crate) fn write_array(&mut self) -> ArrayWriter<'_, O> {
        ArrayWriter::new(self.buf)
    }

    /// Push an array inside of the array.
    #[inline]
    pub(crate) fn write_struct(&mut self) -> StructWriter<'_, O> {
        StructWriter::new(self.buf)
    }

    #[inline(always)]
    fn finalize(&mut self) {
        let end = self.buf.len();
        let len = (end - self.start) as u32;
        self.buf.store_at(self.len, len);
    }
}

impl<O: ?Sized> Drop for ArrayWriter<'_, O>
where
    O: BufMut,
{
    fn drop(&mut self) {
        self.finalize();
    }
}
