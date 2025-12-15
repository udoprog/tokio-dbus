use core::str::from_utf8;

use crate::{Body, Error};

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A type who's reference can be read directly from a buffer.
pub trait Read: self::sealed::Sealed {
    /// Read the type from the given buffer.
    #[doc(hidden)]
    fn read_from<'de>(buf: &mut Body<'de>) -> Result<&'de Self, Error>;
}

impl self::sealed::Sealed for [u8] {}

impl Read for [u8] {
    #[inline]
    fn read_from<'de>(buf: &mut Body<'de>) -> Result<&'de Self, Error> {
        let len = buf.load::<u32>()? as usize;
        buf.load_slice(len)
    }
}

impl self::sealed::Sealed for str {}

impl Read for str {
    #[inline]
    fn read_from<'de>(buf: &mut Body<'de>) -> Result<&'de Self, Error> {
        let len = buf.load::<u32>()? as usize;
        let bytes = buf.load_slice_nul(len)?;
        Ok(from_utf8(bytes)?)
    }
}
