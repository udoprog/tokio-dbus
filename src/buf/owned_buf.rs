use std::alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout};
use std::marker::PhantomData;
use std::mem::{align_of, size_of};
use std::ptr;
use std::slice::{from_raw_parts, from_raw_parts_mut};

use crate::buf::{max_size_for_align, padding_to, ArrayWriter, ReadBuf, StructWriter};
use crate::frame::Frame;
use crate::protocol::Endianness;
use crate::Serialize;

/// An allocated location in the buffer that can be written to later.
pub struct Alloc<T>(usize, PhantomData<T>);

impl<T> Clone for Alloc<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Alloc<T> {}

/// The alignment of the buffer.
const ALIGNMENT: usize = 8;

/// A buffer that can be used for receiving messages.
pub struct OwnedBuf {
    /// Pointed to data of the buffer.
    data: ptr::NonNull<u8>,
    /// The initialized capacity of the buffer.
    capacity: usize,
    /// Write position in the buffer.
    written: usize,
    /// Read position in the buffer.
    read: usize,
    /// Dynamic endainness of the buffer.
    endianness: Endianness,
}

impl OwnedBuf {
    /// Construct a new empty buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::OwnedBuf;
    ///
    /// let buf = OwnedBuf::new();
    /// ```
    pub fn new() -> Self {
        Self::with_endianness(Endianness::NATIVE)
    }

    /// Construct a new buffer with the specified endianness.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Endianness, OwnedBuf};
    ///
    /// let buf = OwnedBuf::with_endianness(Endianness::LITTLE);
    /// ```
    pub fn with_endianness(endianness: Endianness) -> Self {
        Self {
            data: ptr::NonNull::dangling(),
            capacity: 0,
            read: 0,
            written: 0,
            endianness,
        }
    }

    /// Get the endianness of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Endianness, OwnedBuf};
    ///
    /// let buf = OwnedBuf::with_endianness(Endianness::LITTLE);
    /// assert_eq!(buf.endianness(), Endianness::LITTLE);
    /// ```
    pub fn endianness(&self) -> Endianness {
        self.endianness
    }

    /// Set the endianness of the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Endianness, OwnedBuf};
    ///
    /// let mut buf = OwnedBuf::with_endianness(Endianness::LITTLE);
    /// assert_eq!(buf.endianness(), Endianness::LITTLE);
    /// buf.set_endianness(Endianness::BIG);
    /// assert_eq!(buf.endianness(), Endianness::BIG);
    /// ```
    pub fn set_endianness(&mut self, endianness: Endianness) {
        self.endianness = endianness;
    }

