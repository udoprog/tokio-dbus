use std::fmt;
use std::io;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ptr;
use std::slice::from_raw_parts;

use crate::error::{ErrorKind, Result};
use crate::ty;
use crate::{Error, Frame, Read};

use super::helpers::new_array_reader;
use super::{padding_to, AlignedBuf, ArrayReader, Buf, StructReader};

/// A read-only view into a buffer.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Result, Aligned};
///
/// fn read(buf: &mut Aligned<'_>) -> Result<()> {
///     assert_eq!(buf.load::<u32>()?, 7u32);
///     assert_eq!(buf.load::<u8>()?, b'f');
///     assert_eq!(buf.load::<u8>()?, b'o');
///     assert_eq!(buf.get(), &[b'o', b' ', b'b', b'a', b'r', 0]);
///     Ok(())
/// }
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
pub struct Aligned<'a> {
    data: ptr::NonNull<u8>,
    read: usize,
    written: usize,
    _marker: PhantomData<&'a [u8]>,
}

impl<'a> Aligned<'a> {
    /// Construct an empty read buffer.
    pub(crate) const fn empty() -> Self {
        Self::new(ptr::NonNull::<u64>::dangling().cast(), 0)
    }

    /// Construct a new read buffer wrapping pointed to data.
    pub(crate) const fn new(data: ptr::NonNull<u8>, written: usize) -> Self {
        Self {
            data,
            read: 0,
            written,
            _marker: PhantomData,
        }
    }

    /// Get a slice out of the buffer that has ben written to.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Result, Aligned};
    ///
    /// fn read(buf: &mut Aligned<'_>) -> Result<()> {
    ///     assert_eq!(buf.load::<u32>()?, 7u32);
    ///     assert_eq!(buf.load::<u8>()?, b'f');
    ///     assert_eq!(buf.load::<u8>()?, b'o');
    ///     assert_eq!(buf.get(), &[b'o', b' ', b'b', b'a', b'r', 0]);
    ///     Ok(())
    /// }
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn get(&self) -> &'a [u8] {
        unsafe {
            let at = self.data.as_ptr().add(self.read);
            from_raw_parts(at, self.len())
        }
    }

    /// Test if the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.read == self.written
    }

    /// Get the length of the slice.
    pub fn len(&self) -> usize {
        self.written - self.read
    }

    /// Read a reference from the buffer.
    ///
    /// This is possible for unaligned types such as `str` and `[u8]` which
    /// implement [`Read`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Result, Aligned};
    ///
    /// fn read(buf: &mut Aligned<'_>) -> Result<()> {
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
        T::read_from(self)
    }

    /// Read `len` bytes from the buffer and make accessible through a
    /// [`Aligned`].
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
    /// use tokio_dbus::{Result, Aligned};
    ///
    /// fn read(buf: &mut Aligned<'_>) -> Result<()> {
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
    pub fn read_until(&mut self, n: usize) -> Aligned<'a> {
        assert!(n <= self.len(), "requested: {n} > length: {}", self.len());
        let data = unsafe { ptr::NonNull::new_unchecked(self.data.as_ptr().add(self.read)) };
        self.read += n;
        Aligned::new(data, n)
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
    /// let mut array = buf.read_array::<u32>()?;
    /// assert_eq!(array.load()?, Some(10));
    /// assert_eq!(array.load()?, Some(20));
    /// assert_eq!(array.load()?, Some(30));
    /// assert_eq!(array.load()?, None);
    ///
    /// let mut array = buf.read_array::<ty::Array<ty::Str>>()?;
    ///
    /// let Some(mut inner) = array.read_array()? else {
    ///     panic!("Missing inner array");
    /// };
    ///
    /// assert_eq!(inner.read()?, Some("foo"));
    /// assert_eq!(inner.read()?, Some("bar"));
    /// assert_eq!(inner.read()?, Some("baz"));
    /// assert_eq!(inner.read()?, None);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn read_array<E>(&mut self) -> Result<ArrayReader<Self, E>>
    where
        E: ty::Marker,
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
    /// let mut buf = buf.peek();
    /// assert_eq!(buf.load::<u8>()?, 10u8);
    ///
    /// let mut st = buf.read_struct()?;
    /// assert_eq!(st.load::<u16>()?, 20u16);
    /// assert_eq!(st.load::<u32>()?, 30u32);
    ///
    /// let mut array = st.read_array::<u8>()?;
    /// assert_eq!(array.load()?, Some(1));
    /// assert_eq!(array.load()?, Some(2));
    /// assert_eq!(array.load()?, Some(3));
    /// assert_eq!(array.load()?, None);
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
    /// use tokio_dbus::{Result, Aligned};
    ///
    /// fn read(buf: &mut Aligned<'_>) -> Result<()> {
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
        let padding = padding_to::<T>(self.read);

        if self.read + padding + size_of::<T>() > self.written {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        self.read += padding;

        // SAFETY: read is guaranteed to be in bounds of the buffer.
        let frame = unsafe { ptr::read(self.data.as_ptr().add(self.read).cast::<T>()) };
        self.read += size_of::<T>();
        Ok(frame)
    }

    /// Advance the read cursor by `n`.
    pub(crate) fn advance(&mut self, n: usize) -> Result<()> {
        if n == 0 {
            return Ok(());
        }

        if self.read + n > self.written {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        self.read += n;
        Ok(())
    }

    /// Align the read side of the buffer.
    pub(crate) fn align<T>(&mut self) -> Result<()> {
        let padding = padding_to::<T>(self.read);

        if self.read + padding > self.written {
            return Err(Error::from(io::Error::from(io::ErrorKind::UnexpectedEof)));
        }

        self.read += padding;
        Ok(())
    }

    /// Load a slice.
    pub(crate) fn load_slice(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.read + len > self.written {
            return Err(Error::from(io::Error::from(io::ErrorKind::UnexpectedEof)));
        }

        // SAFETY: We just checked that the slice is available just above.
        let slice = unsafe {
            let ptr = self.data.as_ptr().add(self.read);
            from_raw_parts(ptr, len)
        };

        self.read += len;
        Ok(slice)
    }

    /// Load a slice ending with a NUL byte, excluding the null byte.
    pub(crate) fn load_slice_nul(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.read + len + 1 > self.written {
            return Err(Error::from(io::Error::from(io::ErrorKind::UnexpectedEof)));
        }

        // SAFETY: We just checked that the slice is available just above.
        let slice = unsafe {
            let ptr = self.data.as_ptr().add(self.read);

            if ptr.add(len).read() != 0 {
                return Err(Error::new(ErrorKind::NotNullTerminated));
            }

            from_raw_parts(ptr, len)
        };

        self.read += len + 1;
        Ok(slice)
    }
}

