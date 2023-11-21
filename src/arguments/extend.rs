use crate::error::Result;
use crate::{Frame, Write};

mod sealed {
    use crate::BodyBuf;
    pub trait Sealed {}
    impl Sealed for BodyBuf {}
}

/// Trait governing types which can be extended with [`Collection`].
///
/// Like [`BodyBuf::extend`].
///
/// [`BodyBuf::extend`]: crate::BodyBuf::extend
pub trait Extend: self::sealed::Sealed {
    /// Write a [`Write`] of type `T` in the buffer.
    #[doc(hidden)]
    fn write<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Write;

    /// Store a [`Frame`] of type `T` in the buffer.
    ///
    /// This both allocates enough space for the frame and ensures that the
    /// buffer is aligned per the requirements of the frame.
    #[doc(hidden)]
    fn store<T>(&mut self, frame: T) -> Result<()>
    where
        T: Frame;
}
