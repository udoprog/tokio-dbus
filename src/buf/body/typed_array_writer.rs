use std::marker::PhantomData;

use crate::buf::AlignedBuf;
use crate::ty::{self, Aligned};
use crate::{Frame, Write};

use super::{ArrayWriter, TypedStructWriter};

/// Write a typed array.
///
/// See [`BodyBuf::write_array`].
///
/// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
pub struct TypedArrayWriter<'a, E>
where
    E: Aligned,
{
    inner: ArrayWriter<'a, AlignedBuf, E>,
    _marker: PhantomData<E>,
}

impl<'a, E> TypedArrayWriter<'a, E>
where
    E: Aligned,
{
    pub(crate) fn new(inner: ArrayWriter<'a, AlignedBuf, E>) -> Self {
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

impl<'a, E> TypedArrayWriter<'a, E>
where
    E: Aligned,
{
    /// Store a value and return the builder for the next value to store.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    pub fn store(&mut self, value: E)
    where
        E: Frame,
    {
        self.inner.store(value);
    }

    /// Write a value and return the builder for the next value to store.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    pub fn write(&mut self, value: &E::Target)
    where
        E: ty::Unsized,
        E::Target: Write,
    {
        self.inner.write(value);
    }

    /// Write a struct inside of the array.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    #[inline]
    pub fn write_struct(&mut self) -> TypedStructWriter<'_, E>
    where
        E: ty::Fields,
    {
        TypedStructWriter::new(self.inner.write_struct())
    }
}

impl<'a, E> TypedArrayWriter<'a, ty::Array<E>>
where
    E: Aligned,
{
    /// Write an array inside of the array.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    #[inline]
    pub fn write_array(&mut self) -> TypedArrayWriter<'_, E> {
        TypedArrayWriter::new(self.inner.write_array())
    }
}

impl<'a> TypedArrayWriter<'a, ty::Array<u8>> {
    /// Write a byte array inside of the array.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    #[inline]
    pub fn write_slice(&mut self, bytes: &[u8]) {
        self.inner.write_array::<u8>().write_slice(bytes);
    }
}
