use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use crate::buf::Alloc;
use crate::ty;
use crate::{BodyBuf, Storable};

use super::TypedStructWriter;

/// Write a typed array.
///
/// See [`BodyBuf::store_array`].
///
/// [`BodyBuf::store_array`]: crate::BodyBuf::store_array
pub struct TypedArrayWriter<'a, T>
where
    T: ty::Aligned,
{
    buf: &'a mut BodyBuf,
    len: Alloc<u32>,
    start: usize,
    _marker: PhantomData<T>,
}

impl<'a, T> TypedArrayWriter<'a, T>
where
    T: ty::Aligned,
{
    pub(crate) fn new(buf: &'a mut BodyBuf) -> Self {
        let len = buf.alloc();
        let start = buf.len();

        Self {
            buf,
            start,
            len,
            _marker: PhantomData,
        }
    }

    /// Finish writing the array.
    ///
    /// This will also be done implicitly once this is dropped.
    ///
    /// See [`BodyBuf::store_array`].
    ///
    /// [`BodyBuf::store_array`]: crate::BodyBuf::store_array
    #[inline]
    pub fn finish(self) {
        ManuallyDrop::new(self).finalize();
    }

    #[inline(always)]
    fn finalize(&mut self) {
        let end = self.buf.len();
        let len = (end - self.start) as u32;
        self.buf.store_at(self.len, len);
        self.buf.align_mut::<T::Type>();
    }
}

impl<'a, T> TypedArrayWriter<'a, T>
where
    T: ty::Aligned,
{
    /// Store a value and return the builder for the next value to store.
    ///
    /// See [`BodyBuf::store_array`].
    ///
    /// [`BodyBuf::store_array`]: crate::BodyBuf::store_array
    pub fn store(&mut self, value: T::Return<'_>)
    where
        T: ty::Marker,
        for<'b> T::Return<'b>: Storable,
    {
        value.store_to(self.buf);
    }

    /// Write a struct inside of the array.
    ///
    /// See [`BodyBuf::store_array`].
    ///
    /// [`BodyBuf::store_array`]: crate::BodyBuf::store_array
    #[inline]
    pub fn store_struct(&mut self) -> TypedStructWriter<'_, T>
    where
        T: ty::Fields,
    {
        TypedStructWriter::new(self.buf)
    }
}

impl<'a, T> TypedArrayWriter<'a, ty::Array<T>>
where
    T: ty::Aligned,
{
    /// Write an array inside of the array.
    ///
    /// See [`BodyBuf::store_array`].
    ///
    /// [`BodyBuf::store_array`]: crate::BodyBuf::store_array
    #[inline]
    pub fn store_array(&mut self) -> TypedArrayWriter<'_, T> {
        TypedArrayWriter::new(self.buf)
    }
}

impl<'a> TypedArrayWriter<'a, u8> {
    /// Write a byte array inside of the array.
    ///
    /// See [`BodyBuf::store_array`].
    ///
    /// [`BodyBuf::store_array`]: crate::BodyBuf::store_array
    #[inline]
    pub fn write_slice(self, bytes: &[u8]) {
        let mut this = ManuallyDrop::new(self);
        this.buf.extend_from_slice(bytes);
        this.finalize();
    }
}
