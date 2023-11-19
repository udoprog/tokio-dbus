use std::mem::ManuallyDrop;

use crate::buf::owned_buf::Alloc;
use crate::buf::{OwnedBuf, StructWriter};
use crate::Serialize;

/// Write an array into a [`Buf`].
#[must_use = "Arrays must be finalized using ArrayWriter::finish"]
pub struct ArrayWriter<'a> {
    start: usize,
    len: Alloc<u32>,
    buf: &'a mut OwnedBuf,
}

impl<'a> ArrayWriter<'a> {
    pub(super) fn new(buf: &'a mut OwnedBuf) -> Self {
        let len = buf.alloc();
        let start = buf.len();
        Self { start, len, buf }
    }

    /// Push a value into the array.
    #[inline]
    pub fn push<T>(&mut self, value: &T)
    where
        T: ?Sized + Serialize,
    {
        value.write_to(self.buf);
    }

    /// Push a struct inside of the array.
    #[inline]
    pub fn write_struct(&mut self) -> StructWriter<'_> {
        StructWriter::new(self.buf)
    }

    /// Push an array inside of the array.
    #[inline]
    pub fn write_array(&mut self) -> ArrayWriter<'_> {
        ArrayWriter::new(self.buf)
    }

    /// Finish writing the array.
    pub fn finish(self) {
        ManuallyDrop::new(self).finalize();
    }

    #[inline(always)]
    fn finalize(&mut self) {
        let end = self.buf.len();
        let len = (end - self.start) as u32;
        self.buf.store_at(self.len, &len);
    }
}

impl Drop for ArrayWriter<'_> {
    fn drop(&mut self) {
        self.finalize();
    }
}
