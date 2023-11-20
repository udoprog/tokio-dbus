use std::marker::PhantomData;

use crate::buf::{ArrayWriter, BufMut, TypedStructWriter};
use crate::ty;
use crate::{Frame, Write};

/// Write a typed array.
pub struct TypedArrayWriter<'a, O, E>
where
    O: BufMut,
{
    inner: ArrayWriter<'a, O>,
    _marker: PhantomData<E>,
}

impl<'a, O, E> TypedArrayWriter<'a, O, E>
where
    O: BufMut,
{
    pub(super) fn new(inner: ArrayWriter<'a, O>) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }

    /// Finish writing the array.
    ///
    /// This will also be done implicitly once this is dropped.
    pub fn finish(self) {
        self.inner.finish();
    }
}

impl<'a, O, E> TypedArrayWriter<'a, O, E>
where
    O: BufMut,
{
    /// Store a value and return the builder for the next value to store.
    pub fn store(&mut self, value: E)
    where
        E: Frame,
    {
        self.inner.store(value);
    }

    /// Write a value and return the builder for the next value to store.
    pub fn write(&mut self, value: &E::Target)
    where
        E: ty::Unsized,
        E::Target: Write,
    {
        self.inner.write(value);
    }

    /// Write a struct inside of the array.
    #[inline]
    pub fn write_struct(&mut self) -> TypedStructWriter<'_, O, E>
    where
        E: ty::Fields,
    {
        TypedStructWriter::new(self.inner.write_struct())
    }
}

impl<'a, O, E> TypedArrayWriter<'a, O, ty::Array<E>>
where
    O: BufMut,
{
    /// Write an array inside of the array.
    #[inline]
    pub fn write_array(&mut self) -> TypedArrayWriter<'_, O, E> {
        TypedArrayWriter::new(self.inner.write_array())
    }
}
