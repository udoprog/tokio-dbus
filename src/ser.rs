use crate::frame::Frame;
use crate::OwnedBuf;

mod sealed {
    use crate::frame::Frame;
    use crate::sasl::SaslRequest;
    use crate::Signature;

    pub trait Sealed {}

    impl<T> Sealed for T where T: Frame {}
    impl Sealed for Signature {}
    impl Sealed for SaslRequest<'_> {}
    impl Sealed for [u8] {}
    impl<const N: usize> Sealed for [u8; N] {}
    impl Sealed for str {}
}

/// An element that can be serialized to a buffer.
pub trait Serialize: self::sealed::Sealed {
    /// Write `self` into `buf`.
    fn write_to(&self, buf: &mut OwnedBuf);
}

impl<T> Serialize for T
where
    T: Frame,
{
    #[inline]
    fn write_to(&self, buf: &mut OwnedBuf) {
        buf.store(self);
    }
}

/// Write a length-prefixed string to the buffer.
///
/// # Examples
///
/// ```
/// use tokio_dbus::OwnedBuf;
///
/// let mut buf = OwnedBuf::new();
/// buf.write(&b"foo"[..]);
///
/// assert_eq!(buf.get(), &[3, 0, 0, 0, 102, 111, 111, 0])
/// ```
impl Serialize for [u8] {
    #[inline]
    fn write_to(&self, buf: &mut OwnedBuf) {
        buf.store(&(self.len() as u32));
        buf.extend_from_slice_nul(self);
    }
}

/// Write a length-prefixed string to the buffer.
///
/// # Examples
///
/// ```
/// use tokio_dbus::OwnedBuf;
///
/// let mut buf = OwnedBuf::new();
/// buf.write("foo");
///
/// assert_eq!(buf.get(), &[3, 0, 0, 0, 102, 111, 111, 0])
/// ```
impl Serialize for str {
    #[inline]
    fn write_to(&self, buf: &mut OwnedBuf) {
        self.as_bytes().write_to(buf);
    }
}

/// Write a length-prefixed string to the buffer.
///
/// # Examples
///
/// ```
/// use tokio_dbus::OwnedBuf;
///
/// let mut buf = OwnedBuf::new();
/// buf.write(b"foo");
/// assert_eq!(buf.get(), &[3, 0, 0, 0, 102, 111, 111, 0])
/// ```
impl<const N: usize> Serialize for [u8; N] {
    #[inline]
    fn write_to(&self, buf: &mut OwnedBuf) {
        self[..].write_to(buf)
    }
}