    /// Write an array into the buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Endianness, OwnedBuf};
    ///
    /// let mut buf = OwnedBuf::with_endianness(Endianness::LITTLE);
    /// let mut array = buf.write_array();
    /// array.push(&1u32);
    /// array.finish();
    ///
    /// assert_eq!(buf.get(), &[4, 0, 0, 0, 1, 0, 0, 0]);
    /// ```
    pub fn write_array(&mut self) -> ArrayWriter<'_> {
        ArrayWriter::new(self)
    }

    /// Write a struct.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Endianness, OwnedBuf};
    ///
    /// let mut buf = OwnedBuf::with_endianness(Endianness::LITTLE);
    /// buf.write(&10u8);
    ///
    /// let mut st = buf.write_struct();
    /// st.write(&1u32);
    ///
    /// assert_eq!(buf.get(), &[10, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0]);
    /// ```
    pub fn write_struct(&mut self) -> StructWriter<'_> {
        StructWriter::new(self)
    }

    /// Write a type which can be serialized.
    pub fn write<T>(&mut self, value: &T)
    where
        T: ?Sized + Serialize,
    {
        value.write_to(self);
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
    /// dbg!(buf.len());
    ///
    /// let mut read_buf = buf.read_buf(6);
    ///
    /// assert_eq!(read_buf.read::<u32>()?, &4);
    /// assert_eq!(read_buf.read::<u8>()?, &1);
    /// assert_eq!(read_buf.read::<u8>()?, &2);
    /// assert_eq!(buf.get(), &[3, 4, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn read_buf(&mut self, len: usize) -> ReadBuf<'_> {
        assert!(len <= self.len());
        let read = self.read;
        self.read += len;
        ReadBuf::new(self.data, read, read + len, self.endianness)
    }

    /// Read a frame of the given type.
    pub(crate) fn load<T>(&mut self) -> &T
    where
        T: Frame,
    {
        let padding = padding_to::<T>(self.read);

        assert!(
            self.read + padding + size_of::<T>() <= self.written,
            "read underflow"
        );

        self.read += padding;

        // SAFETY: read is guaranteed to be in bounds of the buffer.
        unsafe {
            let ptr = self.data.as_ptr().add(self.read).cast::<T>();
            self.read += size_of::<T>();
            // NB: The pointer is aligned.
            (*ptr).adjust(self.endianness);
            &*ptr
        }
    }

    /// Allocate, zero space for and align data for `T`.
    pub(crate) fn alloc<T>(&mut self) -> Alloc<T>
    where
        T: Frame,
    {
        self.reserve_and_align::<T>();
        let at = self.written;

        // SAFETY: We've just reserved and aligned the buffer above.
        unsafe {
            self.zero(size_of::<T>());
        }

        Alloc(at, PhantomData)
    }

    /// Write the given value at the given offset.
    pub(crate) fn store_at<T>(&mut self, at: Alloc<T>, frame: &T)
    where
        T: Frame,
    {
        let Alloc(at, _) = at;
        assert!(at + size_of::<T>() <= self.written, "write underflow");

        // SAFETY: We've just asserted that the write is in bounds above.
        unsafe {
            let at = self.data.as_ptr().add(at).cast::<T>();
            // NB: The write is aligned.
            ptr::copy_nonoverlapping(frame, at, 1);
            (*at).adjust(self.endianness);
        }
    }

    /// Write the given frame to the buffer.
    pub(crate) fn store<T>(&mut self, frame: &T)
    where
        T: Frame,
    {
        self.reserve_and_align::<T>();

        // SAFETY: We've just reserved and aligned the buffer in the `reserve`
        // call just above.
        unsafe {
            // NB: The write is aligned.
            let at = self.data.as_ptr().add(self.written).cast::<T>();
            ptr::copy_nonoverlapping(frame, at, 1);
            (*at).adjust(self.endianness);
            self.written += size_of::<T>();
        }
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

    /// Reserve space for `T` and align the buffer according to it.
    pub(crate) fn reserve_and_align<T>(&mut self) {
        debug_assert!(align_of::<T>() <= ALIGNMENT);

        let padding = padding_to::<T>(self.written);
        let requested = self.written + padding + size_of::<T>();
        self.ensure_capacity(requested);

        // SAFETY: We've ensured that the buffer has sufficient capacity just
        // above.
        unsafe {
            self.zero(padding);
        }
    }

    /// Reserve space for `bytes` additional bytes in the buffer.
    pub(crate) fn reserve_bytes(&mut self, bytes: usize) {
        let requested = self.written + bytes;
        self.ensure_capacity(requested);
    }

    /// Test if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.read == self.written
    }

    /// Remaining data to be read from the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.written - self.read
    }

    /// Get a slice out of the buffer that has ben written to.
    pub fn get(&self) -> &[u8] {
        unsafe {
            let at = self.data.as_ptr().add(self.read);
            from_raw_parts(at, self.len())
        }
    }

    /// Take `n` bytes from the buffer while advancing the reader.
    ///
    /// If `n` is larger than the length as reported by [`len()`], the length
    /// will be used instead.
    ///
    /// [`len()`]: Self::len
    pub fn take(&mut self, n: usize) -> &[u8] {
        unsafe {
            let at = self.data.as_ptr().add(self.read);
            let len = self.len().min(n);
            self.read += n;
            let bytes = from_raw_parts(at, len);
            bytes
        }
    }

    /// Indicate that we've read `n` bytes from the buffer.
    pub fn advance(&mut self, n: usize) {
        self.read += n;

        if self.read == self.written {
            self.read = 0;
            self.written = 0;
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
    pub fn advance_mut(&mut self, n: usize) {
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
        let padding = padding_to::<T>(self.written);
        let requested = self.written + padding;
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
