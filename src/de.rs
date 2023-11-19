use std::str::from_utf8;

use crate::frame::Frame;
use crate::{Error, ReadBuf};

mod sealed {
    use crate::frame::Frame;
    use crate::Signature;

    pub trait Sealed {}
    impl Sealed for [u8] {}
    impl Sealed for str {}
    impl Sealed for Signature {}
    impl<T> Sealed for T where T: Frame {}
}

/// An element that can be deserialize from a buffer.
pub trait Deserialize: self::sealed::Sealed {
    /// Read the type from the given buffer.
    fn read_from<'de>(buf: &mut ReadBuf<'de>) -> Result<&'de Self, Error>;
}

impl<T> Deserialize for T
where
    T: Frame,
{
    #[inline]
    fn read_from<'de>(buf: &mut ReadBuf<'de>) -> Result<&'de Self, Error> {
        buf.load()
    }
}

impl Deserialize for [u8] {
    #[inline]
    fn read_from<'de>(buf: &mut ReadBuf<'de>) -> Result<&'de Self, Error> {
        let len = *buf.load::<u32>()? as usize;
        buf.load_slice_nul(len)
    }
}

impl Deserialize for str {
    #[inline]
    fn read_from<'de>(buf: &mut ReadBuf<'de>) -> Result<&'de Self, Error> {
        let bytes = <[u8]>::read_from(buf)?;
        Ok(from_utf8(bytes)?)
    }
}
