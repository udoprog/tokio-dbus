use std::fmt;

use crate::buf::AlignedBuf;
use crate::error::Result;
use crate::signature::{SignatureBuilder, SignatureError, SignatureErrorKind};
use crate::{ty, Endianness, Frame, OwnedSignature, Signature, Write};

use crate::arguments::{Arguments, ExtendBuf};

use super::helpers::{ArrayWriter, StructWriter, TypedArrayWriter, TypedStructWriter};
use super::{Alloc, Body, BufMut};

/// A buffer that can be used to write a body.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};
///
/// let mut body = BodyBuf::new();
///
/// body.store(10u16)?;
/// body.store(10u32)?;
///
/// assert_eq!(body.signature(), Signature::new(b"qu")?);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct BodyBuf {
    buf: AlignedBuf,
    endianness: Endianness,
    signature: SignatureBuilder,
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
    /// body.store(10u16)?;
    /// body.store(10u32)?;
    ///
    /// assert_eq!(body.signature(), Signature::new(b"qu")?);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn new() -> Self {
        Self::with_endianness(Endianness::NATIVE)
    }

    /// Construct a body buffer from its raw parts.
    pub(crate) fn from_raw_parts(
        buf: AlignedBuf,
        endianness: Endianness,
        signature: OwnedSignature,
    ) -> Self {
        Self {
            buf,
            endianness,
            signature: SignatureBuilder::from_owned_signature(signature),
        }
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
            signature: SignatureBuilder::new(),
            endianness,
            buf: AlignedBuf::new(),
        }
    }

    /// Access the raw interior aligned buf.
    ///
    /// Note that operations over this buffer will not take endianness into
    /// account.
    #[inline]
    pub(crate) fn buf_mut(&mut self) -> &mut AlignedBuf {
        &mut self.buf
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
    /// body.store(10u16)?;
    /// body.store(10u32)?;
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

    /// Get the signature of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u16)?;
    /// body.store(10u32)?;
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
        self.endianness
    }

    /// Test if the buffer is empty.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Remaining data to be read from the buffer.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.buf.len()
    }

    /// Align the buffer to the alignment of the given type `T`.
    #[inline]
    pub(crate) fn align_mut<T>(&mut self) {
        self.buf.align_mut::<T>();
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
    /// body.store(10u16)?;
    /// body.store(10u32)?;
    ///
    /// assert_eq!(body.signature(), Signature::new(b"qu")?);
    /// assert_eq!(body.get(), &[10, 0, 0, 0, 10, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn get(&self) -> &[u8] {
        self.buf.get()
    }

    /// Read `len` bytes from the buffer and make accessible through a [`Body`]
    /// instance.
    ///
    /// # Panics
    ///
    /// This panics if `len` is larger than [`len()`].
    ///
    /// [`len()`]: Self::len
    #[inline]
    pub(crate) fn read_until(&mut self, len: usize) -> Body<'_> {
        let data = self.buf.read_until(len);
        Body::from_raw_parts(data, self.endianness, &self.signature)
    }

    /// Read the whole buffer until its end.
    #[inline]
    pub fn read_until_end(&mut self) -> Body<'_> {
        let data = self.buf.read_until_end();
        Body::from_raw_parts(data, self.endianness, &self.signature)
    }

    /// Access a read buf which peeks into the buffer without advancing it.
    #[inline]
    pub fn peek(&self) -> Body<'_> {
        let data = self.buf.peek();
        Body::from_raw_parts(data, self.endianness, &self.signature)
    }

    /// Allocate, zero space for and align data for `T`.
    #[inline]
    pub(crate) fn alloc<T>(&mut self) -> Alloc<T>
    where
        T: Frame,
    {
        self.buf.alloc()
    }

    /// Write the given value at the previously [`Alloc<T>`] position.
    #[inline]
    pub(crate) fn store_at<T>(&mut self, at: Alloc<T>, mut frame: T)
    where
        T: Frame,
    {
        frame.adjust(self.endianness);
        self.buf.store_at(at, frame);
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
    #[inline]
    pub fn set_endianness(&mut self, endianness: Endianness) {
        self.endianness = endianness;
    }

    /// Store a [`Frame`] of type `T` in the buffer and add its signature.
    ///
    /// This both allocates enough space for the frame and ensures that the
    /// buffer is aligned per the requirements of the frame.    /// Write a type to the buffer and update the buffer's signature to indicate
    /// that the type `T` is stored.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{BodyBuf, Message, MessageKind, ObjectPath, SendBuf, Signature};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10f64)?;
    /// body.store(20u32)?;
    ///
    /// let m = send.method_call(PATH, "Hello")
    ///     .with_body(&body);
    ///
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    /// assert_eq!(m.signature(), Signature::new(b"du")?);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn store<T>(&mut self, mut frame: T) -> Result<()>
    where
        T: Frame,
    {
        if !self.signature.extend_from_signature(T::SIGNATURE) {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong).into());
        }

        frame.adjust(self.endianness);
        self.buf.store(frame);
        Ok(())
    }

    /// Extend the buffer with a slice.
    pub(crate) fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Extend the buffer with a slice ending with a NUL byte.
    pub(crate) fn extend_from_slice_nul(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice_nul(bytes);
    }

    /// Write a type `T` to the buffer and and its signature.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{BodyBuf, Message, MessageKind, ObjectPath, SendBuf, Signature};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    /// let mut body = BodyBuf::new();
    ///
    /// body.write("Hello World!")?;
    /// body.write(PATH)?;
    ///
    /// let m = send.method_call(PATH, "Hello")
    ///     .with_body(&body);
    ///
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    /// assert_eq!(m.signature(), Signature::new(b"so")?);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn write<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Write,
    {
        if !self.signature.extend_from_signature(T::SIGNATURE) {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong).into());
        }

        self.buf.write(value)?;
        Ok(())
    }

    /// Extend the body with multiple arguments.
    ///
    /// This can be a more convenient variant compared with subsequent calls to
    /// type-dependent calls to [`BodyBuf::store`] or [`BodyBuf::write`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{BodyBuf, Message, MessageKind, ObjectPath, SendBuf, Signature};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    /// let mut body = BodyBuf::new();
    ///
    /// body.extend(("Hello World!", PATH, 10u32));
    ///
    /// let m = send.method_call(PATH, "Hello")
    ///     .with_body(&body);
    ///
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    /// assert_eq!(m.signature(), Signature::new(b"sou")?);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn extend<T>(&mut self, value: T) -> Result<()>
    where
        T: Arguments,
    {
        value.extend_to(self)
    }

    /// Write a raw array into the buffer.
    pub fn write_array_raw<A>(&mut self) -> ArrayWriter<'_, Self, A>
    where
        A: ty::Aligned,
    {
        ArrayWriter::new(self)
    }

    /// Write an array into the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// let mut array = buf.write_array::<u32>()?;
    /// array.store(1u32)?;
    /// array.finish();
    ///
    /// assert_eq!(buf.signature(), b"au");
    /// assert_eq!(buf.get(), &[4, 0, 0, 0, 1, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    ///
    /// Writing an empty array still enforces element alignment:
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// let mut array = buf.write_array::<u64>()?;
    /// array.finish();
    ///
    /// assert_eq!(buf.signature(), b"at");
    /// assert_eq!(buf.get(), &[0, 0, 0, 0, 0, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn write_array<E>(&mut self) -> Result<TypedArrayWriter<'_, AlignedBuf, E>>
    where
        E: ty::Marker,
    {
        <ty::Array<E> as ty::Marker>::write_signature(&mut self.signature)?;
        // NB: We write directly onto the underlying buffer, because we've
        // already applied the correct signature.
        Ok(TypedArrayWriter::new(ArrayWriter::new(&mut self.buf)))
    }

    /// Write a slice as an byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness, Signature};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// buf.write_slice(&[1, 2, 3, 4])?;
    ///
    /// assert_eq!(buf.signature(), Signature::new(b"ay")?);
    /// assert_eq!(buf.get(), &[4, 0, 0, 0, 1, 2, 3, 4]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn write_slice(&mut self, data: &[u8]) -> Result<()> {
        self.write_array::<u8>()?.write_slice(data);
        Ok(())
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
    /// buf.write_struct::<(u16, u32, ty::Array<u8>, ty::Str)>()?
    ///     .store(10u16)?
    ///     .store(10u32)?
    ///     .write_array(|w| {
    ///         w.store(1u8)?;
    ///         w.store(2u8)?;
    ///         w.store(3u8)?;
    ///         Ok(())
    ///     })?
    ///     .write("Hello World")?
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"y(quays)");
    /// assert_eq!(buf.get(), &[10, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 10, 0, 0, 0, 3, 0, 0, 0, 1, 2, 3, 0, 11, 0, 0, 0, 72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn write_struct<E>(&mut self) -> Result<TypedStructWriter<'_, AlignedBuf, E>>
    where
        E: ty::Fields,
    {
        E::write_signature(&mut self.signature)?;
        // NB: We write directly onto the underlying buffer, because we've
        // already applied the correct signature.
        Ok(TypedStructWriter::new(StructWriter::new(&mut self.buf)))
    }
}

