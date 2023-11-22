use std::marker::PhantomData;

use crate::{Frame, Result, Write};

mod sealed {
    use crate::buf::{AlignedBuf, BodyBuf, UnalignedBuf};

    pub trait Sealed {}

    impl Sealed for BodyBuf {}
    impl Sealed for AlignedBuf {}
    impl Sealed for UnalignedBuf {}
}

/// An allocated location in the buffer that can be written to later.
pub struct Alloc<T>(usize, PhantomData<T>);

impl<T> Alloc<T> {
    pub(crate) fn new(at: usize) -> Self {
        Self(at, PhantomData)
    }

    pub(crate) fn into_usize(self) -> usize {
        self.0
    }
}

impl<T> Clone for Alloc<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Alloc<T> {}

/// A mutable buffer.
pub trait BufMut: self::sealed::Sealed {
    /// Align the write end of the buffer and zero-initialize any padding.
    #[doc(hidden)]
    fn align_mut<T>(&mut self);

    /// Remaining data to be read from the buffer.
    #[doc(hidden)]
    fn len(&self) -> usize;

    /// Test if the buffer is empty.
    #[doc(hidden)]
    fn is_empty(&self) -> bool;

    /// Store a [`Frame`] of type `T` in the buffer.
    ///
    /// This both allocates enough space for the frame and ensures that the
    /// buffer is aligned per the requirements of the frame.
    #[doc(hidden)]
    fn store<T>(&mut self, frame: T) -> Result<()>
    where
        T: Frame;

    /// Write a [`Write`] of type `T` in the buffer.
    #[doc(hidden)]
    fn write<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Write;

    /// Allocate, zero space for and align data for `T`.
    #[doc(hidden)]
    fn alloc<T>(&mut self) -> Alloc<T>
    where
        T: Frame;

    /// Write the given value at the previously [`Alloc<T>`] position.
    #[doc(hidden)]
    fn store_at<T>(&mut self, at: Alloc<T>, frame: T)
    where
        T: Frame;

    /// Extend the buffer with a slice.
    #[doc(hidden)]
    fn extend_from_slice(&mut self, bytes: &[u8]);

    /// Extend the buffer with a slice ending with a NUL byte.
    #[doc(hidden)]
    fn extend_from_slice_nul(&mut self, bytes: &[u8]);
}
