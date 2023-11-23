use std::marker::PhantomData;

use crate::ty::{self, Aligned};
use crate::{Frame, Write};

use super::{ArrayWriter, TypedStructWriter};

/// Write a typed array.
///
/// See [`BodyBuf::write_array`].
///
/// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
pub struct TypedArrayWriter<'a, T>
where
    T: Aligned,
{
    inner: ArrayWriter<'a, T>,
    _marker: PhantomData<T>,
}

impl<'a, T> TypedArrayWriter<'a, T>
where
    T: Aligned,
{
    pub(crate) fn new(inner: ArrayWriter<'a, T>) -> Self {
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
    #[inline]
    pub fn finish(self) {
        self.inner.finish();
    }
}

impl<'a, T> TypedArrayWriter<'a, T>
where
    T: Aligned,
{
    /// Store a value and return the builder for the next value to store.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    pub fn store(&mut self, value: T)
    where
        T: Frame,
    {
        self.inner.store(value);
    }

    /// Write a value and return the builder for the next value to store.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    pub fn write(&mut self, value: &T::Target)
    where
        T: ty::Unsized,
        T::Target: Write,
    {
        self.inner.write(value);
    }

    /// Write a struct inside of the array.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    #[inline]
    pub fn write_struct(&mut self) -> TypedStructWriter<'_, T>
    where
        T: ty::Fields,
    {
        TypedStructWriter::new(self.inner.write_struct())
    }
}

impl<'a, T> TypedArrayWriter<'a, ty::Array<T>>
where
    T: Aligned,
{
    /// Write an array inside of the array.
    ///
    /// See [`BodyBuf::write_array`].
    ///
    /// [`BodyBuf::write_array`]: crate::BodyBuf::write_array
    #[inline]
    pub fn write_array(&mut self) -> TypedArrayWriter<'_, T> {
        TypedArrayWriter::new(self.inner.write_array())
    }
}

impl<'a> TypedArrayWriter<'a, u8> {
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
