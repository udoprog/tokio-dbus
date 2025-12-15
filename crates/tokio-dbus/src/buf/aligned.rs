use core::fmt;
use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr;
use core::slice::from_raw_parts;

use crate::error::{ErrorKind, Result};
use crate::{Error, Frame};

#[cfg(feature = "alloc")]
use super::AlignedBuf;
use super::padding_to;

/// A read-only view into an aligned buffer.
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
    pub(crate) fn get(&self) -> &'a [u8] {
        unsafe {
            let at = self.data.as_ptr().add(self.read);
            from_raw_parts(at, self.len())
        }
    }

    /// Test if the slice is empty.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.read == self.written
    }

    /// Get the length of the slice.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.written - self.read
    }

    /// Read `len` bytes from the buffer and make accessible through another
    /// [`Aligned`] instance constituting that sub-slice.
    pub(crate) fn read_until(&mut self, n: usize) -> Aligned<'a> {
        assert!(n <= self.len(), "requested: {n} > length: {}", self.len());
        let data = unsafe { ptr::NonNull::new_unchecked(self.data.as_ptr().add(self.read)) };
        self.read += n;
        Aligned::new(data, n)
    }

    /// Load a frame of the given type.
    pub(crate) fn load<T>(&mut self) -> Result<T>
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
    #[cfg(feature = "alloc")]
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
            return Err(Error::from(ErrorKind::BufferUnderflow));
        }

        self.read += padding;
        Ok(())
    }

    /// Load a slice.
    pub(crate) fn load_slice(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.read + len > self.written {
            return Err(Error::from(ErrorKind::BufferUnderflow));
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
            return Err(Error::from(ErrorKind::BufferUnderflow));
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

impl<'a> PartialEq<Aligned<'a>> for Aligned<'_> {
    #[inline]
    fn eq(&self, other: &Aligned<'a>) -> bool {
        self.get() == other.get()
    }
}

#[cfg(feature = "alloc")]
impl PartialEq<AlignedBuf> for Aligned<'_> {
    #[inline]
    fn eq(&self, other: &AlignedBuf) -> bool {
        self.get() == other.get()
    }
}

impl Eq for Aligned<'_> {}
