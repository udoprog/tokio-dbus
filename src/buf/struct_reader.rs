use crate::{Deserialize, Error, ReadBuf};

/// Write an struct into a [`Buf`].
pub struct StructReader<'a, 'de> {
    buf: &'a mut ReadBuf<'de>,
}

impl<'a, 'de> StructReader<'a, 'de> {
    #[inline]
    pub(super) fn new(buf: &'a mut ReadBuf<'de>) -> Self {
        buf.align::<u64>();
        Self { buf }
    }

    /// Read a a field from the struct.
    pub fn read<T>(&mut self) -> Result<&'de T, Error>
    where
        T: ?Sized + Deserialize,
    {
        T::read_from(self.buf)
    }
}
