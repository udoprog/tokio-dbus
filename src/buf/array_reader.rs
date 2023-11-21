use crate::buf::MAX_ARRAY_LENGTH;
use crate::error::ErrorKind;
use crate::{Error, Frame, Read, ReadBuf, Result};

/// Read an array from a buffer.
///
/// See [`ReadBuf::read_array`].
pub struct ArrayReader<'de> {
    buf: ReadBuf<'de>,
}

impl<'de> ArrayReader<'de> {
    #[inline]
    pub(super) fn new(buf: &mut ReadBuf<'de>) -> Result<Self> {
        let bytes = buf.load::<u32>()?;

        if bytes > MAX_ARRAY_LENGTH {
            return Err(Error::new(ErrorKind::ArrayTooLong(bytes)));
        }

        Ok(Self {
            buf: buf.read_until(bytes as usize),
        })
    }

    /// Load the next value from the array.
    ///
    /// See [`ReadBuf::read_array`].
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
    /// See [`ReadBuf::read_array`].
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
    /// See [`ReadBuf::read_struct`].
    pub fn read_array(&mut self) -> Result<Option<ArrayReader<'_>>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(ArrayReader::new(&mut self.buf)?))
    }
}
