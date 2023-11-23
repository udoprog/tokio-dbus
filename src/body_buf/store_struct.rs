use std::marker::PhantomData;

use crate::ty;
use crate::{Arguments, BodyBuf, Storable};

use super::StoreArray;

/// Write a struct.
///
/// See [`BodyBuf::store_struct`].
///
/// [`BodyBuf::store_struct`]: crate::BodyBuf::store_struct
#[must_use = "Must call `finish` after writing all related fields"]
pub struct StoreStruct<'a, T> {
    buf: &'a mut BodyBuf,
    _marker: PhantomData<T>,
}

impl<'a, T> StoreStruct<'a, T> {
    pub(crate) fn new(buf: &'a mut BodyBuf) -> Self {
        buf.align_mut::<u64>();
        Self::inner(buf)
    }

    pub(crate) fn inner(buf: &'a mut BodyBuf) -> Self {
        Self {
            buf,
            _marker: PhantomData,
        }
    }

    /// Store a value and return the builder for the next value to store.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    /// use tokio_dbus::ty;
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    ///
    /// buf.store_struct::<(u16, u32)>()?
    ///     .store(10u16)
    ///     .store(10u32)
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"(qu)");
    /// assert_eq!(buf.get(), &[10, 0, 0, 0, 10, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    ///
    /// Examples using unsized types:
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    /// use tokio_dbus::ty;
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    ///
    /// buf.store_struct::<(ty::Str,)>()?
    ///     .store("Hello World")
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"(s)");
    /// assert_eq!(buf.get(), &[11, 0, 0, 0, 72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn store(self, value: <T::First as ty::Marker>::Return<'_>) -> StoreStruct<'a, T::Remaining>
    where
        T: ty::Fields,
        T::First: ty::Marker,
        for<'b> <T::First as ty::Marker>::Return<'b>: Storable,
    {
        value.store_to(self.buf);
        StoreStruct::inner(self.buf)
    }

    /// Store a value and return the builder for the next value to store.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    /// use tokio_dbus::ty;
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    ///
    /// buf.store_struct::<(u8, u32)>()?.fields((42u8, 42u32));
    ///
    /// assert_eq!(buf.signature(), b"(yu)");
    /// assert_eq!(buf.get(), &[42, 0, 0, 0, 42, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn fields(self, arguments: T)
    where
        T: Arguments,
    {
        arguments.buf_to(self.buf);
    }

    /// Store a value and return the builder for the next value to store.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Endianness};
    /// use tokio_dbus::ty;
    ///
    /// let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    ///
    /// buf.store_struct::<(ty::Array<u32>,)>()?
    ///     .store_array(|w| {
    ///         w.store(1);
    ///         w.store(2);
    ///         w.store(3);
    ///         w.store(4);
    ///     })
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"(au)");
    /// assert_eq!(buf.get(), &[16, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn store_array<W, U>(self, writer: W) -> StoreStruct<'a, T::Remaining>
    where
        W: FnOnce(&mut StoreArray<'_, U>),
        T: ty::Fields<First = ty::Array<U>>,
        U: ty::Aligned,
    {
        let mut w = StoreArray::new(self.buf);
        writer(&mut w);
        w.finish();
        StoreStruct::inner(self.buf)
    }

    /// Store a value and return the builder for the next value to store.
    ///
    /// See [`BodyBuf::store_struct`].
    ///
    /// [`BodyBuf::store_struct`]: crate::BodyBuf::store_struct
    #[inline]
    pub fn store_struct<W>(self, writer: W) -> StoreStruct<'a, T::Remaining>
    where
        W: FnOnce(&mut StoreStruct<'_, T::First>),
        T: ty::Fields,
        T::First: ty::Fields,
    {
        let mut w = StoreStruct::new(self.buf);
        writer(&mut w);
        StoreStruct::inner(self.buf)
    }
}

impl StoreStruct<'_, ()> {
    /// Finish writing the struct.
    ///
    /// See [`BodyBuf::store_struct`].
    ///
    /// [`BodyBuf::store_struct`]: crate::BodyBuf::store_struct
    pub fn finish(self) {}
}
