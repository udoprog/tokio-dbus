#[cfg(feature = "alloc")]
use alloc::string::String;

use crate::WriteAligned;
use crate::signature::SignatureBuilder;

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Trait used for types which can be stored with a `store()` call.
///
/// # Examples
///
/// ```
/// use tokio_dbus::BodyBuf;
///
/// let mut body = BodyBuf::new();
///
/// body.store(10u16)?;
/// body.store("Hello World")?;
///
/// assert_eq!(body.signature(), "qs");
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
pub trait Storable: self::sealed::Sealed {
    /// Store a frame into a buffer body.
    #[doc(hidden)]
    fn store_to<B>(self, buf: &mut B)
    where
        B: ?Sized + WriteAligned;

    /// Write a signature.
    #[doc(hidden)]
    fn write_signature(builder: &mut SignatureBuilder) -> bool;
}

#[cfg(feature = "alloc")]
impl self::sealed::Sealed for String {}

/// [`Storable`] implementation for [`String`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::BodyBuf;
///
/// let mut body = BodyBuf::new();
///
/// body.store(10u16)?;
/// body.store(String::from("Hello World"))?;
///
/// assert_eq!(body.signature(), "qs");
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
#[cfg(feature = "alloc")]
impl Storable for String {
    #[inline]
    fn store_to<B>(self, buf: &mut B)
    where
        B: ?Sized + WriteAligned,
    {
        self.as_str().store_to(buf);
    }

    #[inline]
    fn write_signature(builder: &mut SignatureBuilder) -> bool {
        <&str as Storable>::write_signature(builder)
    }
}
