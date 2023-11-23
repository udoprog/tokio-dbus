use crate::{Body, Frame, Read, Result};

use super::ArrayReader;
use crate::ty;

/// Read a struct from a buffer.
///
/// See [`Body::read_struct`].
///
/// [`Body::read_struct`]: crate::Body::read_struct
pub struct StructReader<'a, 'de> {
    buf: &'a mut Body<'de>,
}

impl<'a, 'de> StructReader<'a, 'de> {
    #[inline]
    pub(crate) fn new(buf: &'a mut Body<'de>) -> Result<Self> {
        buf.align::<u64>()?;
        Ok(Self { buf })
    }

    /// Reborrow the underlying buffer.
    #[inline]
    pub(crate) fn buf_mut(&mut self) -> &mut Body<'de> {
        &mut self.buf
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
        T::read_from(self.buf)
    }

    /// Read an array from within the struct.
    ///
    /// See [`Body::read_struct`].
    ///
    /// [`Body::read_struct`]: crate::Body::read_struct
    pub fn read_array<E>(&mut self) -> Result<ArrayReader<'de, E>>
    where
        E: ty::Marker,
    {
        ArrayReader::from_mut(self.buf)
    }
}
