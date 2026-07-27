use std::marker::PhantomData;
use std::mem::ManuallyDrop;

use crate::buf::Alloc;
use crate::ty;
use crate::{BodyBuf, Signature, Storable};

use super::{StoreStruct, StoreVariant};

/// Write a typed array.
///
/// See [`BodyBuf::store_array`].
///
/// [`BodyBuf::store_array`]: crate::BodyBuf::store_array
pub struct StoreArray<'a, T>
where
    T: ty::Aligned,
{
    buf: &'a mut BodyBuf,
    len: Alloc<u32>,
    start: usize,
    _marker: PhantomData<T>,
}

impl<'a, T> StoreArray<'a, T>
where
    T: ty::Aligned,
{
    pub(crate) fn new(buf: &'a mut BodyBuf) -> Self {
        let len = buf.alloc();
        // NB: The length prefix is followed by padding up to the alignment of
        // the element type. This padding is present even if the array is empty,
        // and is *not* included in the encoded length.
        buf.align_mut::<T::Alignment>();
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
    }
}

impl<T> Drop for StoreArray<'_, T>
where
    T: ty::Aligned,
{
    #[inline]
    fn drop(&mut self) {
        self.finalize();
    }
}

impl<T> StoreArray<'_, T>
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
    pub fn store_struct(&mut self) -> StoreStruct<'_, T>
    where
        T: ty::Fields,
    {
        StoreStruct::new(self.buf)
    }
}

impl StoreArray<'_, ty::Variant> {
    /// Write a variant inside of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Signature, Variant};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// let mut array = buf.store_array::<ty::Variant>()?;
    /// array.store_variant(Signature::STRING).store("Hello World!");
    /// array.store_variant(Signature::UINT32).store(42u32);
    /// array.finish();
    ///
    /// assert_eq!(buf.signature(), "av");
    ///
    /// let mut buf = buf.as_body();
    /// let mut array = buf.load_array::<ty::Variant>()?;
    /// assert_eq!(array.load_variant()?, Some(Variant::String("Hello World!")));
    /// assert_eq!(array.load_variant()?, Some(Variant::U32(42)));
    /// assert_eq!(array.load_variant()?, None);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn store_variant(&mut self, signature: &Signature) -> StoreVariant<'_> {
        StoreVariant::new(self.buf, signature)
    }
}

impl<K, V> StoreArray<'_, ty::Dict<K, V>>
where
    K: ty::Marker,
    V: ty::Marker,
{
    /// Write a dict entry inside of the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Signature, Variant};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// let mut dict = buf.store_array::<ty::Dict<ty::Str, ty::Variant>>()?;
    /// dict.store_entry().store("Id").store(Variant::String("example")).finish();
    /// dict.store_entry().store("ItemIsMenu").store(Variant::Bool(false)).finish();
    /// dict.finish();
    ///
    /// assert_eq!(buf.signature(), "a{sv}");
    ///
    /// let mut buf = buf.as_body();
    /// let mut dict = buf.load_array::<ty::Dict<ty::Str, ty::Variant>>()?;
    ///
    /// assert_eq!(dict.load_entry()?, Some(("Id", Variant::String("example"))));
    /// assert_eq!(dict.load_entry()?, Some(("ItemIsMenu", Variant::Bool(false))));
    /// assert_eq!(dict.load_entry()?, None);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn store_entry(&mut self) -> StoreStruct<'_, (K, V)> {
        // NB: Dict entries are laid out exactly like a two-field struct.
        StoreStruct::new(self.buf)
    }
}

impl<T> StoreArray<'_, ty::Array<T>>
where
    T: ty::Aligned,
{
    /// Write an array inside of the array.
    ///
    /// See [`BodyBuf::store_array`].
    ///
    /// [`BodyBuf::store_array`]: crate::BodyBuf::store_array
    #[inline]
    pub fn store_array(&mut self) -> StoreArray<'_, T> {
        StoreArray::new(self.buf)
    }
}

impl StoreArray<'_, u8> {
    /// Extend a byte array with the given slice.
    ///
    /// See [`BodyBuf::store_array`].
    ///
    /// [`BodyBuf::store_array`]: crate::BodyBuf::store_array
    #[inline]
    pub fn write_slice(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }
}
