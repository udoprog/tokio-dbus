use std::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};
use std::fmt;
use std::mem::{align_of, size_of};
use std::ptr;
use std::slice::{from_raw_parts, from_raw_parts_mut};

use crate::buf::{max_size_for_align, padding_to, Alloc, BufMut, ReadBuf};
use crate::{Frame, Result, Write};

/// The type we're basing our alignment on.
pub(crate) type AlignType = u64;

/// An owned buffer which is aligned per the specification of D-Bus messages.
pub struct AlignedBuf {
    /// Pointed to data of the buffer.
    data: ptr::NonNull<u8>,
    /// The initialized capacity of the buffer.
    capacity: usize,
    /// Write position in the buffer.
    written: usize,
    /// Read position in the buffer.
    read: usize,
}

impl AlignedBuf {
    /// Construct a new empty buffer.
    pub(crate) const fn new() -> Self {
        Self {
            data: ptr::NonNull::<AlignType>::dangling().cast(),
            capacity: 0,
            written: 0,
            read: 0,
        }
    }

    /// Read `len` bytes from the buffer and make accessible through a
    /// [`ReadBuf`].
    ///
    /// # Panics
    ///
    /// This panics if `len` is larger than [`len()`].
    ///
    /// [`len()`]: Self::len
    pub(crate) fn read_until(&mut self, len: usize) -> ReadBuf<'_> {
        assert!(len <= self.len());
        let data = unsafe { ptr::NonNull::new_unchecked(self.data.as_ptr().add(self.read)) };
        self.advance(len);
        ReadBuf::new(data, len)
    }

    /// Read the entire buffer and make accessible through [`ReadBuf`].
    pub(crate) fn read_until_end(&mut self) -> ReadBuf<'_> {
        let len = self.len();
        let data = unsafe { ptr::NonNull::new_unchecked(self.data.as_ptr().add(self.read)) };
        self.clear();
        ReadBuf::new(data, len)
    }

    /// Access a read buf which peeks into the buffer without advancing it.
    pub(crate) fn peek(&self) -> ReadBuf<'_> {
        let len = self.len();
        let data = unsafe { ptr::NonNull::new_unchecked(self.data.as_ptr().add(self.read)) };
        ReadBuf::new(data, len)
    }

    /// Allocate, zero space for and align data for `T`.
    pub(crate) fn alloc<T>(&mut self) -> Alloc<T>
    where
        T: Frame,
    {
        self.align_mut::<T>();
        let at = self.written;

        // SAFETY: We've just reserved and aligned the buffer above.
        unsafe {
            self.zero(size_of::<T>());
        }

        Alloc::new(at)
    }

    /// Write the given value at the previously [`Alloc<T>`] position.
    pub(crate) fn store_at<T>(&mut self, at: Alloc<T>, frame: T)
    where
        T: Frame,
    {
        let at = at.into_usize();
        assert!(at + size_of::<T>() <= self.written, "write underflow");

        // SAFETY: We've just asserted that the write is in bounds above and
        // this buffer ensures that all types that implement `Frame` are written
        // to aligned location.
        unsafe {
            self.data.as_ptr().add(at).cast::<T>().write(frame);
        }
    }

    /// Store a [`Frame`] of type `T` in the buffer.
    ///
    /// This both allocates enough space for the frame and ensures that the
    /// buffer is aligned per the requirements of the frame.
    pub(crate) fn store<T>(&mut self, frame: T)
    where
        T: Frame,
    {
        self.align_mut::<T>();

        // SAFETY: We've just reserved and aligned the buffer in the `reserve`
        // call just above.
        unsafe {
            self.data
                .as_ptr()
                .add(self.written)
                .cast::<T>()
                .write(frame);
            self.written += size_of::<T>();
        }
    }

    /// Write a type to the buffer.
    pub(crate) fn write<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Write,
    {
        value.write_to(self)
    }

    /// Extend the buffer with a slice.
    pub(crate) fn extend_from_slice(&mut self, bytes: &[u8]) {
        let requested = self.written + bytes.len();
        self.ensure_capacity(requested);

        // SAFETY: We've ensures that the necessary capacity is available.
        unsafe {
            self.data
                .as_ptr()
                .add(self.written)
                .copy_from(bytes.as_ptr(), bytes.len());
        }

        self.written += bytes.len();
    }

    /// Extend the buffer with a slice ending with a NUL byte.
    pub(crate) fn extend_from_slice_nul(&mut self, bytes: &[u8]) {
        let requested = self.written + bytes.len() + 1;
        self.ensure_capacity(requested);

        // SAFETY: We've ensures that the necessary capacity is available.
        unsafe {
            let ptr = self.data.as_ptr().add(self.written);
            ptr.copy_from(bytes.as_ptr(), bytes.len());
            ptr.add(bytes.len()).write(0u8);
        }

        self.written += bytes.len() + 1;
    }

    /// Reserve space for `bytes` additional bytes in the buffer.
    pub(crate) fn reserve_bytes(&mut self, bytes: usize) {
        let requested = self.written + bytes;
        self.ensure_capacity(requested);
    }

    /// Test if the buffer is empty.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.read == self.written
    }

    /// Remaining data to be read from the buffer.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.written - self.read
    }

    /// Get a slice out of the buffer that has ben written to.
    pub(crate) fn get(&self) -> &[u8] {
        unsafe {
            let at = self.data.as_ptr().add(self.read);
            from_raw_parts(at, self.len())
        }
    }

    /// Get remaining slice of the buffer that can be written.
    pub(crate) fn get_mut(&mut self) -> &mut [u8] {
        unsafe {
            let len = self.capacity - self.written;
            let at = self.data.as_ptr().add(self.written);
            from_raw_parts_mut(at, len)
        }
    }

    /// Indicate that we've written `n` bytes to the buffer.
    pub(crate) fn advance_mut(&mut self, n: usize) {
        self.written += n;
    }

    /// Indicate that we've read `n` bytes from the buffer.
    pub(crate) fn advance(&mut self, n: usize) {
        self.read += n;

        if self.read == self.written {
            self.clear();
        }
    }

    /// Clear the current buffer.
    pub(crate) fn clear(&mut self) {
        self.read = 0;
        self.written = 0;
    }

    /// Ensure that the buffer has at least `capacity` bytes.
    fn ensure_capacity(&mut self, capacity: usize) {
        if capacity <= self.capacity {
            return;
        }

        let capacity = 16usize.max(capacity.next_power_of_two());

        assert!(
            capacity <= max_size_for_align(align_of::<AlignType>()),
            "capacity overflow"
        );

        self.realloc(capacity);
        self.capacity = capacity;
    }

    fn realloc(&mut self, capacity: usize) {
        unsafe {
            if self.capacity == 0 {
                let layout = Layout::from_size_align_unchecked(capacity, align_of::<AlignType>());
                let ptr = alloc(layout);

                if ptr.is_null() {
                    handle_alloc_error(layout);
                }

                self.data = ptr::NonNull::new_unchecked(ptr);
            } else {
                let layout =
                    Layout::from_size_align_unchecked(self.capacity, align_of::<AlignType>());
                let ptr = realloc(self.data.as_ptr(), layout, capacity);

                if ptr.is_null() {
                    handle_alloc_error(layout);
                }

                self.data = ptr::NonNull::new_unchecked(ptr);
            }
        }
    }

    /// Align the write end of the buffer and zero-initialize any padding.
    pub(crate) fn align_mut<T>(&mut self) {
        let padding = padding_to::<T>(self.written);
        let requested = self.written + padding + size_of::<T>();

        self.ensure_capacity(requested);

        // SAFETY: We've ensured that the buffer has sufficient capacity just
        // above.
        unsafe {
            self.zero(padding);
        }
    }

    unsafe fn zero(&mut self, len: usize) {
        let at = self.data.as_ptr().add(self.written);
        at.write_bytes(0, len);
        // Skip over calculating padding.
        self.written += len;
    }
}

