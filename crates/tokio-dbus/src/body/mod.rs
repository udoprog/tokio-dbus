#[cfg(test)]
mod tests;

pub use self::load_array::LoadArray;
mod load_array;

pub use self::as_body::AsBody;
mod as_body;

use core::fmt;

#[cfg(feature = "alloc")]
use crate::BodyBuf;
use crate::buf::Aligned;
use crate::error::Result;
use crate::ty;
use crate::{Endianness, Frame, Read, Signature};

/// A read-only view into a buffer suitable for use as a body in a [`Message`].
///
/// [`Message`]: crate::Message
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Result, Body};
///
/// fn read(buf: &mut Body<'_>) -> Result<()> {
///     assert_eq!(buf.load::<u32>()?, 7u32);
///     assert_eq!(buf.load::<u8>()?, b'f');
///     assert_eq!(buf.load::<u8>()?, b'o');
///     assert_eq!(buf.get(), &[b'o', b' ', b'b', b'a', b'r', 0]);
///     Ok(())
/// }
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
pub struct Body<'a> {
    data: Aligned<'a>,
    endianness: Endianness,
    signature: &'a Signature,
}

impl<'a> Body<'a> {
    /// Construct an empty buffer.
    pub(crate) const fn empty() -> Self {
        Self::from_raw_parts(Aligned::empty(), Endianness::NATIVE, Signature::EMPTY)
    }

    /// Construct a new buffer wrapping pointed to data.
    #[inline]
    pub(crate) const fn from_raw_parts(
        data: Aligned<'a>,
        endianness: Endianness,
        signature: &'a Signature,
    ) -> Self {
        Self {
            data,
            endianness,
            signature,
        }
    }

    /// Deconstruct into raw parts.
    #[cfg(feature = "alloc")]
    #[inline]
    pub(crate) const fn into_raw_parts(self) -> (Aligned<'a>, Endianness, &'a Signature) {
        (self.data, self.endianness, self.signature)
    }

