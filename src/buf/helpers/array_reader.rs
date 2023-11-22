use std::marker::PhantomData;

use crate::buf::{Buf, MAX_ARRAY_LENGTH};
use crate::error::ErrorKind;
use crate::ty;
use crate::{Error, Frame, Read, Result};

use super::StructReader;

/// Read an array from a buffer.
///
/// See [`Body::read_array`].
///
/// [`Body::read_array`]: crate::Body::read_array
pub struct ArrayReader<B, E> {
    buf: B,
    _marker: PhantomData<E>,
}

#[inline]
pub(crate) fn new_array_reader<'de, B, E>(mut buf: B) -> Result<ArrayReader<B::ReadUntil, E>>
where
    B: Buf<'de>,
    E: ty::Aligned,
{
    let bytes = buf.load::<u32>()?;

    if bytes > MAX_ARRAY_LENGTH {
        return Err(Error::new(ErrorKind::ArrayTooLong(bytes)));
    }

    let buf = buf.read_until(bytes as usize);

    Ok(ArrayReader::new(buf))
}

impl<'de, B, E> ArrayReader<B, E>
where
    B: Buf<'de>,
    E: ty::Aligned,
{
    /// Construct a new array reader around a buffer.
    pub(crate) fn new(buf: B) -> Self {
        ArrayReader {
            buf,
            _marker: PhantomData,
        }
    }

    /// Load the next value from the array.
    ///
    /// See [`Body::read_array`].
    ///
    /// [`Body::read_array`]: crate::Body::read_array
    pub fn load<T>(&mut self) -> Result<Option<T>>
    where
        T: Frame,
    {
        if self.buf.is_empty() {
            self.buf.align::<E::Type>()?;
            return Ok(None);
        }

        Ok(Some(self.buf.load()?))
    }

    /// Read the next value from the array.
    ///
    /// See [`Body::read_array`].
    ///
    /// [`Body::read_array`]: crate::Body::read_array
    pub fn read<T>(&mut self) -> Result<Option<&'de T>>
    where
        T: ?Sized + Read,
    {
        if self.buf.is_empty() {
            self.buf.align::<E::Type>()?;
            return Ok(None);
        }

        Ok(Some(T::read_from(&mut self.buf)?))
    }

    /// Read an array from within the array.
    ///
    /// See [`Body::read_struct`].
    pub fn read_array<U>(&mut self) -> Result<Option<ArrayReader<B::ReadUntil, U>>>
    where
        U: ty::Aligned,
    {
        if self.buf.is_empty() {
            self.buf.align::<E::Type>()?;
            return Ok(None);
        }

        Ok(Some(new_array_reader(self.buf.reborrow())?))
    }

    /// Read a struct from within the array.
    ///
    /// See [`Body::read_struct`].
    pub fn read_struct(&mut self) -> Result<Option<StructReader<B::Reborrow<'_>>>> {
        if self.buf.is_empty() {
            self.buf.align::<E::Type>()?;
            return Ok(None);
        }

        Ok(Some(StructReader::new(self.buf.reborrow())?))
    }
}
