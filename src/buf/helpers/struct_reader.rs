use crate::{Frame, Read, Result};

use super::{new_array_reader, ArrayReader};
use crate::buf::Buf;

/// Read a struct from a buffer.
///
/// See [`Body::read_struct`].
///
/// [`Body::read_struct`]: crate::Body::read_struct
pub struct StructReader<B> {
    buf: B,
}

impl<'de, B> StructReader<B>
where
    B: Buf<'de>,
{
    #[inline]
    pub(crate) fn new(mut buf: B) -> Result<Self> {
        buf.align::<u64>()?;
        Ok(Self { buf })
    }

    /// Load a field from the struct.
    ///
    /// See [`Body::read_struct`].
    ///
    /// [`Body::read_struct`]: crate::Body::read_struct
    pub fn load<T>(&mut self) -> Result<T>
    where
        T: Frame,
    {
        self.buf.load()
    }

    /// Read a field from the struct.
    ///
    /// See [`Body::read_struct`].
    ///
    /// [`Body::read_struct`]: crate::Body::read_struct
    pub fn read<T>(&mut self) -> Result<&'de T>
    where
        T: ?Sized + Read,
    {
        T::read_from(self.buf.reborrow())
    }

    /// Read an array from within the struct.
    ///
    /// See [`Body::read_struct`].
    ///
    /// [`Body::read_struct`]: crate::Body::read_struct
    pub fn read_array(&mut self) -> Result<ArrayReader<B::ReadUntil>> {
        new_array_reader(self.buf.reborrow())
    }
}