// SAFETY: [`AlignedBuf`] is `Send` because it owns data of type `u8`.
unsafe impl Send for AlignedBuf {}
// SAFETY: [`AlignedBuf`] is `Send` because it owns data of type `u8`.
unsafe impl Sync for AlignedBuf {}

impl fmt::Debug for AlignedBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AlignedBuf")
            .field("len", &self.len())
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl Default for AlignedBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AlignedBuf {
    fn drop(&mut self) {
        unsafe {
            if self.capacity > 0 {
                let layout =
                    Layout::from_size_align_unchecked(self.capacity, align_of::<AlignType>());
                dealloc(self.data.as_ptr(), layout);
                self.capacity = 0;
            }
        }
    }
}

impl BufMut for AlignedBuf {
    #[inline]
    fn align_mut<T>(&mut self) {
        AlignedBuf::align_mut::<T>(self)
    }

    #[inline]
    fn len(&self) -> usize {
        AlignedBuf::len(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        AlignedBuf::is_empty(self)
    }

    #[inline]
    fn write<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Write,
    {
        AlignedBuf::write(self, value)
    }

    #[inline]
    fn store<T>(&mut self, frame: T) -> Result<()>
    where
        T: Frame,
    {
        AlignedBuf::store(self, frame);
        Ok(())
    }

    #[inline]
    fn alloc<T>(&mut self) -> Alloc<T>
    where
        T: Frame,
    {
        AlignedBuf::alloc(self)
    }

    #[inline]
    fn store_at<T>(&mut self, at: Alloc<T>, frame: T)
    where
        T: Frame,
    {
        AlignedBuf::store_at(self, at, frame)
    }

    #[inline]
    fn extend_from_slice(&mut self, bytes: &[u8]) {
        AlignedBuf::extend_from_slice(self, bytes);
    }

    #[inline]
    fn extend_from_slice_nul(&mut self, bytes: &[u8]) {
        AlignedBuf::extend_from_slice_nul(self, bytes);
    }
}

impl PartialEq<AlignedBuf> for AlignedBuf {
    #[inline]
    fn eq(&self, other: &AlignedBuf) -> bool {
        self.get() == other.get()
    }
}

impl PartialEq<ReadBuf<'_>> for AlignedBuf {
    #[inline]
    fn eq(&self, other: &ReadBuf<'_>) -> bool {
        self.get() == other.get()
    }
}

impl Eq for AlignedBuf {}

impl Clone for AlignedBuf {
    #[inline]
    fn clone(&self) -> Self {
        let mut buf = Self::new();
        buf.extend_from_slice(self.get());
        buf
    }
}

/// Construct an aligned buffer from a read buffer.
impl From<ReadBuf<'_>> for AlignedBuf {
    #[inline]
    fn from(value: ReadBuf<'_>) -> Self {
        let mut buf = Self::new();
        buf.extend_from_slice(value.get());
        buf
    }
}
