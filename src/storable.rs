use crate::signature::SignatureBuilder;
use crate::{BodyBuf, Signature};

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
    fn store_to(self, buf: &mut BodyBuf);

    /// Write a signature.
    #[doc(hidden)]
    fn write_signature(builder: &mut SignatureBuilder) -> bool;
}

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
impl Storable for String {
    #[inline]
    fn store_to(self, buf: &mut BodyBuf) {
        self.as_str().store_to(buf);
    }

    #[inline]
    fn write_signature(builder: &mut SignatureBuilder) -> bool {
        builder.extend_from_signature(Signature::STRING)
    }
}
