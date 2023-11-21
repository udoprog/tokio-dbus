use crate::{buf::BufMut, Signature};

mod sealed {
    use crate::sasl::SaslRequest;
    use crate::Signature;

    pub trait Sealed {}

    impl Sealed for Signature {}
    impl Sealed for SaslRequest<'_> {}
    impl Sealed for [u8] {}
    impl Sealed for str {}
}

/// An element that can be serialized to a buffer.
pub trait Write: self::sealed::Sealed {
    /// The signature of the type.
    const SIGNATURE: &'static Signature;

    /// Write `self` into `buf`.
    fn write_to<O: ?Sized>(&self, buf: &mut O)
    where
        O: BufMut;
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
    fn write_to<O: ?Sized>(&self, buf: &mut O)
    where
        O: BufMut,
    {
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
    fn write_to<O: ?Sized>(&self, buf: &mut O)
    where
        O: BufMut,
    {
        buf.store(self.len() as u32);
        buf.extend_from_slice_nul(self.as_bytes());
    }
}
