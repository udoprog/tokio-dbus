use std::marker::PhantomData;

use crate::buf::BufMut;
use crate::ty::{self, Aligned};
use crate::{Frame, Result, Write};

use super::{ArrayWriter, TypedStructWriter};

/// Write a typed array.
///
/// See [`BodyBuf::write_array`].
///
/// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
pub struct TypedArrayWriter<'a, B, E>
where
    B: BufMut,
    E: Aligned,
{
    inner: ArrayWriter<'a, B, E>,
    _marker: PhantomData<E>,
}

impl<'a, B, E> TypedArrayWriter<'a, B, E>
where
    B: BufMut,
    E: Aligned,
{
    pub(crate) fn new(inner: ArrayWriter<'a, B, E>) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }

    /// Finish writing the array.
    ///
    /// This will also be done implicitly once this is dropped.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    pub fn finish(self) {
        self.inner.finish();
    }
}

impl<'a, B, E> TypedArrayWriter<'a, B, E>
where
    B: BufMut,
    E: Aligned,
{
    /// Store a value and return the builder for the next value to store.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    pub fn store(&mut self, value: E) -> Result<()>
    where
        E: Frame,
    {
        self.inner.store(value)
    }

    /// Write a value and return the builder for the next value to store.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    pub fn write(&mut self, value: &E::Target) -> Result<()>
    where
        E: ty::Unsized,
        E::Target: Write,
    {
        self.inner.write(value)
    }

    /// Write a struct inside of the array.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    #[inline]
    pub fn write_struct(&mut self) -> TypedStructWriter<'_, B, E>
    where
        E: ty::Fields,
    {
        TypedStructWriter::new(self.inner.write_struct())
    }
}

impl<'a, B, E> TypedArrayWriter<'a, B, ty::Array<E>>
where
    B: BufMut,
    E: Aligned,
{
    /// Write an array inside of the array.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    #[inline]
    pub fn write_array(&mut self) -> TypedArrayWriter<'_, B, E> {
        TypedArrayWriter::new(self.inner.write_array())
    }
}

impl<'a, B> TypedArrayWriter<'a, B, u8>
where
    B: BufMut,
{
    /// Write a byte array inside of the array.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    #[inline]
    pub fn write_slice(self, bytes: &[u8]) {
        self.inner.write_slice(bytes);
    }
}
