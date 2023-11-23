use std::marker::PhantomData;

use crate::ty;
use crate::{Arguments, BodyBuf, Storable};

use super::ArrayWriter;

/// Write a typed struct.
///
/// See [`BodyBuf::store_struct`].
///
/// [`BodyBuf::store_struct`]: crate::BodyBuf::store_struct
#[must_use = "Must call `finish` after writing all related fields"]
pub struct StructWriter<'a, T> {
    buf: &'a mut BodyBuf,
    _marker: PhantomData<T>,
}

impl<'a, T> StructWriter<'a, T> {
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
    pub fn store(
        self,
        value: <T::First as ty::Marker>::Return<'_>,
    ) -> StructWriter<'a, T::Remaining>
    where
        T: ty::Fields,
        T::First: ty::Marker,
        for<'b> <T::First as ty::Marker>::Return<'b>: Storable,
    {
        value.store_to(self.buf);
        StructWriter::inner(self.buf)
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
    pub fn store_array<W, U>(self, writer: W) -> StructWriter<'a, T::Remaining>
    where
        W: FnOnce(&mut ArrayWriter<'_, U>),
        T: ty::Fields<First = ty::Array<U>>,
        U: ty::Aligned,
    {
        let mut w = ArrayWriter::new(self.buf);
        writer(&mut w);
        w.finish();
        StructWriter::inner(self.buf)
    }

    /// Store a value and return the builder for the next value to store.
    ///
    /// See [`BodyBuf::store_struct`].
    ///
    /// [`BodyBuf::store_struct`]: crate::BodyBuf::store_struct
    #[inline]
    pub fn store_struct<W>(self, writer: W) -> StructWriter<'a, T::Remaining>
    where
        W: FnOnce(&mut StructWriter<'_, T::First>),
        T: ty::Fields,
        T::First: ty::Fields,
    {
        let mut w = StructWriter::new(self.buf);
        writer(&mut w);
        StructWriter::inner(self.buf)
    }
}

impl StructWriter<'_, ()> {
    /// Finish writing the struct.
    ///
    /// See [`BodyBuf::store_struct`].
    ///
    /// [`BodyBuf::store_struct`]: crate::BodyBuf::store_struct
    pub fn finish(self) {}
}
