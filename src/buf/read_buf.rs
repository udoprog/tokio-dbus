use std::fmt;
use std::io;
use std::marker::PhantomData;
use std::mem::size_of;
use std::ptr;
use std::slice::from_raw_parts;

use crate::error::ErrorKind;
use crate::frame::Frame;
use crate::{Deserialize, Endianness, Error};

use super::{padding_to, ArrayReader, StructReader};

/// An aligned read-only buffer.
///
/// # Examples
///
/// ```
/// use tokio_dbus::buf::OwnedBuf;
///
/// let mut buf = OwnedBuf::new();
/// buf.write("foo bar");
///
/// let mut read_buf = buf.read_buf(6);
///
/// assert_eq!(read_buf.read::<u32>()?, &7u32);
/// assert_eq!(read_buf.read::<u8>()?, &b'f');
/// assert_eq!(read_buf.read::<u8>()?, &b'o');
/// assert_eq!(buf.get(), &[b'o', b' ', b'b', b'a', b'r', 0]);
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
        Self {
            data: ptr::NonNull::dangling(),
            read: 0,
            written: 0,
            endianness: Endianness::NATIVE,
            _marker: PhantomData,
        }
    }

    /// Construct a new read buffer wrapping pointed to data.
    pub(crate) fn new(
        data: ptr::NonNull<u8>,
        read: usize,
        written: usize,
        endianness: Endianness,
    ) -> Self {
        Self {
            data,
            read,
            written,
            endianness,
            _marker: PhantomData,
        }
    }

    /// Get the endianness of the buffer.
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Get a slice out of the buffer that has ben written to.
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

    /// Read a single value from the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::buf::OwnedBuf;
    ///
    /// let mut buf = OwnedBuf::new();
    /// buf.write(b"\x01\x02\x03\x04");
    ///
    /// let mut read_buf = buf.read_buf(6);
    ///
    /// assert_eq!(read_buf.read::<u32>()?, &4);
    /// assert_eq!(read_buf.read::<u8>()?, &1);
    /// assert_eq!(read_buf.read::<u8>()?, &2);
    /// assert!(read_buf.read::<u8>().is_err());
    /// assert!(read_buf.is_empty());
    ///
    /// let _ = buf.read_buf(3);
    /// assert!(buf.is_empty());
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ````
    pub fn read<T>(&mut self) -> Result<&'a T, Error>
    where
        T: ?Sized + Deserialize,
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
    /// use tokio_dbus::buf::OwnedBuf;
    ///
    /// let mut buf = OwnedBuf::new();
    /// buf.write(b"\x01\x02\x03\x04");
    ///
    /// let mut read_buf = buf.read_buf(6);
    /// assert_eq!(read_buf.read::<u32>()?, &4);
    ///
    /// let mut read_buf2 = read_buf.read_buf(2);
    /// assert_eq!(read_buf2.read::<u8>()?, &1);
    /// assert_eq!(read_buf2.read::<u8>()?, &2);
    ///
    /// assert!(read_buf.is_empty());
    /// assert!(read_buf2.is_empty());
    ///
    /// assert_eq!(buf.get(), &[3, 4, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn read_buf(&mut self, len: usize) -> ReadBuf<'a> {
        assert!(len <= self.len());
        let read = self.read;
        self.read += len;
        ReadBuf::new(self.data, read, read + len, self.endianness)
    }

    /// Read an array.
    pub fn read_array(&mut self) -> Result<ArrayReader<'a>, Error> {
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
    pub(crate) fn load<T>(&mut self) -> Result<&'a T, Error>
    where
        T: Frame,
    {
        let padding = padding_to::<T>(self.read);

        if self.read + padding + size_of::<T>() > self.written {
            return Err(Error::new(ErrorKind::BufferUnderflow));
        }

        self.read += padding;

        // SAFETY: read is guaranteed to be in bounds of the buffer.
        unsafe {
            let ptr = self.data.as_ptr().add(self.read).cast::<T>();
            self.read += size_of::<T>();
            // NB: The pointer is aligned.
            (*ptr).adjust(self.endianness);
            Ok(&*ptr)
        }
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

    /// Load a slice ending with a NUL byte, excluding the null byte.
    pub(crate) fn load_slice_nul(&mut self, len: usize) -> Result<&'a [u8], Error> {
        if self.read + len + 1 > self.written {
            return Err(Error::from(io::Error::from(io::ErrorKind::UnexpectedEof)));
        }

        // SAFETY: We just checked that the slice is available just above.
        let slice = unsafe {
            let ptr = self.data.as_ptr().add(self.read) as *const _;
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
            .field("len", &self.len())
            .field("endianness", &self.endianness)
            .finish()
    }
}
