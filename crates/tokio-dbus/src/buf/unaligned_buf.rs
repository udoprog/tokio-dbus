use core::alloc::Layout;
use core::mem::size_of;
use core::ptr;

use alloc::alloc::{alloc_zeroed, dealloc, handle_alloc_error, realloc};

use crate::buf::{max_size_for_align, padding_to};
use crate::{Frame, Write, WriteUnaligned};

use super::Alloc;

/// A buffer that can be used for buffering unaligned data.
pub struct UnalignedBuf {
    /// Pointed to data of the buffer.
    data: ptr::NonNull<u8>,
    /// The capacity of the buffer. All of it is initialized, since growth
    /// zeroes the new region.
    capacity: usize,
    /// Write position in the buffer.
    written: usize,
    /// Read position in the buffer.
    read: usize,
    /// Current frame basis used for alignment for types which need it, since
    /// the buffer itself is not aligned the write location must be offset by
    /// this when writing new frames.
    base: usize,
}

impl UnalignedBuf {
    /// Construct a new empty buffer.
    pub(crate) fn new() -> Self {
        Self {
            data: ptr::NonNull::dangling(),
            capacity: 0,
            written: 0,
            read: 0,
            base: 0,
        }
    }

    /// Update alignment basis to match the write location.
    ///
    /// This ensures that subsequent writes are aligned even if the underlying
    /// buffer is not.
    pub(crate) fn update_base_align(&mut self) {
        self.base = self.written;
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
            let from = (&frame as *const T).cast::<u8>();
            self.data
                .as_ptr()
                .add(at)
                .copy_from_nonoverlapping(from, size_of::<T>());
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
            let src = (&frame as *const T).cast::<u8>();
            let dst = self.data.as_ptr().add(self.written);
            ptr::copy_nonoverlapping(src, dst, size_of::<T>());
            self.written += size_of::<T>();
        }
    }

    /// Write a type to the buffer.
    pub(crate) fn write<T>(&mut self, value: &T)
    where
        T: ?Sized + Write,
    {
        value.write_to_unaligned(self);
    }

    /// Extend the buffer with a slice.
    pub(crate) fn extend_from_slice(&mut self, bytes: &[u8]) {
        let requested = self.written + bytes.len();
        self.ensure_capacity(requested);

        // SAFETY: We've ensures that we have the necessary capacity just above.
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
        let len = bytes.len() + 1;
        self.ensure_capacity(self.written + len);

        // SAFETY: We've ensures that we have the necessary capacity just above.
        unsafe {
            let ptr = self.data.as_ptr().add(self.written);
            ptr.copy_from(bytes.as_ptr(), bytes.len());
            ptr.add(bytes.len()).write(0u8);
        }

        self.written += len;
    }

    #[cfg(feature = "tokio")]
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

    #[cfg(feature = "tokio")]
    /// Get a slice out of the buffer that has ben written to.
    pub(crate) fn get(&self) -> &[u8] {
        unsafe {
            let at = self.data.as_ptr().add(self.read);
            core::slice::from_raw_parts(at, self.len())
        }
    }

    #[cfg(feature = "tokio")]
    /// Get the remaining capacity of the buffer that has not been written to.
    ///
    /// Its contents are unspecified: zeroes, or bytes left over from before the
    /// buffer was last cleared.
    pub(crate) fn get_mut(&mut self) -> &mut [u8] {
        // SAFETY: `written <= capacity`, and every byte up to `capacity` is
        // initialized since `realloc` zeroes all memory it adds.
        unsafe {
            let len = self.capacity - self.written;
            let at = self.data.as_ptr().add(self.written);
            core::slice::from_raw_parts_mut(at, len)
        }
    }

    #[cfg(feature = "tokio")]
    /// Indicate that we've written `n` bytes to the buffer.
    pub(crate) fn advance_mut(&mut self, n: usize) {
        self.written += n;
    }

    #[cfg(feature = "tokio")]
    /// Indicate that we've read `n` bytes from the buffer.
    pub(crate) fn advance(&mut self, n: usize) {
        self.read += n;

        if self.read == self.written {
            self.clear();
        }
    }

    #[cfg(feature = "tokio")]
    /// Clear the current buffer.
    pub(crate) fn clear(&mut self) {
        self.read = 0;
        self.written = 0;
        self.base = 0;
    }

    /// Ensure that the buffer has at least `capacity` bytes.
    fn ensure_capacity(&mut self, capacity: usize) {
        if capacity <= self.capacity {
            return;
        }

        let capacity = 16usize.max(capacity.next_power_of_two());

        assert!(capacity <= max_size_for_align(1), "capacity overflow");

        self.realloc(capacity);
        self.capacity = capacity;
    }

    /// Grow the allocation to `capacity` bytes, zeroing the added region.
    ///
    /// `capacity` must be non-zero, larger than the current capacity, and
    /// within `max_size_for_align(1)`, as `ensure_capacity` checks.
    fn realloc(&mut self, capacity: usize) {
        // SAFETY: `ensure_capacity` bounds `capacity` to fit in `isize`. The
        // old layout is the one the current allocation was made with, and
        // `capacity > self.capacity` so the zeroed tail is in bounds of the new
        // allocation.
        unsafe {
            let new_layout = Layout::from_size_align_unchecked(capacity, 1);

            let ptr = if self.capacity == 0 {
                alloc_zeroed(new_layout)
            } else {
                let layout = Layout::from_size_align_unchecked(self.capacity, 1);
                let ptr = realloc(self.data.as_ptr(), layout, capacity);

                if !ptr.is_null() {
                    ptr.add(self.capacity)
                        .write_bytes(0, capacity - self.capacity);
                }

                ptr
            };

            if ptr.is_null() {
                handle_alloc_error(new_layout);
            }

            self.data = ptr::NonNull::new_unchecked(ptr);
        }
    }

    /// Align the write end of the buffer and zero-initialize any padding.
    pub(crate) fn align_mut<T>(&mut self)
    where
        T: Frame,
    {
        let padding = padding_to::<T>(self.written - self.base);
        let requested = self.written + padding + size_of::<T>();
        self.ensure_capacity(requested);

        // SAFETY: We've ensured that the buffer has sufficient capacity just
        // above.
        unsafe {
            self.zero(padding);
        }
    }

    unsafe fn zero(&mut self, len: usize) {
        unsafe {
            let at = self.data.as_ptr().wrapping_add(self.written);
            at.write_bytes(0, len);
        }

        // Skip over calculating padding.
        self.written += len;
    }
}

// SAFETY: [`UnalignedBuf`] is `Send` because it owns data of type `u8`.
unsafe impl Send for UnalignedBuf {}
// SAFETY: [`UnalignedBuf`] is `Send` because it owns data of type `u8`.
unsafe impl Sync for UnalignedBuf {}

impl Default for UnalignedBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for UnalignedBuf {
    fn drop(&mut self) {
        unsafe {
            if self.capacity > 0 {
                let layout = Layout::from_size_align_unchecked(self.capacity, 1);
                dealloc(self.data.as_ptr(), layout);
                self.capacity = 0;
            }
        }
    }
}

impl WriteUnaligned for UnalignedBuf {
    #[inline]
    fn store<T>(&mut self, frame: T)
    where
        T: Frame,
    {
        UnalignedBuf::store(self, frame)
    }

    #[inline]
    fn extend_from_slice(&mut self, bytes: &[u8]) {
        UnalignedBuf::extend_from_slice(self, bytes)
    }

    #[inline]
    fn extend_from_slice_nul(&mut self, bytes: &[u8]) {
        UnalignedBuf::extend_from_slice_nul(self, bytes)
    }
}
