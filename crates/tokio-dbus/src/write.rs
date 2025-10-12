use crate::{BodyBuf, Signature, buf::UnalignedBuf};

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A type who's reference can be written directly to a buffer.
///
/// These types are written using methods such as [`BodyBuf::store`].
///
/// [`BodyBuf::store`]: crate::BodyBuf::store
pub trait Write: self::sealed::Sealed {
    /// The signature of the type.
    #[doc(hidden)]
    const SIGNATURE: &'static Signature;

    /// Write `self` into `buf`.
    #[doc(hidden)]
    fn write_to(&self, buf: &mut BodyBuf);

    /// Write `self` into `buf`.
    #[doc(hidden)]
    fn write_to_unaligned(&self, buf: &mut UnalignedBuf);
}

impl self::sealed::Sealed for [u8] {}

/// Write a byte array to the buffer.
///
/// # Examples
///
/// ```
/// use tokio_dbus::BodyBuf;
///
/// let mut buf = BodyBuf::new();
/// buf.store(&b"foo"[..]);
///
/// assert_eq!(buf.signature(), "ay");
/// assert_eq!(buf.get(), &[3, 0, 0, 0, 102, 111, 111]);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
impl Write for [u8] {
    const SIGNATURE: &'static Signature = Signature::new_const(b"ay");

    #[inline]
    fn write_to(&self, buf: &mut BodyBuf) {
        buf.store_frame(self.len() as u32);
        buf.extend_from_slice(self);
    }

    #[inline]
    fn write_to_unaligned(&self, buf: &mut UnalignedBuf) {
        buf.store(self.len() as u32);
        buf.extend_from_slice(self);
    }
}

impl_traits_for_write!([u8], &b"abcd"[..], "qay");

impl self::sealed::Sealed for str {}

/// Write a length-prefixed string to the buffer.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};
///
/// let mut buf = BodyBuf::new();
/// buf.store("foo");
///
/// assert_eq!(buf.signature(), Signature::STRING);
/// assert_eq!(buf.get(), &[3, 0, 0, 0, 102, 111, 111, 0])
/// ```
impl Write for str {
    const SIGNATURE: &'static Signature = Signature::STRING;

    #[inline]
    fn write_to(&self, buf: &mut BodyBuf) {
        buf.store_frame(self.len() as u32);
        buf.extend_from_slice_nul(self.as_bytes());
    }

    #[inline]
    fn write_to_unaligned(&self, buf: &mut UnalignedBuf) {
        buf.store(self.len() as u32);
        buf.extend_from_slice_nul(self.as_bytes());
    }
}

impl_traits_for_write!(str, "Hello World", "qs");
