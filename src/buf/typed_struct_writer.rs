use std::marker::PhantomData;

use crate::buf::{BufMut, StructWriter, TypedArrayWriter};
use crate::ty;
use crate::{Frame, Write};

/// Write a typed struct.
pub struct TypedStructWriter<'a, O, E>
where
    O: BufMut,
{
    inner: StructWriter<'a, O>,
    _marker: PhantomData<E>,
}

impl<'a, O, E> TypedStructWriter<'a, O, E>
where
    O: BufMut,
{
    pub(super) fn new(inner: StructWriter<'a, O>) -> Self {
        Self {
            inner,
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
    /// buf.write_struct::<(u16, u32)>()
    ///     .store(10u16)
    ///     .store(10u32)
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"(qu)");
    /// assert_eq!(buf.get(), &[10, 0, 0, 0, 10, 0, 0, 0]);
    /// ```
    #[inline]
    pub fn store(mut self, value: E::First) -> TypedStructWriter<'a, O, E::Remaining>
    where
        E: ty::Fields,
        E::First: Frame,
    {
        self.inner.store(value);
        TypedStructWriter::new(self.inner)
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
    /// buf.write_struct::<(ty::Str,)>()
    ///     .write("Hello World")
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"(s)");
    /// assert_eq!(buf.get(), &[11, 0, 0, 0, 72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 0]);
    /// ```
    #[inline]
    pub fn write(
        mut self,
        value: &<E::First as ty::Unsized>::Target,
    ) -> TypedStructWriter<'a, O, E::Remaining>
    where
        E: ty::Fields,
        E::First: ty::Unsized,
        <E::First as ty::Unsized>::Target: Write,
    {
        self.inner.write(value);
        TypedStructWriter::new(self.inner)
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
    /// buf.write_struct::<(ty::Array<u32>,)>()
    ///     .write_array(|w| {
    ///         w.store(1);
    ///         w.store(2);
    ///         w.store(3);
    ///         w.store(4);
    ///     })
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"(au)");
    /// assert_eq!(buf.get(), &[16, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]);
    /// ```
    #[inline]
    pub fn write_array<W, T>(mut self, writer: W) -> TypedStructWriter<'a, O, E::Remaining>
    where
        W: FnOnce(&mut TypedArrayWriter<'_, O, T>),
        E: ty::Fields<First = ty::Array<T>>,
    {
        let mut w = TypedArrayWriter::new(self.inner.write_array());
        writer(&mut w);
        w.finish();
        TypedStructWriter::new(self.inner)
    }

    /// Store a value and return the builder for the next value to store.
    #[inline]
    pub fn write_struct<W>(mut self, writer: W) -> TypedStructWriter<'a, O, E::Remaining>
    where
        W: FnOnce(&mut TypedStructWriter<'_, O, E::First>),
        E: ty::Fields,
        E::First: ty::Fields,
    {
        let mut w = TypedStructWriter::new(self.inner.write_struct());
        writer(&mut w);
        TypedStructWriter::new(self.inner)
    }
}

impl<'a, O> TypedStructWriter<'a, O, ()>
where
    O: BufMut,
{
    /// Finish writing the struct.
    pub fn finish(self) {}
}