impl fmt::Debug for BodyBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BodyBuf")
            .field("buf", &self.buf)
            .field("endianness", &self.endianness)
            .field("signature", &self.signature.to_signature())
            .finish()
    }
}

impl Default for BodyBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl BufMut for BodyBuf {
    #[inline]
    fn align_mut<T>(&mut self) {
        BodyBuf::align_mut::<T>(self)
    }

    #[inline]
    fn len(&self) -> usize {
        BodyBuf::len(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        BodyBuf::is_empty(self)
    }

    #[inline]
    fn write<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Write,
    {
        BodyBuf::write(self, value)
    }

    #[inline]
    fn store<T>(&mut self, frame: T) -> Result<()>
    where
        T: Frame,
    {
        BodyBuf::store(self, frame)
    }

    #[inline]
    fn alloc<T>(&mut self) -> Alloc<T>
    where
        T: Frame,
    {
        BodyBuf::alloc(self)
    }

    #[inline]
    fn store_at<T>(&mut self, at: Alloc<T>, frame: T)
    where
        T: Frame,
    {
        BodyBuf::store_at(self, at, frame)
    }

    #[inline]
    fn extend_from_slice(&mut self, bytes: &[u8]) {
        BodyBuf::extend_from_slice(self, bytes);
    }

    #[inline]
    fn extend_from_slice_nul(&mut self, bytes: &[u8]) {
        BodyBuf::extend_from_slice_nul(self, bytes);
    }
}

impl ExtendBuf for BodyBuf {
    #[inline]
    fn write<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Write,
    {
        BodyBuf::write(self, value)
    }

    #[inline]
    fn store<T>(&mut self, frame: T) -> Result<()>
    where
        T: Frame,
    {
        BodyBuf::store(self, frame)
    }
}

/// Construct an aligned buffer from a read buffer.
impl From<Body<'_>> for BodyBuf {
    #[inline]
    fn from(buf: Body<'_>) -> Self {
        let (buf, endianness, signature) = buf.into_raw_parts();
        let buf = AlignedBuf::from(buf);
        let signature = signature.to_owned();
        Self::from_raw_parts(buf, endianness, signature)
    }
}
