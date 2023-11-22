use std::fmt;

use crate::error::Result;
use crate::ty;
use crate::{Endianness, Frame, Read, Signature};

use super::helpers::new_array_reader;
use super::{Aligned, ArrayReader, BodyBuf, Buf, StructReader};

/// A read-only view into a buffer.
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
    /// Construct an empty read buffer.
    pub(crate) const fn empty() -> Self {
        Self::from_raw_parts(Aligned::empty(), Endianness::NATIVE, Signature::EMPTY)
    }

    /// Construct a new read buffer wrapping pointed to data.
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
    #[inline]
    pub(crate) const fn into_raw_parts(self) -> (Aligned<'a>, Endianness, &'a Signature) {
        (self.data, self.endianness, self.signature)
    }

    /// Get the endianness of the buffer.
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Adjust endianness of read buffer.
    pub fn with_endianness(self, endianness: Endianness) -> Self {
        Self { endianness, ..self }
    }

    /// Get the signature of the buffer.
    pub fn signature(&self) -> &'a Signature {
        self.signature
    }

    /// Adjust signature of read buffer.
    pub fn with_signature(self, signature: &'a Signature) -> Self {
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

    /// Test if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the length of the slice.
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
    ///     assert_eq!(buf.load::<u8>()?, 1);
    ///     assert_eq!(buf.load::<u8>()?, 2);
    ///     assert!(buf.load::<u8>().is_err());
    ///     assert!(buf.is_empty());
    ///     Ok(())
    /// }
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ````
    pub fn read<T>(&mut self) -> Result<&'a T>
    where
        T: ?Sized + Read,
    {
        self.data.read()
    }

    /// Read `len` bytes from the buffer and make accessible through a
    /// [`Body`].
    ///
    /// # Panics
    ///
    /// This panics if `len` is larger than [`len()`].
    ///
    /// [`len()`]: Self::len
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Result, Body};
    ///
    /// fn read(buf: &mut Body<'_>) -> Result<()> {
    ///     let mut read_buf = buf.read_until(6);
    ///     assert_eq!(read_buf.load::<u32>()?, 4);
    ///
    ///     let mut read_buf2 = read_buf.read_until(2);
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
    pub fn read_until(&mut self, len: usize) -> Body<'a> {
        Body::from_raw_parts(self.data.read_until(len), self.endianness, self.signature)
    }

    /// Read an array from the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Endianness};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// let mut array = buf.write_array::<u32>()?;
    /// array.store(10u32)?;
    /// array.store(20u32)?;
    /// array.store(30u32)?;
    /// array.finish();
    ///
    /// let mut array = buf.write_array::<ty::Array<ty::Str>>()?;
    /// let mut inner = array.write_array();
    /// inner.write("foo")?;
    /// inner.write("bar")?;
    /// inner.write("baz")?;
    /// inner.finish();
    /// array.finish();
    ///
    /// assert_eq!(buf.signature(), b"auaas");
    ///
    /// let mut buf = buf.read_until_end();
    /// let mut array = buf.read_array()?;
    /// assert_eq!(array.load::<u32>()?, Some(10));
    /// assert_eq!(array.load::<u32>()?, Some(20));
    /// assert_eq!(array.load::<u32>()?, Some(30));
    /// assert_eq!(array.load::<u32>()?, None);
    ///
    /// let mut array = buf.read_array()?;
    ///
    /// let Some(mut inner) = array.read_array()? else {
    ///     panic!("Missing inner array");
    /// };
    ///
    /// assert_eq!(inner.read::<str>()?, Some("foo"));
    /// assert_eq!(inner.read::<str>()?, Some("bar"));
    /// assert_eq!(inner.read::<str>()?, Some("baz"));
    /// assert_eq!(inner.read::<str>()?, None);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn read_array<E>(&mut self) -> Result<ArrayReader<Self, E>>
    where
        E: ty::Aligned,
    {
        new_array_reader(self)
    }

    /// Read a struct from the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Endianness, Signature};
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    /// buf.store(10u8);
    ///
    /// buf.write_struct::<(u16, u32, ty::Array<u8>, ty::Str)>()?
    ///     .store(20u16)?
    ///     .store(30u32)?
    ///     .write_array(|w| {
    ///         w.store(1u8)?;
    ///         w.store(2u8)?;
    ///         w.store(3u8)?;
    ///         Ok(())
    ///     })?
    ///     .write("Hello World")?
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), Signature::new(b"y(quays)")?);
    ///
    /// let mut buf = buf.read_until_end();
    /// assert_eq!(buf.load::<u8>()?, 10u8);
    ///
    /// let mut st = buf.read_struct()?;
    /// assert_eq!(st.load::<u16>()?, 20u16);
    /// assert_eq!(st.load::<u32>()?, 30u32);
    ///
    /// let mut array = st.read_array()?;
    /// assert_eq!(array.load::<u8>()?, Some(1));
    /// assert_eq!(array.load::<u8>()?, Some(2));
    /// assert_eq!(array.load::<u8>()?, Some(3));
    /// assert_eq!(array.load::<u8>()?, None);
    ///
    /// assert_eq!(st.read::<str>()?, "Hello World");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn read_struct(&mut self) -> Result<StructReader<&mut Self>> {
        StructReader::new(self)
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

    /// Advance the read cursor by `n`.
    pub(crate) fn advance(&mut self, n: usize) -> Result<()> {
        self.data.advance(n)
    }

    /// Align the read side of the buffer.
    pub(crate) fn align<T>(&mut self) -> Result<()> {
        self.data.align::<T>()
    }

    /// Load a slice.
    pub(crate) fn load_slice(&mut self, len: usize) -> Result<&'a [u8]> {
        self.data.load_slice(len)
    }

    /// Load a slice ending with a NUL byte, excluding the null byte.
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

impl<'a, 'b> PartialEq<Body<'a>> for Body<'b> {
    #[inline]
    fn eq(&self, other: &Body<'a>) -> bool {
        self.get() == other.get() && self.endianness == other.endianness
    }
}

impl PartialEq<BodyBuf> for Body<'_> {
    #[inline]
    fn eq(&self, other: &BodyBuf) -> bool {
        self.get() == other.get() && self.endianness == other.endianness()
    }
}

impl<'a> Eq for Body<'a> {}

impl<'de> Buf<'de> for Body<'de> {
    type Reborrow<'this> = &'this mut Body<'de> where Self: 'this;
    type ReadUntil = Body<'de>;

    #[inline]
    fn reborrow(&mut self) -> Self::Reborrow<'_> {
        self
    }

    #[inline]
    fn advance(&mut self, n: usize) -> Result<()> {
        Body::advance(self, n)
    }

    #[inline]
    fn read_until(&mut self, len: usize) -> Self::ReadUntil {
        Body::read_until(self, len)
    }

    #[inline]
    fn len(&self) -> usize {
        Body::len(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        Body::is_empty(self)
    }

    #[inline]
    fn align<T>(&mut self) -> Result<()> {
        Body::align::<T>(self)
    }

    #[inline]
    fn load<T>(&mut self) -> Result<T>
    where
        T: Frame,
    {
        Body::load(self)
    }

    #[inline]
    fn read<T>(&mut self) -> Result<&'de T>
    where
        T: ?Sized + Read,
    {
        Body::read(self)
    }

    #[inline]
    fn load_slice(&mut self, len: usize) -> Result<&'de [u8]> {
        Body::load_slice(self, len)
    }

    #[inline]
    fn load_slice_nul(&mut self, len: usize) -> Result<&'de [u8]> {
        Body::load_slice_nul(self, len)
    }
}
