use crate::buf::{Buf, MAX_ARRAY_LENGTH};
use crate::error::ErrorKind;
use crate::{Error, Frame, Read, Result};

/// Read an array from a buffer.
///
/// See [`Body::read_array`].
///
/// [`Body::read_array`]: crate::Body::read_array
pub struct ArrayReader<B> {
    buf: B,
}

#[inline]
pub(crate) fn new_array_reader<'de, B>(mut buf: B) -> Result<ArrayReader<B::ReadUntil>>
where
    B: Buf<'de>,
{
    let bytes = buf.load::<u32>()?;

    if bytes > MAX_ARRAY_LENGTH {
        return Err(Error::new(ErrorKind::ArrayTooLong(bytes)));
    }

    let buf = buf.read_until(bytes as usize);
    Ok(ArrayReader { buf })
}

impl<'de, B> ArrayReader<B>
where
    B: Buf<'de>,
{
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
            return Ok(None);
        }

        Ok(Some(T::read_from(&mut self.buf)?))
    }

    /// Read an array from within the array.
    ///
    /// See [`Body::read_struct`].
    pub fn read_array(&mut self) -> Result<Option<ArrayReader<B::ReadUntil>>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(new_array_reader(self.buf.reborrow())?))
    }
}
