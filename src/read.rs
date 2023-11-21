use std::str::from_utf8;

use crate::{Error, ReadBuf};

mod sealed {
    use crate::Signature;

    pub trait Sealed {}
    impl Sealed for [u8] {}
    impl Sealed for str {}
    impl Sealed for Signature {}
}

/// An element that can be deserialize from a buffer.
pub trait Read: self::sealed::Sealed {
    /// Read the type from the given buffer.
    fn read_from<'de>(buf: &mut ReadBuf<'de>) -> Result<&'de Self, Error>;
}

impl Read for [u8] {
    #[inline]
    fn read_from<'de>(buf: &mut ReadBuf<'de>) -> Result<&'de Self, Error> {
        let len = buf.load::<u32>()? as usize;
        buf.load_slice(len)
    }
}

impl Read for str {
    #[inline]
    fn read_from<'de>(buf: &mut ReadBuf<'de>) -> Result<&'de Self, Error> {
        let len = buf.load::<u32>()? as usize;
        let bytes = buf.load_slice_nul(len)?;
        Ok(from_utf8(bytes)?)
    }
}
