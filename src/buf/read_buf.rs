use std::fmt;
use std::io;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ptr;
use std::slice::from_raw_parts;

use crate::error::{ErrorKind, Result};
use crate::{Endianness, Error, Frame, Read};

use super::{padding_to, ArrayReader, StructReader};

/// A read-only view into a buffer.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Result, ReadBuf};
///
/// fn read(buf: &mut ReadBuf<'_>) -> Result<()> {
///     assert_eq!(buf.load::<u32>()?, 7u32);
///     assert_eq!(buf.load::<u8>()?, b'f');
///     assert_eq!(buf.load::<u8>()?, b'o');
///     assert_eq!(buf.get(), &[b'o', b' ', b'b', b'a', b'r', 0]);
///     Ok(())
/// }
/// # read(&mut ReadBuf::from_slice_le(b"\x07\x00\x00\x00foo bar\x00"))?;
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
pub struct ReadBuf<'a> {
    data: ptr::NonNull<u8>,
    read: usize,
    written: usize,
    endianness: Endianness,
    _marker: PhantomData<&'a [u8]>,
}

impl<'a> ReadBuf<'a> {
    /// Construct an empty read buffer.
    pub(crate) const fn empty() -> Self {
        Self::new(ptr::NonNull::dangling(), 0, Endianness::NATIVE)
    }

    /// Construct a read buffer from a slice.
    #[doc(hidden)]
    #[inline]
    pub const fn from_slice_le(data: &'a [u8]) -> Self {
        Self::from_slice(data, Endianness::LITTLE)
    }

    /// Construct a read buffer from a slice.
    #[inline]
    pub(crate) const fn from_slice(data: &'a [u8], endianness: Endianness) -> Self {
        // SAFETY: data is taken directly from a slice, so it's guaranteed
        // to be non-null.
        Self::new(
            unsafe { ptr::NonNull::new_unchecked(data.as_ptr() as *mut _) },
            data.len(),
            endianness,
        )
    }

    /// Construct a new read buffer wrapping pointed to data.
    pub(crate) const fn new(
        data: ptr::NonNull<u8>,
        written: usize,
        endianness: Endianness,
    ) -> Self {
        Self {
            data,
            read: 0,
            written,
            endianness,
            _marker: PhantomData,
        }
    }

    /// Get the endianness of the buffer.
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Adjust endianness of read buffer.
    pub fn with_endianness(self, endianness: Endianness) -> Self {
        Self { endianness, ..self }
    }

    /// Get a slice out of the buffer that has ben written to.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Result, ReadBuf};
    ///
    /// fn read(buf: &mut ReadBuf<'_>) -> Result<()> {
    ///     assert_eq!(buf.load::<u32>()?, 7u32);
    ///     assert_eq!(buf.load::<u8>()?, b'f');
    ///     assert_eq!(buf.load::<u8>()?, b'o');
    ///     assert_eq!(buf.get(), &[b'o', b' ', b'b', b'a', b'r', 0]);
    ///     Ok(())
    /// }
    /// # read(&mut ReadBuf::from_slice_le(b"\x07\x00\x00\x00foo bar\x00"))?;
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
    /// use tokio_dbus::{Result, ReadBuf};
    ///
    /// fn read(buf: &mut ReadBuf<'_>) -> Result<()> {
    ///     assert_eq!(buf.load::<u32>()?, 4);
    ///     assert_eq!(buf.load::<u8>()?, 1);
    ///     assert_eq!(buf.load::<u8>()?, 2);
    ///     assert!(buf.load::<u8>().is_err());
    ///     assert!(buf.is_empty());
    ///     Ok(())
    /// }
    /// # read(&mut ReadBuf::from_slice_le(b"\x04\x00\x00\x00\x01\x02"))?;
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ````
    pub fn read<T>(&mut self) -> Result<&'a T>
    where
        T: ?Sized + Read,
    {
        T::read_from(self)
    }

    /// Read `len` bytes from the buffer and make accessible through a
    /// [`ReadBuf`].
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
    /// use tokio_dbus::{Result, ReadBuf};
    ///
    /// fn read(buf: &mut ReadBuf<'_>) -> Result<()> {
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
    pub fn read_until(&mut self, len: usize) -> ReadBuf<'a> {
        assert!(len <= self.len());
        let data = unsafe { ptr::NonNull::new_unchecked(self.data.as_ptr().add(self.read)) };
        self.read += len;
        ReadBuf::new(data, len, self.endianness)
    }

    /// Read an array.
    pub fn read_array(&mut self) -> Result<ArrayReader<'a>> {
        ArrayReader::new(self)
    }

    /// Read the contents of a struct.
    pub fn read_struct(&mut self) -> StructReader<'_, 'a> {
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
    /// use tokio_dbus::{Result, ReadBuf};
    ///
    /// fn read(buf: &mut ReadBuf<'_>) -> Result<()> {
    ///     assert_eq!(buf.load::<u32>()?, 7u32);
    ///     assert_eq!(buf.load::<u8>()?, b'f');
    ///     assert_eq!(buf.load::<u8>()?, b'o');
    ///     assert_eq!(buf.get(), &[b'o', b' ', b'b', b'a', b'r', 0]);
    ///     Ok(())
    /// }
    /// # read(&mut ReadBuf::from_slice_le(b"\x07\x00\x00\x00foo bar\x00"))?;
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
        let mut frame =
            unsafe { ptr::read_unaligned(self.data.as_ptr().add(self.read).cast::<T>()) };

        self.read += size_of::<T>();
        frame.adjust(self.endianness);
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
    pub(crate) fn align<T>(&mut self) {
        let padding = padding_to::<T>(self.read);

        assert!(
            self.read + padding <= self.written,
            "{} + {padding} overflows buffer",
            self.read
        );

        self.read += padding;
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

impl Clone for ReadBuf<'_> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            data: self.data,
            read: self.read,
            written: self.written,
            endianness: self.endianness,
            _marker: self._marker,
        }
    }
}

impl fmt::Debug for ReadBuf<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadBuf")
            .field("len", &self.written)
            .field("endianness", &self.endianness)
            .finish()
    }
}

impl<'a, 'b> PartialEq<ReadBuf<'a>> for ReadBuf<'b> {
    #[inline]
    fn eq(&self, other: &ReadBuf<'a>) -> bool {
        self.get() == other.get() && self.endianness == other.endianness
    }
}

impl<'a> Eq for ReadBuf<'a> {}