    /// Get the endianness of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Body, BodyBuf, Endianness};
    ///
    /// let buf = BodyBuf::new();
    ///
    /// let buf: Body<'_> = buf.as_body();
    /// assert_eq!(buf.endianness(), Endianness::NATIVE);
    ///
    /// let buf = buf.with_endianness(Endianness::BIG);
    /// assert_eq!(buf.endianness(), Endianness::BIG);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Adjust endianness of buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Body, BodyBuf, Endianness};
    ///
    /// let buf = BodyBuf::new();
    ///
    /// let buf: Body<'_> = buf.as_body();
    /// assert_eq!(buf.endianness(), Endianness::NATIVE);
    ///
    /// let buf = buf.with_endianness(Endianness::BIG);
    /// assert_eq!(buf.endianness(), Endianness::BIG);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn with_endianness(self, endianness: Endianness) -> Self {
        Self { endianness, ..self }
    }

    /// Get the signature of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Body, BodyBuf};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// buf.store(10u16)?;
    /// buf.store(10u32)?;
    ///
    /// let buf: Body<'_> = buf.as_body();
    ///
    /// assert_eq!(buf.signature(), "qu");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn signature(&self) -> &'a Signature {
        self.signature
    }

    /// Adjust the signature of buffer.
    #[cfg(feature = "alloc")]
    pub(crate) fn with_signature(self, signature: &'a Signature) -> Self {
        Self { signature, ..self }
    }

    /// Get a slice out of the buffer that has ben written to.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Result, Body};
    ///
    /// fn read(buf: &mut Body<'_>) -> Result<()> {
    ///     assert_eq!(buf.load::<u32>()?, 7u32);
    ///     assert_eq!(buf.load::<u8>()?, b'f');
    ///     assert_eq!(buf.load::<u8>()?, b'o');
    ///     assert_eq!(buf.get(), &[b'o', b' ', b'b', b'a', b'r', 0]);
    ///     Ok(())
    /// }
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn get(&self) -> &'a [u8] {
        self.data.get()
    }

    /// Test if the buffer is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Body, BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// let b: Body<'_> = buf.as_body();
    /// assert!(b.is_empty());
    ///
    /// buf.store(10u16)?;
    /// buf.store(10u32)?;
    ///
    /// let b: Body<'_> = buf.as_body();
    /// assert!(!b.is_empty());
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Remaining data to be read from the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Body, BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// assert!(buf.is_empty());
    ///
    /// buf.store(10u16)?;
    /// buf.store(10u32)?;
    ///
    /// let b: Body<'_> = buf.as_body();
    /// assert_eq!(b.len(), 8);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Read a reference from the buffer.
    ///
    /// This is possible for unaligned types such as `str` and `[u8]` which
    /// implement [`Read`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Result, Body};
    ///
    /// fn read(buf: &mut Body<'_>) -> Result<()> {
    ///     assert_eq!(buf.load::<u32>()?, 4);
    ///     assert_eq!(buf.read::<str>()?, "hi");
    ///     assert!(buf.is_empty());
    ///     Ok(())
    /// }
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ````
    pub fn read<T>(&mut self) -> Result<&'a T>
    where
        T: ?Sized + Read,
    {
        T::read_from(self)
    }

    /// Read `len` bytes from the buffer and make accessible through another
    /// [`Body`] instance constituting that sub-slice.
    ///
    /// # Errors
    ///
    /// Errors if `len` is larger than [`len()`].
    ///
    /// [`len()`]: Self::len
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Result, Body};
    ///
    /// fn read(buf: &mut Body<'_>) -> Result<()> {
    ///     let mut read_buf = buf.read_until(6)?;
    ///     assert_eq!(read_buf.load::<u32>()?, 4);
    ///
    ///     let mut read_buf2 = read_buf.read_until(2)?;
    ///     assert_eq!(read_buf2.load::<u8>()?, 1);
    ///     assert_eq!(read_buf2.load::<u8>()?, 2);
    ///
    ///     assert!(read_buf.is_empty());
    ///     assert!(read_buf2.is_empty());
    ///
    ///     assert_eq!(buf.get(), &[3, 4, 0]);
    ///     Ok(())
    /// }
    /// ```
    pub fn read_until(&mut self, len: usize) -> Result<Body<'a>> {
        let data = self.data.read_until(len)?;
        Ok(Body::from_raw_parts(data, self.endianness, self.signature))
    }

    /// Read an array from the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// let mut array = buf.store_array::<u32>()?;
    /// array.store(10u32);
    /// array.store(20u32);
    /// array.store(30u32);
    /// array.finish();
    ///
    /// let mut array = buf.store_array::<ty::Array<ty::Str>>()?;
    /// let mut inner = array.store_array();
    /// inner.store("foo");
    /// inner.store("bar");
    /// inner.store("baz");
    /// inner.finish();
    /// array.finish();
    ///
    /// assert_eq!(buf.signature(), b"auaas");
    ///
    /// let mut buf = buf.as_body();
    /// let mut array = buf.load_array::<u32>()?;
    /// assert_eq!(array.load()?, Some(10));
    /// assert_eq!(array.load()?, Some(20));
    /// assert_eq!(array.load()?, Some(30));
    /// assert_eq!(array.load()?, None);
    ///
    /// let mut array = buf.load_array::<ty::Array<ty::Str>>()?;
    ///
    /// let Some(mut inner) = array.load_array()? else {
    ///     panic!("Missing inner array");
    /// };
    ///
    /// assert_eq!(inner.read()?, Some("foo"));
    /// assert_eq!(inner.read()?, Some("bar"));
    /// assert_eq!(inner.read()?, Some("baz"));
    /// assert_eq!(inner.read()?, None);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn load_array<E>(&mut self) -> Result<LoadArray<'a, E>>
    where
        E: ty::Marker,
    {
        LoadArray::from_mut(self)
    }

    /// Read a struct from the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// buf.store(10u8);
    ///
    /// buf.store_struct::<(u16, u32, ty::Array<u8>, ty::Str)>()?
    ///     .store(20u16)
    ///     .store(30u32)
    ///     .store_array(|w| {
    ///         w.store(1u8);
    ///         w.store(2u8);
    ///         w.store(3u8);
    ///     })
    ///     .store("Hello World")
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), "y(quays)");
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.load::<u8>()?, 10u8);
    ///
    /// let (a, b, mut array, string) = buf.load_struct::<(u16, u32, ty::Array<u8>, ty::Str)>()?;
    /// assert_eq!(a, 20u16);
    /// assert_eq!(b, 30u32);
    ///
    /// assert_eq!(array.load()?, Some(1));
    /// assert_eq!(array.load()?, Some(2));
    /// assert_eq!(array.load()?, Some(3));
    /// assert_eq!(array.load()?, None);
    ///
    /// assert_eq!(string, "Hello World");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn load_struct<E>(&mut self) -> Result<E::Return<'a>>
    where
        E: ty::Fields,
    {
        self.align::<u64>()?;
        E::load_struct(self)
    }

    /// Read a struct whose fields are read by the given closure.
    ///
    /// This aligns the buffer as a struct and then hands it to `f`. It is an
    /// escape hatch for structs which [`load_struct()`] cannot describe, such as
    /// ones containing a variant of an unknown type.
    ///
    /// [`load_struct()`]: Self::load_struct
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Signature};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// buf.store_struct::<(u32, ty::Variant)>()?
    ///     .store(42u32)
    ///     .store_variant(Signature::new("as")?, |w| {
    ///         w.store_array::<ty::Str>().store("Hello");
    ///     })
    ///     .finish();
    ///
    /// let mut buf = buf.as_body();
    ///
    /// let n = buf.load_struct_with(|b| {
    ///     let n = b.load::<u32>()?;
    ///     b.skip_variant()?;
    ///     Ok(n)
    /// })?;
    ///
    /// assert_eq!(n, 42);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn load_struct_with<F, O>(&mut self, f: F) -> Result<O>
    where
        F: FnOnce(&mut Body<'a>) -> Result<O>,
    {
        self.align::<u64>()?;
        f(self)
    }

    /// Load a frame of the given type.
    ///
    /// This advances the read cursor of the buffer by the alignment and size of
    /// the type. The return value has been endian-adjusted as per
    /// [`endianness()`].
    ///
    /// [`endianness()`]: Self::endianness
    ///
    /// # Error
    ///
    /// Errors if the underlying buffer does not have enough space to represent
    /// the type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Result, Body};
    ///
    /// fn read(buf: &mut Body<'_>) -> Result<()> {
    ///     assert_eq!(buf.load::<u32>()?, 7u32);
    ///     assert_eq!(buf.load::<u8>()?, b'f');
    ///     assert_eq!(buf.load::<u8>()?, b'o');
    ///     assert_eq!(buf.get(), &[b'o', b' ', b'b', b'a', b'r', 0]);
    ///     Ok(())
    /// }
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn load<T>(&mut self) -> Result<T>
    where
        T: Frame,
    {
        let mut frame = self.data.load::<T>()?;
        frame.adjust(self.endianness);
        Ok(frame)
    }

    /// Load a [`bool`] from the buffer.
    ///
    /// The D-Bus `BOOLEAN` type is marshalled as a 32-bit integer, which is why
    /// it cannot be loaded through [`load()`].
    ///
    /// [`load()`]: Self::load
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::BodyBuf;
    ///
    /// let mut buf = BodyBuf::new();
    /// buf.store(true)?;
    /// buf.store(false)?;
    ///
    /// let mut buf = buf.as_body();
    /// assert!(buf.load_bool()?);
    /// assert!(!buf.load_bool()?);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn load_bool(&mut self) -> Result<bool> {
        Ok(self.load::<u32>()? != 0)
    }

    /// Read a [`Variant`] holding a value of a basic type from the buffer.
    ///
    /// [`Variant`]: crate::Variant
    ///
    /// # Errors
    ///
    /// Errors if the variant holds a container. Use [`skip_variant()`] to skip
    /// over a variant of an unknown type instead.
    ///
    /// [`skip_variant()`]: Self::skip_variant
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Variant};
    ///
    /// let mut buf = BodyBuf::new();
    /// buf.store(Variant::U32(42))?;
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.read_variant()?, Variant::U32(42));
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn read_variant(&mut self) -> Result<crate::Variant<'a>> {
        <ty::Variant as ty::Marker>::load_struct(self)
    }

    /// Read a variant which is expected to contain a value of type `T`.
    ///
    /// Unlike [`read_variant()`] this can read containers, but requires the
    /// caller to know which type the variant contains.
    ///
    /// [`read_variant()`]: Self::read_variant
    ///
    /// # Errors
    ///
    /// Errors if the variant does not contain a value of type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Signature};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// let mut array = buf.store_variant(Signature::new("as")?)?.store_array::<ty::Str>();
    /// array.store("Hello");
    /// array.store("World");
    /// array.finish();
    ///
    /// let mut buf = buf.as_body();
    /// let mut array = buf.read_variant_as::<ty::Array<ty::Str>>()?;
    ///
    /// assert_eq!(array.read()?, Some("Hello"));
    /// assert_eq!(array.read()?, Some("World"));
    /// assert_eq!(array.read()?, None);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn read_variant_as<T>(&mut self) -> Result<T::Return<'a>>
    where
        T: ty::Marker,
    {
        let signature = self.read::<Signature>()?;

        let mut expected = crate::signature::SignatureBuilder::new();
        T::write_signature(&mut expected)?;

        if signature != expected.to_signature() {
            #[cfg(feature = "alloc")]
            return Err(crate::Error::new(
                crate::error::ErrorKind::UnsupportedVariant(signature.into()),
            ));
            #[cfg(not(feature = "alloc"))]
            return Err(crate::Error::new(
                crate::error::ErrorKind::UnsupportedVariantNoAlloc,
            ));
        }

        self.align::<T::Alignment>()?;
        T::load_struct(self)
    }

    /// Skip over a variant of any type, returning the signature of the value it
    /// contained.
    ///
    /// This is useful for arguments which are declared as variants but which
    /// the receiver has no interest in, such as the `data` argument of the
    /// `com.canonical.dbusmenu.Event` method.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Signature};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// buf.store_variant(Signature::new("as")?)?
    ///     .store_array::<ty::Str>()
    ///     .store("Hello");
    /// buf.store(42u32)?;
    ///
    /// assert_eq!(buf.signature(), "vu");
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.skip_variant()?, Signature::new("as")?);
    /// assert_eq!(buf.load::<u32>()?, 42);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn skip_variant(&mut self) -> Result<&'a Signature> {
        let signature = self.read::<Signature>()?;
        crate::signature::skip(signature, self, 1)?;
        Ok(signature)
    }

    /// Align the read cursor to the given alignment.
    ///
    /// This is the counterpart of [`Raw::align`], and is needed before reading
    /// the fields of a struct or a dict entry whose shape is only known at
    /// runtime.
    ///
    /// [`Raw::align`]: crate::Raw::align
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, Alignment, BodyBuf};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// buf.store(1u8)?;
    /// buf.store_struct::<(u32, u32)>()?.store(2u32).store(3u32).finish();
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.load::<u8>()?, 1);
    ///
    /// buf.align_to(Alignment::U64)?;
    /// assert_eq!(buf.load::<u32>()?, 2);
    /// assert_eq!(buf.load::<u32>()?, 3);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn align_to(&mut self, alignment: crate::Alignment) -> Result<()> {
        self.data.align_to(alignment.in_bytes())
    }

    /// Read an array whose elements have the given alignment, returning a
    /// [`Body`] over its contents.
    ///
    /// This is the counterpart of [`Raw::store_array`].
    ///
    /// [`Raw::store_array`]: crate::Raw::store_array
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, Alignment, BodyBuf};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// let mut array = buf.store_array::<ty::Str>()?;
    /// array.store("Hello");
    /// array.store("World");
    /// array.finish();
    ///
    /// let mut buf = buf.as_body();
    /// let mut array = buf.load_raw_array(Alignment::U32)?;
    ///
    /// let mut out = Vec::new();
    ///
    /// while !array.is_empty() {
    ///     out.push(array.read::<str>()?);
    /// }
    ///
    /// assert_eq!(out, ["Hello", "World"]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn load_raw_array(&mut self, alignment: crate::Alignment) -> Result<Body<'a>> {
        let bytes = self.load::<u32>()?;

        if bytes > crate::buf::MAX_ARRAY_LENGTH {
            return Err(crate::Error::new(crate::error::ErrorKind::ArrayTooLong(
                bytes,
            )));
        }

        self.align_to(alignment)?;
        self.read_until(bytes as usize)
    }

    /// Advance the read cursor by `n`.
    #[inline]
    pub(crate) fn advance(&mut self, n: usize) -> Result<()> {
        self.data.advance(n)
    }

    /// Align the read side of the buffer.
    #[inline]
    pub(crate) fn align<T>(&mut self) -> Result<()> {
        self.data.align::<T>()
    }

    /// Load a slice.
    #[inline]
    pub(crate) fn load_slice(&mut self, len: usize) -> Result<&'a [u8]> {
        self.data.load_slice(len)
    }

    /// Load a slice ending with a NUL byte, excluding the null byte.
    #[inline]
    pub(crate) fn load_slice_nul(&mut self, len: usize) -> Result<&'a [u8]> {
        self.data.load_slice_nul(len)
    }
}

// SAFETY: Body is equivalent to `&[u8]`.
unsafe impl Send for Body<'_> {}
// SAFETY: Body is equivalent to `&[u8]`.
unsafe impl Sync for Body<'_> {}

impl Clone for Body<'_> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            endianness: self.endianness,
            signature: self.signature,
        }
    }
}

impl fmt::Debug for Body<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Body")
            .field("data", &self.data)
            .field("endianness", &self.endianness)
            .finish()
    }
}

impl<'a> PartialEq<Body<'a>> for Body<'_> {
    #[inline]
    fn eq(&self, other: &Body<'a>) -> bool {
        self.get() == other.get() && self.endianness == other.endianness
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<BodyBuf> for Body<'_> {
    #[inline]
    fn eq(&self, other: &BodyBuf) -> bool {
        self.get() == other.get() && self.endianness == other.endianness()
    }
}

impl Eq for Body<'_> {}
