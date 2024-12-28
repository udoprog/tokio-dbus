use std::marker::PhantomData;

use crate::buf::MAX_ARRAY_LENGTH;
use crate::error::ErrorKind;
use crate::{ty, Body};
use crate::{Error, Frame, Read, Result};

/// Read an array from a buffer.
///
/// See [`Body::load_array`].
///
/// [`Body::load_array`]: crate::Body::load_array
pub struct LoadArray<'de, T> {
    buf: Body<'de>,
    _marker: PhantomData<T>,
}

impl<'de, T> LoadArray<'de, T> {
    #[inline]
    pub(crate) fn from_mut(buf: &mut Body<'de>) -> Result<LoadArray<'de, T>> {
        let bytes = buf.load::<u32>()?;

        if bytes > MAX_ARRAY_LENGTH {
            return Err(Error::new(ErrorKind::ArrayTooLong(bytes)));
        }

        let buf = buf.read_until(bytes as usize);
        Ok(LoadArray::new(buf))
    }

    /// Construct a new array reader around a buffer.
    pub(crate) fn new(buf: Body<'de>) -> Self {
        LoadArray {
            buf,
            _marker: PhantomData,
        }
    }
}

impl<T> LoadArray<'_, T>
where
    T: Frame,
{
    /// Load the next value from the array.
    ///
    /// See [`Body::load_array`].
    ///
    /// [`Body::load_array`]: crate::Body::load_array
    pub fn load(&mut self) -> Result<Option<T>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(self.buf.load()?))
    }
}

impl<'de, T> LoadArray<'de, T>
where
    T: ty::Unsized,
    T::Target: Read,
{
    /// Read the next value from the array.
    ///
    /// See [`Body::load_array`].
    ///
    /// [`Body::load_array`]: crate::Body::load_array
    pub fn read(&mut self) -> Result<Option<&'de T::Target>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(T::Target::read_from(&mut self.buf)?))
    }
}

impl<'de, T> LoadArray<'de, ty::Array<T>>
where
    T: ty::Marker,
{
    /// Read an array from within the array.
    ///
    /// See [`Body::load_struct`].
    pub fn load_array(&mut self) -> Result<Option<LoadArray<'de, T>>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(LoadArray::from_mut(&mut self.buf)?))
    }
}

impl<'de, T> LoadArray<'de, T>
where
    T: ty::Fields,
{
    /// Read a struct from within the array.
    ///
    /// See [`Body::load_struct`].
    pub fn load_struct(&mut self) -> Result<Option<T::Return<'de>>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(T::load_struct(&mut self.buf)?))
    }
}
