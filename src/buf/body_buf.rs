use crate::buf::{OwnedBuf, TypedArrayWriter, TypedStructWriter};
use crate::{ty, Endianness, Frame, OwnedSignature, ReadBuf, Signature, Write};

/// A buffer that can be used to write a body.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};
///
/// let mut body = BodyBuf::new();
///
/// body.store(10u16);
/// body.store(10u32);
///
/// assert_eq!(body.signature(), Signature::new(b"qu")?);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
pub struct BodyBuf {
    signature: OwnedSignature,
    buf: OwnedBuf,
}

impl BodyBuf {
    /// Construct a new empty body buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u16);
    /// body.store(10u32);
    ///
    /// assert_eq!(body.signature(), Signature::new(b"qu")?);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn new() -> Self {
        Self::with_endianness(Endianness::NATIVE)
    }

    /// Clear the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u16);
    /// body.store(10u32);
    ///
    /// assert_eq!(body.signature(), Signature::new(b"qu")?);
    /// body.clear();
    /// assert_eq!(body.signature(), Signature::empty());
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn clear(&mut self) {
        self.signature.clear();
        self.buf.clear();
    }

    /// Construct a new buffer with the specified endianness.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// ```
    pub fn with_endianness(endianness: Endianness) -> Self {
        Self {
            signature: OwnedSignature::new(),
            buf: OwnedBuf::with_endianness(endianness),
        }
    }

    /// Get the signature of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u16);
    /// body.store(10u32);
    ///
    /// assert_eq!(body.signature(), Signature::new(b"qu")?);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Get the endianness of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut body = BodyBuf::new();
    /// assert_eq!(body.endianness(), Endianness::NATIVE);
    ///
    /// body.set_endianness(Endianness::BIG);
    /// assert_eq!(body.endianness(), Endianness::BIG);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn endianness(&self) -> Endianness {
        self.buf.endianness()
    }

    /// Get a slice out of the buffer that has ben written to.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature, Endianness};
    ///
    /// let mut body = BodyBuf::with_endianness(Endianness::LITTLE);
    ///
    /// body.store(10u16);
    /// body.store(10u32);
    ///
    /// assert_eq!(body.signature(), Signature::new(b"qu")?);
    /// assert_eq!(body.get(), &[10, 0, 0, 0, 10, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn get(&self) -> &[u8] {
        self.buf.get()
    }

    /// Return a read buffer over the entire contents of this buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature, Endianness};
    ///
    /// let mut body = BodyBuf::with_endianness(Endianness::LITTLE);
    ///
    /// body.store(10u16);
    /// body.store(20u32);
    ///
    /// assert_eq!(body.signature(), Signature::new(b"qu")?);
    ///
    /// let mut buf = body.read();
    /// assert_eq!(buf.load::<u16>()?, 10);
    /// assert_eq!(buf.load::<u32>()?, 20);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn read(&self) -> ReadBuf<'_> {
        let len = self.buf.len();
        self.buf.peek_buf(len)
    }

    /// Set the endianness of the buffer.
    ///
    /// Note that this will not affect any data that has already been written to
    /// the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut body = BodyBuf::new();
    /// assert_eq!(body.endianness(), Endianness::NATIVE);
    ///
    /// body.set_endianness(Endianness::BIG);
    /// assert_eq!(body.endianness(), Endianness::BIG);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn set_endianness(&mut self, endianness: Endianness) {
        self.buf.set_endianness(endianness);
    }

    /// Store a [`Frame`] of type `T` in the buffer and add its signature.
    ///
    /// This both allocates enough space for the frame and ensures that the
    /// buffer is aligned per the requirements of the frame.
    pub fn store<T>(&mut self, frame: T)
    where
        T: Frame,
    {
        self.signature.extend_from_signature(T::SIGNATURE);
        self.buf.store(frame);
    }

    /// Write a type which can be serialized.
    pub fn write<T>(&mut self, value: &T)
    where
        T: ?Sized + Write,
    {
        self.signature.extend_from_signature(T::SIGNATURE);
        self.buf.write(value);
    }

    /// Write an array into the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// let mut array = buf.write_array::<u32>();
    /// array.store(1u32);
    /// array.finish();
    ///
    /// assert_eq!(buf.signature(), b"au");
    /// assert_eq!(buf.get(), &[4, 0, 0, 0, 1, 0, 0, 0]);
    /// ```
    pub fn write_array<E>(&mut self) -> TypedArrayWriter<'_, E>
    where
        E: ty::Marker,
    {
        <ty::Array<E> as ty::Marker>::write_signature(&mut self.signature);
        TypedArrayWriter::new(self.buf.write_array())
    }

    /// Write a slice as an byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// buf.write_slice(&[1, 2, 3, 4]);
    ///
    /// assert_eq!(buf.signature(), b"ay");
    /// assert_eq!(buf.get(), &[4, 0, 0, 0, 1, 2, 3, 4]);
    /// ```
    pub fn write_slice(&mut self, data: &[u8]) {
        <ty::Array<u8> as ty::Marker>::write_signature(&mut self.signature);
        self.buf.write_array().write_slice(data);
    }

    /// Write a struct into the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    /// use tokio_dbus::ty;
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// buf.store(10u8);
    ///
    /// buf.write_struct::<(u16, u32, ty::Array<u8>, ty::Str)>()
    ///     .store(10u16)
    ///     .store(10u32)
    ///     .write_array(|w| {
    ///         w.store(1u8);
    ///         w.store(2u8);
    ///         w.store(3u8);
    ///     })
    ///     .write("Hello World")
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"y(quays)");
    /// assert_eq!(buf.get(), &[10, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 10, 0, 0, 0, 3, 0, 0, 0, 1, 2, 3, 0, 11, 0, 0, 0, 72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 0]);
    /// ```
    pub fn write_struct<E>(&mut self) -> TypedStructWriter<'_, E>
    where
        E: ty::Fields,
    {
        E::write_signature(&mut self.signature);
        TypedStructWriter::new(self.buf.write_struct())
    }
}

impl Default for BodyBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
