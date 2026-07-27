pub use self::store_array::StoreArray;
mod store_array;

pub use self::store_struct::StoreStruct;
mod store_struct;

pub use self::store_variant::StoreVariant;
mod store_variant;

pub use self::raw::{Raw, RawArray};
mod raw;

#[cfg(test)]
mod tests;

use core::fmt;

use alloc::borrow::ToOwned;

use crate::arguments::Arguments;
use crate::buf::{AlignedBuf, Alloc};
use crate::error::Result;
use crate::signature::{SignatureBuilder, SignatureError};
use crate::ty;
use crate::{Body, Endianness, Frame, Signature, SignatureBuf, Storable, Write, WriteAligned};

/// A buffer that can be used to write a body.
///
/// # Examples
///
/// ```
/// use tokio_dbus::BodyBuf;
///
/// let mut body = BodyBuf::new();
///
/// body.store(10u16)?;
/// body.store(10u32)?;
///
/// assert_eq!(body.signature(), "qu");
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
    /// use tokio_dbus::BodyBuf;
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u16)?;
    /// body.store(10u32)?;
    ///
    /// assert_eq!(body.signature(), "qu");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn new() -> Self {
        Self::with_endianness(Endianness::NATIVE)
    }

    /// Construct a body buffer from its raw parts.
    pub(crate) fn from_raw_parts(
        buf: AlignedBuf,
        endianness: Endianness,
        signature: SignatureBuf,
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

    /// Clear the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::BodyBuf;
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u16)?;
    /// body.store(10u32)?;
    ///
    /// assert_eq!(body.signature(), "qu");
    /// body.clear();
    /// assert_eq!(body.signature(), "");
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
    /// use tokio_dbus::BodyBuf;
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u16)?;
    /// body.store(10u32)?;
    ///
    /// assert_eq!(body.signature(), "qu");
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
    /// let body = BodyBuf::new();
    /// assert_eq!(body.endianness(), Endianness::NATIVE);
    ///
    /// let body = BodyBuf::with_endianness(Endianness::BIG);
    /// assert_eq!(body.endianness(), Endianness::BIG);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Test if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut body = BodyBuf::with_endianness(Endianness::LITTLE);
    /// assert!(body.is_empty());
    ///
    /// body.store(10u16)?;
    /// body.store(10u32)?;
    ///
    /// assert!(!body.is_empty());
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Remaining data to be read from the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut body = BodyBuf::with_endianness(Endianness::LITTLE);
    /// assert!(body.is_empty());
    ///
    /// body.store(10u16)?;
    /// body.store(10u32)?;
    ///
    /// assert_eq!(body.len(), 8);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Align the buffer to the alignment of the given type `T`.
    #[inline]
    pub(crate) fn align_mut<T>(&mut self) {
        self.buf.align_mut::<T>();
    }

    /// Align the buffer to a dynamically determined alignment.
    #[inline]
    pub(crate) fn align_mut_to(&mut self, align: usize) {
        self.buf.align_mut_to(align);
    }

    /// Get a slice out of the buffer that has ben written to.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut body = BodyBuf::with_endianness(Endianness::LITTLE);
    ///
    /// body.store(10u16)?;
    /// body.store(10u32)?;
    ///
    /// assert_eq!(body.signature(), "qu");
    /// assert_eq!(body.get(), &[10, 0, 0, 0, 10, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn get(&self) -> &[u8] {
        self.buf.get()
    }

    /// Access a [`Body`] over the entire contents of the buffer.
    ///
    /// This is a reader-like abstraction that has a read cursor and endianness,
    /// allowing convenient read access over the contents of the buffer.
    ///
    /// It is also used in combination with [`Message::with_body`] to set the
    /// message of a body.
    ///
    /// [`Message::with_body`]: crate::Message::with_body
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    ///
    /// buf.store_struct::<(u16, u32)>()?
    ///     .store(20u16)
    ///     .store(30u32)
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), "(qu)");
    ///
    /// let mut buf = buf.as_body();
    ///
    /// let (a, b) = buf.load_struct::<(u16, u32)>()?;
    /// assert_eq!(a, 20u16);
    /// assert_eq!(b, 30u32);
    ///
    /// assert!(buf.is_empty());
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn as_body(&self) -> Body<'_> {
        let data = self.buf.as_aligned();
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

    /// Store a [`Frame`] of type `T` in the buffer and add its signature.
    ///
    /// This both allocates enough space for the frame and ensures that the
    /// buffer is aligned per the requirements of the frame.    /// Write a type to the buffer and update the buffer's signature to indicate
    /// that the type `T` is stored.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, MessageKind, ObjectPath, SendBuf};
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
    /// assert_eq!(m.signature(), "du");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    ///
    /// Write unsized types:
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, MessageKind, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    /// let mut body = BodyBuf::new();
    ///
    /// body.store("Hello World!")?;
    /// body.store(PATH)?;
    ///
    /// let m = send.method_call(PATH, "Hello")
    ///     .with_body(&body);
    ///
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    /// assert_eq!(m.signature(), "so");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn store<T>(&mut self, frame: T) -> Result<()>
    where
        T: Storable,
    {
        if !T::write_signature(&mut self.signature) {
            return Err(SignatureError::too_long().into());
        }

        frame.store_to(self);
        Ok(())
    }

    /// Only store the specified value without appending its signature.
    pub(crate) fn store_frame<T>(&mut self, mut frame: T)
    where
        T: Frame,
    {
        frame.adjust(self.endianness);
        self.buf.store(frame);
    }

    /// Extend the buffer with a slice.
    pub(crate) fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Extend the buffer with a slice ending with a NUL byte.
    pub(crate) fn extend_from_slice_nul(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice_nul(bytes);
    }

    /// Only write to the buffer without appending a signature.
    pub(crate) fn write_only<T>(&mut self, value: &T)
    where
        T: ?Sized + Write,
    {
        value.write_to(self);
    }

    /// Extend the body with multiple arguments.
    ///
    /// This can be a more convenient variant compared with subsequent calls to
    /// type-dependent calls to [`BodyBuf::store`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, MessageKind, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    /// let mut body = BodyBuf::new();
    ///
    /// body.arguments(("Hello World!", PATH, 10u32));
    ///
    /// let m = send.method_call(PATH, "Hello")
    ///     .with_body(&body);
    ///
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    /// assert_eq!(m.signature(), "sou");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn arguments<T>(&mut self, value: T) -> Result<()>
    where
        T: Arguments,
    {
        value.extend_to(self)
    }

    /// Write an array into the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// let mut array = buf.store_array::<u32>()?;
    /// array.store(1u32);
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
    /// let mut array = buf.store_array::<u64>()?;
    /// array.finish();
    ///
    /// assert_eq!(buf.signature(), b"at");
    /// assert_eq!(buf.get(), &[0, 0, 0, 0, 0, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn store_array<E>(&mut self) -> Result<StoreArray<'_, E>>
    where
        E: ty::Marker,
    {
        <ty::Array<E> as ty::Marker>::write_signature(&mut self.signature)?;
        // NB: We write directly onto the underlying buffer, because we've
        // already applied the correct signature.
        Ok(StoreArray::new(self))
    }

    /// Write a slice as an byte array.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// buf.write_slice(&[1, 2, 3, 4])?;
    ///
    /// assert_eq!(buf.signature(), "ay");
    /// assert_eq!(buf.get(), &[4, 0, 0, 0, 1, 2, 3, 4]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn write_slice(&mut self, data: &[u8]) -> Result<()> {
        self.store_array::<u8>()?.write_slice(data);
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
    /// buf.store_struct::<(u16, u32, ty::Array<u8>, ty::Str)>()?
    ///     .store(10u16)
    ///     .store(10u32)
    ///     .store_array(|w| {
    ///         w.store(1u8);
    ///         w.store(2u8);
    ///         w.store(3u8);
    ///     })
    ///     .store("Hello World")
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"y(quays)");
    /// assert_eq!(buf.get(), &[10, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 10, 0, 0, 0, 3, 0, 0, 0, 1, 2, 3, 0, 11, 0, 0, 0, 72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn store_struct<E>(&mut self) -> Result<StoreStruct<'_, E>>
    where
        E: ty::Fields,
    {
        E::write_signature(&mut self.signature)?;
        // NB: We write directly onto the underlying buffer, because we've
        // already applied the correct signature.
        Ok(StoreStruct::new(self))
    }

    /// Write a variant containing a value of the given signature into the
    /// buffer.
    ///
    /// The signature of the contained value is provided at runtime, which is
    /// what makes it possible to write recursive types.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature, Variant};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// buf.store_variant(Signature::UINT32)?.store(42u32);
    ///
    /// assert_eq!(buf.signature(), Signature::VARIANT);
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.read_variant()?, Variant::U32(42));
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    ///
    /// Containers can be written into the variant as well:
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Signature};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// buf.store_variant(Signature::new("(iiay)")?)?
    ///     .store_struct::<(i32, i32, ty::Array<u8>)>()
    ///     .store(2i32)
    ///     .store(2i32)
    ///     .store_array(|w| w.write_slice(&[0xff; 16]))
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), Signature::VARIANT);
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.skip_variant()?, Signature::new("(iiay)")?);
    /// assert!(buf.is_empty());
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn store_variant(&mut self, signature: &Signature) -> Result<StoreVariant<'_>> {
        if !self.signature.extend_from_signature(Signature::VARIANT) {
            return Err(SignatureError::too_long().into());
        }

        Ok(StoreVariant::new(self, signature))
    }

    /// Extend the signature of the buffer with `signature`, and return a writer
    /// for a value matching it whose shape does not have to be known when the
    /// code is written.
    ///
    /// This is the entry point used by code which is generic over, or generated
    /// for, arbitrary D-Bus types. Prefer [`store()`] and the typed container
    /// writers when the shape of the value is known.
    ///
    /// [`store()`]: Self::store
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, Alignment, BodyBuf, Signature};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// let mut raw = buf.store_raw(Signature::new("as")?)?;
    /// let mut array = raw.store_array(Alignment::U32);
    /// array.as_raw().store("Hello");
    /// array.as_raw().store("World");
    /// array.finish();
    ///
    /// assert_eq!(buf.signature(), "as");
    ///
    /// let mut buf = buf.as_body();
    /// let mut array = buf.load_array::<ty::Str>()?;
    /// assert_eq!(array.read()?, Some("Hello"));
    /// assert_eq!(array.read()?, Some("World"));
    /// assert_eq!(array.read()?, None);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn store_raw(&mut self, signature: &Signature) -> Result<Raw<'_>> {
        self.extend_signature(signature)?;
        Ok(Raw::new(self))
    }

    /// Extend the signature of the buffer without writing anything.
    ///
    /// This is used together with [`raw()`] by code which writes several values
    /// into the same buffer and declares their combined signature up front, such
    /// as the argument list of a message.
    ///
    /// [`raw()`]: Self::raw
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut buf = BodyBuf::new();
    /// buf.extend_signature(Signature::new("us")?)?;
    ///
    /// buf.raw().store(42u32);
    /// buf.raw().store("Hello World!");
    ///
    /// assert_eq!(buf.signature(), "us");
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.load::<u32>()?, 42);
    /// assert_eq!(buf.read::<str>()?, "Hello World!");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn extend_signature(&mut self, signature: &Signature) -> Result<()> {
        if !self.signature.extend_from_signature(signature) {
            return Err(SignatureError::too_long().into());
        }

        Ok(())
    }

    /// A writer which writes values without touching the signature of the
    /// buffer.
    ///
    /// See [`extend_signature()`].
    ///
    /// [`extend_signature()`]: Self::extend_signature
    #[inline]
    pub fn raw(&mut self) -> Raw<'_> {
        Raw::new(self)
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

impl WriteAligned for BodyBuf {
    /// Only write to the buffer without appending a signature.
    #[inline]
    fn write_only<T>(&mut self, value: &T)
    where
        T: ?Sized + Write,
    {
        BodyBuf::write_only(self, value);
    }

    #[inline]
    fn store<T>(&mut self, frame: T) -> Result<()>
    where
        T: Storable,
    {
        BodyBuf::store(self, frame)
    }

    #[inline]
    fn store_frame<T>(&mut self, frame: T)
    where
        T: Frame,
    {
        BodyBuf::store_frame(self, frame);
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