// SAFETY: Aligned is equivalent to `&[u8]`.
unsafe impl Send for Aligned<'_> {}
// SAFETY: Aligned is equivalent to `&[u8]`.
unsafe impl Sync for Aligned<'_> {}

impl Clone for Aligned<'_> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            read: self.read,
            written: self.written,
            _marker: self._marker,
        }
    }
}

impl fmt::Debug for Aligned<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Aligned").field("len", &self.len()).finish()
    }
}

impl<'a, 'b> PartialEq<Aligned<'a>> for Aligned<'b> {
    #[inline]
    fn eq(&self, other: &Aligned<'a>) -> bool {
        self.get() == other.get()
    }
}

impl PartialEq<AlignedBuf> for Aligned<'_> {
    #[inline]
    fn eq(&self, other: &AlignedBuf) -> bool {
        self.get() == other.get()
    }
}

impl<'a> Eq for Aligned<'a> {}

impl<'de> Buf<'de> for Aligned<'de> {
    type Reborrow<'this> = &'this mut Aligned<'de> where Self: 'this;
    type ReadUntil = Aligned<'de>;

    #[inline]
    fn reborrow(&mut self) -> Self::Reborrow<'_> {
        self
    }

    #[inline]
    fn advance(&mut self, n: usize) -> Result<()> {
        Aligned::advance(self, n)
    }

    #[inline]
    fn read_until(&mut self, len: usize) -> Self::ReadUntil {
        Aligned::read_until(self, len)
    }

    #[inline]
    fn len(&self) -> usize {
        Aligned::len(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        Aligned::is_empty(self)
    }

    #[inline]
    fn align<T>(&mut self) -> Result<()> {
        Aligned::align::<T>(self)
    }

    #[inline]
    fn load<T>(&mut self) -> Result<T>
    where
        T: Frame,
    {
        Aligned::load(self)
    }

    #[inline]
    fn read<T>(&mut self) -> Result<&'de T>
    where
        T: ?Sized + Read,
    {
        Aligned::read(self)
    }

    #[inline]
    fn load_slice(&mut self, len: usize) -> Result<&'de [u8]> {
        Aligned::load_slice(self, len)
    }

    #[inline]
    fn load_slice_nul(&mut self, len: usize) -> Result<&'de [u8]> {
        Aligned::load_slice_nul(self, len)
    }
}
