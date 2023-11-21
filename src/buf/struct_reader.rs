use crate::{Frame, Read, ReadBuf, Result};

use super::ArrayReader;

/// Read a struct from a buffer.
///
/// See [`ReadBuf::read_struct`].
pub struct StructReader<'a, 'de> {
    buf: &'a mut ReadBuf<'de>,
}

impl<'a, 'de> StructReader<'a, 'de> {
    #[inline]
    pub(super) fn new(buf: &'a mut ReadBuf<'de>) -> Result<Self> {
        buf.align::<u64>()?;
        Ok(Self { buf })
    }

    /// Load a field from the struct.
    ///
    /// See [`ReadBuf::read_struct`].
    pub fn load<T>(&mut self) -> Result<T>
    where
        T: Frame,
    {
        self.buf.load()
    }

    /// Read a field from the struct.
    ///
    /// See [`ReadBuf::read_struct`].
    pub fn read<T>(&mut self) -> Result<&'de T>
    where
        T: ?Sized + Read,
    {
        T::read_from(self.buf)
    }

    /// Read an array from within the struct.
    ///
    /// See [`ReadBuf::read_struct`].
    pub fn read_array(&mut self) -> Result<ArrayReader<'a>> {
        ArrayReader::new(self.buf)
    }
}
