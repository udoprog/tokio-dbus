use std::marker::PhantomData;

use crate::buf::MAX_ARRAY_LENGTH;
use crate::error::ErrorKind;
use crate::{ty, Body};
use crate::{Error, Frame, Read, Result};

use super::StructReader;

/// Read an array from a buffer.
///
/// See [`Body::read_array`].
///
/// [`Body::read_array`]: crate::Body::read_array
pub struct ArrayReader<'de, E> {
    buf: Body<'de>,
    _marker: PhantomData<E>,
}

impl<'de, E> ArrayReader<'de, E> {
    #[inline]
    pub(crate) fn from_mut(buf: &mut Body<'de>) -> Result<ArrayReader<'de, E>> {
        let bytes = buf.load::<u32>()?;

        if bytes > MAX_ARRAY_LENGTH {
            return Err(Error::new(ErrorKind::ArrayTooLong(bytes)));
        }

        let buf = buf.read_until(bytes as usize);
        Ok(ArrayReader::new(buf))
    }

    /// Construct a new array reader around a buffer.
    pub(crate) fn new(buf: Body<'de>) -> Self {
        ArrayReader {
            buf,
            _marker: PhantomData,
        }
    }
}

impl<'de, E> ArrayReader<'de, E>
where
    E: Frame,
{
    /// Load the next value from the array.
    ///
    /// See [`Body::read_array`].
    ///
    /// [`Body::read_array`]: crate::Body::read_array
    pub fn load(&mut self) -> Result<Option<E>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(self.buf.load()?))
    }
}

impl<'de, E> ArrayReader<'de, E>
where
    E: ty::Unsized,
    E::Target: Read,
{
    /// Read the next value from the array.
    ///
    /// See [`Body::read_array`].
    ///
    /// [`Body::read_array`]: crate::Body::read_array
    pub fn read(&mut self) -> Result<Option<&'de E::Target>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(E::Target::read_from(&mut self.buf)?))
    }
}

impl<'de, E> ArrayReader<'de, ty::Array<E>>
where
    E: ty::Marker,
{
    /// Read an array from within the array.
    ///
    /// See [`Body::read_struct`].
    pub fn read_array(&mut self) -> Result<Option<ArrayReader<'de, E>>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(ArrayReader::from_mut(&mut self.buf)?))
    }
}

impl<'de, E> ArrayReader<'de, E>
where
    E: ty::Fields,
{
    /// Read a struct from within the array.
    ///
    /// See [`Body::read_struct`].
    pub fn read_struct(&mut self) -> Result<Option<StructReader<'_, 'de>>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(StructReader::new(&mut self.buf)?))
    }
}
