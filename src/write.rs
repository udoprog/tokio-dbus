use crate::{buf::UnalignedBuf, BodyBuf, Signature};

mod sealed {
    use crate::sasl::SaslRequest;
    use crate::{ObjectPath, Signature};

    pub trait Sealed {}

    impl Sealed for Signature {}
    impl Sealed for SaslRequest<'_> {}
    impl Sealed for [u8] {}
    impl Sealed for str {}
    impl Sealed for ObjectPath {}
}

/// A type who's reference can be written directly to a buffer.
///
/// These types are written using methods such as [`BodyBuf::write`].
///
/// [`BodyBuf::write`]: crate::BodyBuf::write
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

/// Write a byte array to the buffer.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};;
///
/// let mut buf = BodyBuf::new();
/// buf.write(&b"foo"[..]);
///
/// assert_eq!(buf.signature(), Signature::new(b"ay")?);
/// assert_eq!(buf.get(), &[3, 0, 0, 0, 102, 111, 111]);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
impl Write for [u8] {
    const SIGNATURE: &'static Signature = Signature::new_const(b"ay");

    #[inline]
    fn write_to(&self, buf: &mut BodyBuf) {
        buf.store_only(self.len() as u32);
        buf.extend_from_slice(self);
    }

    #[inline]
    fn write_to_unaligned(&self, buf: &mut UnalignedBuf) {
        buf.store(self.len() as u32);
        buf.extend_from_slice(self);
    }
}

/// Write a length-prefixed string to the buffer.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};;
///
/// let mut buf = BodyBuf::new();
/// buf.write("foo");
///
/// assert_eq!(buf.signature(), Signature::STRING);
/// assert_eq!(buf.get(), &[3, 0, 0, 0, 102, 111, 111, 0])
/// ```
impl Write for str {
    const SIGNATURE: &'static Signature = Signature::STRING;

    #[inline]
    fn write_to(&self, buf: &mut BodyBuf) {
        buf.store_only(self.len() as u32);
        buf.extend_from_slice_nul(self.as_bytes());
    }

    #[inline]
    fn write_to_unaligned(&self, buf: &mut UnalignedBuf) {
        buf.store(self.len() as u32);
        buf.extend_from_slice_nul(self.as_bytes());
    }
}
