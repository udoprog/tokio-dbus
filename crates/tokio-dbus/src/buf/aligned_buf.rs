use std::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};
use std::fmt;
use std::mem::{align_of, size_of};
use std::ptr;
use std::slice::{from_raw_parts, from_raw_parts_mut};

use crate::buf::{max_size_for_align, padding_to, Aligned, Alloc};
use crate::Frame;

/// The type we're basing our alignment on.
pub(crate) type AlignType = u64;

/// An owned buffer which is aligned per the specification of D-Bus messages.
pub(crate) struct AlignedBuf {
    /// Pointed to data of the buffer.
    data: ptr::NonNull<u8>,
    /// The initialized capacity of the buffer.
    capacity: usize,
    /// Write position in the buffer.
    len: usize,
}

impl AlignedBuf {
    /// Construct a new empty buffer.
    pub(crate) const fn new() -> Self {
        Self {
            data: ptr::NonNull::<AlignType>::dangling().cast(),
            capacity: 0,
            len: 0,
        }
    }

    /// Access a read buf which peeks into the buffer without advancing it.
    pub(crate) fn as_aligned(&self) -> Aligned<'_> {
        let len = self.len();
        let data = unsafe { ptr::NonNull::new_unchecked(self.data.as_ptr()) };
        Aligned::new(data, len)
    }

    /// Allocate, zero space for and align data for `T`.
    pub(crate) fn alloc<T>(&mut self) -> Alloc<T>
    where
        T: Frame,
    {
        self.align_mut::<T>();
        let at = self.len;

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
        assert!(at + size_of::<T>() <= self.len, "write underflow");

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
            self.data.as_ptr().add(self.len).cast::<T>().write(frame);
            self.len += size_of::<T>();
        }
    }

    /// Extend the buffer with a slice.
    pub(crate) fn extend_from_slice(&mut self, bytes: &[u8]) {
        let requested = self.len + bytes.len();
        self.ensure_capacity(requested);

        // SAFETY: We've ensures that the necessary capacity is available.
        unsafe {
            self.data
                .as_ptr()
                .add(self.len)
                .copy_from(bytes.as_ptr(), bytes.len());
        }

        self.len += bytes.len();
    }

    /// Extend the buffer with a slice ending with a NUL byte.
    pub(crate) fn extend_from_slice_nul(&mut self, bytes: &[u8]) {
        let requested = self.len + bytes.len() + 1;
        self.ensure_capacity(requested);

        // SAFETY: We've ensures that the necessary capacity is available.
        unsafe {
            let ptr = self.data.as_ptr().add(self.len);
            ptr.copy_from(bytes.as_ptr(), bytes.len());
            ptr.add(bytes.len()).write(0u8);
        }

        self.len += bytes.len() + 1;
    }

    /// Reserve space for `bytes` additional bytes in the buffer.
    pub(crate) fn reserve_bytes(&mut self, bytes: usize) {
        let requested = self.len + bytes;
        self.ensure_capacity(requested);
    }

    /// Test if the buffer is empty.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Remaining data to be read from the buffer.
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.len
    }

    /// Get a slice out of the buffer that has been written to.
    pub(crate) fn get(&self) -> &[u8] {
        unsafe {
            let at = self.data.as_ptr();
            from_raw_parts(at, self.len())
        }
    }

    /// Get remaining slice of the buffer that has not been written to, but is
    /// zeroed.
    pub(crate) fn get_mut(&mut self) -> &mut [u8] {
        unsafe {
            let len = self.capacity - self.len;
            let at = self.data.as_ptr().add(self.len);
            from_raw_parts_mut(at, len)
        }
    }

    /// Indicate that we've written `n` bytes to the buffer.
    pub(crate) fn advance(&mut self, n: usize) {
        self.len += n;
    }

    /// Clear the current buffer.
    pub(crate) fn clear(&mut self) {
        self.len = 0;
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
        let padding = padding_to::<T>(self.len);
        let requested = self.len + padding + size_of::<T>();

        self.ensure_capacity(requested);

        // SAFETY: We've ensured that the buffer has sufficient capacity just
        // above.
        unsafe {
            self.zero(padding);
        }
    }

    unsafe fn zero(&mut self, len: usize) {
        let at = self.data.as_ptr().add(self.len);
        at.write_bytes(0, len);
        // Skip over calculating padding.
        self.len += len;
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

impl PartialEq<AlignedBuf> for AlignedBuf {
    #[inline]
    fn eq(&self, other: &AlignedBuf) -> bool {
        self.get() == other.get()
    }
}

impl PartialEq<Aligned<'_>> for AlignedBuf {
    #[inline]
    fn eq(&self, other: &Aligned<'_>) -> bool {
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
impl From<Aligned<'_>> for AlignedBuf {
    #[inline]
    fn from(value: Aligned<'_>) -> Self {
        let mut buf = Self::new();
        buf.extend_from_slice(value.get());
        buf
    }
}
