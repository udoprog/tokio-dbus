use std::str::from_utf8;

use crate::buf::Buf;
use crate::Error;

mod sealed {
    use crate::{ObjectPath, Signature};

    pub trait Sealed {}
    impl Sealed for [u8] {}
    impl Sealed for [u16] {}
    impl Sealed for str {}
    impl Sealed for ObjectPath {}
    impl Sealed for Signature {}
}

/// A type who's reference can be read directly from a buffer.
pub trait Read: self::sealed::Sealed {
    /// Read the type from the given buffer.
    #[doc(hidden)]
    fn read_from<'de, B>(buf: B) -> Result<&'de Self, Error>
    where
        B: Buf<'de>;
}

impl Read for [u8] {
    #[inline]
    fn read_from<'de, B>(mut buf: B) -> Result<&'de Self, Error>
    where
        B: Buf<'de>,
    {
        let len = buf.load::<u32>()? as usize;
        buf.load_slice(len)
    }
}

impl Read for str {
    #[inline]
    fn read_from<'de, B>(mut buf: B) -> Result<&'de Self, Error>
    where
        B: Buf<'de>,
    {
        let len = buf.load::<u32>()? as usize;
        let bytes = buf.load_slice_nul(len)?;
        Ok(from_utf8(bytes)?)
    }
}
