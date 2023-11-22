use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use crate::buf::{Alloc, BufMut};
use crate::ty;
use crate::{Frame, Write};

use super::StructWriter;

/// Write an array into a [`BufMut`].
///
/// Note that this does not enforce that the elements being written have a
/// uniform type.
#[must_use = "Arrays must be finalized using ArrayWriter::finish"]
pub(crate) struct ArrayWriter<'a, O: ?Sized, A>
where
    O: BufMut,
    A: ty::Aligned,
{
    start: usize,
    len: Alloc<u32>,
    buf: &'a mut O,
    _marker: PhantomData<A>,
}

impl<'a, O: ?Sized, A> ArrayWriter<'a, O, A>
where
    O: BufMut,
    A: ty::Aligned,
{
    pub(crate) fn new(buf: &'a mut O) -> Self {
        let len = buf.alloc();
        let start = buf.len();

        Self {
            start,
            len,
            buf,
            _marker: PhantomData,
        }
    }

    /// Finish writing the array.
    pub(crate) fn finish(self) {
        ManuallyDrop::new(self).finalize();
    }

    /// Store a [`Frame`] value into the array.
    #[inline]
    pub(super) fn store<T>(&mut self, value: T)
    where
        T: Frame,
    {
        self.buf.store(value);
    }

    /// Write a value into the array.
    #[inline]
    pub(super) fn write<T>(&mut self, value: &T)
    where
        T: ?Sized + Write,
    {
        value.write_to(self.buf);
    }

    /// Push an array inside of the array.
    #[inline]
    pub(super) fn write_array<B>(&mut self) -> ArrayWriter<'_, O, B>
    where
        B: ty::Aligned,
    {
        ArrayWriter::new(self.buf)
    }

    /// Push an array inside of the array.
    #[inline]
    pub(crate) fn write_struct(&mut self) -> StructWriter<'_, O> {
        StructWriter::new(self.buf)
    }

    /// Write the array as a slice.
    #[inline]
    pub(crate) fn write_slice(self, data: &[u8]) {
        let mut this = ManuallyDrop::new(self);
        this.buf.extend_from_slice(data);
        this.finalize();
    }

    #[inline(always)]
    fn finalize(&mut self) {
        let end = self.buf.len();
        let len = (end - self.start) as u32;
        self.buf.store_at(self.len, len);
        self.buf.align_mut::<A::Type>();
    }
}

impl<O: ?Sized, A> Drop for ArrayWriter<'_, O, A>
where
    O: BufMut,
    A: ty::Aligned,
{
    fn drop(&mut self) {
        self.finalize();
    }
}
