use std::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};
use std::mem::size_of;
use std::ptr;
use std::slice::{from_raw_parts, from_raw_parts_mut};

use crate::buf::{
    max_size_for_align, padding_to, Alloc, ArrayWriter, BufMut, ReadBuf, StructWriter,
};
use crate::proto::Endianness;
use crate::{Frame, Write};

/// The alignment of the buffer.
const ALIGNMENT: usize = 1;

/// A buffer that can be used for buffering aligned data.
pub(crate) struct OwnedBuf {
    /// Pointed to data of the buffer.
    data: ptr::NonNull<u8>,
    /// The initialized capacity of the buffer.
    capacity: usize,
    /// Write position in the buffer.
    written: usize,
    /// Read position in the buffer.
    read: usize,
    /// Current frame basis used for alignment.
    base: usize,
    /// Dynamic endainness of the buffer.
    endianness: Endianness,
}

impl OwnedBuf {
    /// Construct a new empty buffer.
    pub(crate) fn new() -> Self {
        Self::with_endianness(Endianness::NATIVE)
    }

    /// Construct a new buffer with the specified endianness.
    pub(crate) fn with_endianness(endianness: Endianness) -> Self {
        Self {
            data: ptr::NonNull::dangling(),
            capacity: 0,
            written: 0,
            read: 0,
            base: 0,
            endianness,
        }
    }

    /// Get the endianness of the buffer.
    pub(crate) fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Set the endianness of the buffer.
    pub(crate) fn set_endianness(&mut self, endianness: Endianness) {
        self.endianness = endianness;
    }

    /// Write an array into the buffer.
    pub(crate) fn write_array(&mut self) -> ArrayWriter<'_, Self> {
        ArrayWriter::new(self)
    }

    /// Write a struct into the buffer.
    pub(crate) fn write_struct(&mut self) -> StructWriter<'_, Self> {
        StructWriter::new(self)
    }

    /// Update alignment basis to match the write location.
    ///
    /// This ensures that subsequent writes are aligned even if the underlying
    /// buffer is not.
    pub(crate) fn update_alignment_base(&mut self) {
        self.base = self.written;
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
        ReadBuf::new(data, len, self.endianness)
    }

    /// Read the entire buffer and make accessible through [`ReadBuf`].
    #[cfg(test)]
    pub(crate) fn read(&mut self) -> ReadBuf<'_> {
        let len = self.len();
        let data = unsafe { ptr::NonNull::new_unchecked(self.data.as_ptr().add(self.read)) };
        self.clear();
        ReadBuf::new(data, len, self.endianness)
    }

    /// Access a read buf which peeks into the buffer without advancing it.
    pub(crate) fn peek_buf(&self, len: usize) -> ReadBuf<'_> {
        assert!(len <= self.len());
        let data = unsafe { ptr::NonNull::new_unchecked(self.data.as_ptr().add(self.read)) };
        ReadBuf::new(data, len, self.endianness)
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
    pub(crate) fn store_at<T>(&mut self, at: Alloc<T>, mut frame: T)
    where
        T: Frame,
    {
        let at = at.into_usize();
        assert!(at + size_of::<T>() <= self.written, "write underflow");
        frame.adjust(self.endianness);

        // SAFETY: We've just asserted that the write is in bounds above.
        unsafe {
            let src = (&frame as *const T).cast::<u8>();
            let dst = self.data.as_ptr().add(at);
            ptr::copy_nonoverlapping(src, dst, size_of::<T>());
        }
    }

    /// Store a [`Frame`] of type `T` in the buffer.
    ///
    /// This both allocates enough space for the frame and ensures that the
    /// buffer is aligned per the requirements of the frame.
    pub(crate) fn store<T>(&mut self, mut frame: T)
    where
        T: Frame,
    {
        self.align_mut::<T>();
        frame.adjust(self.endianness);

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
        value.write_to(self);
    }

    /// Extend the buffer with a slice.
    pub(crate) fn extend_from_slice(&mut self, bytes: &[u8]) {
        let requested = self.written + bytes.len();
        self.ensure_capacity(requested);

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
        self.base = 0;
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

    /// Ensure that the buffer has at least `capacity` bytes.
    fn ensure_capacity(&mut self, capacity: usize) {
        if capacity <= self.capacity {
            return;
        }

        let capacity = 16usize.max(capacity.next_power_of_two());

        assert!(
            capacity <= max_size_for_align(ALIGNMENT),
            "capacity overflow"
        );

        self.realloc(capacity);
        self.capacity = capacity;
    }

    fn realloc(&mut self, capacity: usize) {
        unsafe {
            if self.capacity == 0 {
                let layout = Layout::from_size_align_unchecked(capacity, ALIGNMENT);
                let ptr = alloc(layout);

                if ptr.is_null() {
                    handle_alloc_error(layout);
                }

                self.data = ptr::NonNull::new_unchecked(ptr);
            } else {
                let layout = Layout::from_size_align_unchecked(self.capacity, ALIGNMENT);
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
        let at = self.data.as_ptr().add(self.written);
        at.write_bytes(0, len);
        // Skip over calculating padding.
        self.written += len;
    }
}

impl Default for OwnedBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for OwnedBuf {
    fn drop(&mut self) {
        unsafe {
            if self.capacity > 0 {
                let layout = Layout::from_size_align_unchecked(self.capacity, ALIGNMENT);
                dealloc(self.data.as_ptr(), layout);
                self.capacity = 0;
            }
        }
    }
}

impl BufMut for OwnedBuf {
    #[inline]
    fn align_mut<T>(&mut self) {
        OwnedBuf::align_mut::<T>(self)
    }

    #[inline]
    fn len(&self) -> usize {
        OwnedBuf::len(self)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        OwnedBuf::is_empty(self)
    }

    #[inline]
    fn store<T>(&mut self, frame: T)
    where
        T: Frame,
    {
        OwnedBuf::store(self, frame)
    }

    #[inline]
    fn alloc<T>(&mut self) -> Alloc<T>
    where
        T: Frame,
    {
        OwnedBuf::alloc(self)
    }

    #[inline]
    fn store_at<T>(&mut self, at: Alloc<T>, frame: T)
    where
        T: Frame,
    {
        OwnedBuf::store_at(self, at, frame)
    }

    #[inline]
    fn extend_from_slice(&mut self, bytes: &[u8]) {
        OwnedBuf::extend_from_slice(self, bytes);
    }

    #[inline]
    fn extend_from_slice_nul(&mut self, bytes: &[u8]) {
        OwnedBuf::extend_from_slice_nul(self, bytes);
    }
}
