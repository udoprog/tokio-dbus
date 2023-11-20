use std::marker::PhantomData;

use crate::buf::{ArrayWriter, OwnedBuf, TypedStructWriter};
use crate::ty;
use crate::{Frame, Write};

/// Write a typed array.
pub struct TypedArrayWriter<'a, E> {
    inner: ArrayWriter<'a, OwnedBuf>,
    _marker: PhantomData<E>,
}

impl<'a, E> TypedArrayWriter<'a, E> {
    pub(super) fn new(inner: ArrayWriter<'a, OwnedBuf>) -> Self {
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

impl<'a, E> TypedArrayWriter<'a, E> {
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
    pub fn write_struct(&mut self) -> TypedStructWriter<'_, E>
    where
        E: ty::Fields,
    {
        TypedStructWriter::new(self.inner.write_struct())
    }
}

impl<'a, E> TypedArrayWriter<'a, ty::Array<E>> {
    /// Write an array inside of the array.
    #[inline]
    pub fn write_array(&mut self) -> TypedArrayWriter<'_, E> {
        TypedArrayWriter::new(self.inner.write_array())
    }
}
