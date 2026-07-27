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

impl self::sealed::Sealed for bool {}

/// [`Storable`] implementation for [`bool`], which is marshalled as the D-Bus
/// `BOOLEAN` type.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};
///
/// let mut body = BodyBuf::new();
///
/// body.store(true)?;
///
/// assert_eq!(body.signature(), Signature::BOOLEAN);
/// assert_eq!(body.get(), &[1, 0, 0, 0]);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
impl Storable for bool {
    #[inline]
    fn store_to<B>(self, buf: &mut B)
    where
        B: ?Sized + WriteAligned,
    {
        buf.store_frame(u32::from(self));
    }

    #[inline]
    fn write_signature(builder: &mut SignatureBuilder) -> bool {
        builder.extend_from_signature(crate::Signature::BOOLEAN)
    }
}

impl crate::arguments::sealed::Sealed for bool {}

impl crate::Arguments for bool {
    #[inline]
    fn extend_to<B>(&self, buf: &mut B) -> crate::Result<()>
    where
        B: ?Sized + WriteAligned,
    {
        buf.store(*self)
    }

    #[inline]
    fn buf_to<B>(&self, buf: &mut B)
    where
        B: ?Sized + WriteAligned,
    {
        buf.store_frame(u32::from(*self));
    }
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
