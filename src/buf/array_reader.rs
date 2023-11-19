use crate::{Error, Read, ReadBuf};

/// Write an struct into a [`Buf`].
pub struct ArrayReader<'de> {
    buf: ReadBuf<'de>,
}

impl<'de> ArrayReader<'de> {
    #[inline]
    pub(super) fn new(buf: &mut ReadBuf<'de>) -> Result<Self, Error> {
        let bytes = buf.load::<u32>()?;
        Ok(Self {
            buf: buf.read_buf(bytes as usize),
        })
    }

    /// Read a a field from the struct.
    pub fn read_next<T>(&mut self) -> Result<Option<&'de T>, Error>
    where
        T: ?Sized + Read,
    {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(T::read_from(&mut self.buf)?))
    }
}
