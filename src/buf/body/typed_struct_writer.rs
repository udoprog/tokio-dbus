use std::marker::PhantomData;

use crate::buf::BufMut;
use crate::{ty, Arguments};
use crate::{Frame, Result, Write};

use super::{StructWriter, TypedArrayWriter};

/// Write a typed struct.
///
/// See [`BodyBuf::write_struct`].
///
/// [`BodyBuf::write_struct`]: crate::BodyBuf::write_struct
#[must_use = "Must call `finish` after writing all related fields"]
pub struct TypedStructWriter<'a, B, E>
where
    B: BufMut,
{
    inner: StructWriter<'a, B>,
    _marker: PhantomData<E>,
}

impl<'a, B, E> TypedStructWriter<'a, B, E>
where
    B: BufMut,
{
    pub(crate) fn new(inner: StructWriter<'a, B>) -> Self {
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
    /// buf.write_struct::<(u16, u32)>()?
    ///     .store(10u16)?
    ///     .store(10u32)?
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"(qu)");
    /// assert_eq!(buf.get(), &[10, 0, 0, 0, 10, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn store(mut self, value: E::First) -> Result<TypedStructWriter<'a, B, E::Remaining>>
    where
        E: ty::Fields,
        E::First: Frame,
    {
        self.inner.store(value)?;
        Ok(TypedStructWriter::new(self.inner))
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
    /// buf.write_struct::<(ty::Str,)>()?
    ///     .write("Hello World")?
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"(s)");
    /// assert_eq!(buf.get(), &[11, 0, 0, 0, 72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn write(
        mut self,
        value: &<E::First as ty::Unsized>::Target,
    ) -> Result<TypedStructWriter<'a, B, E::Remaining>>
    where
        E: ty::Fields,
        E::First: ty::Unsized,
        <E::First as ty::Unsized>::Target: Write,
    {
        self.inner.write(value)?;
        Ok(TypedStructWriter::new(self.inner))
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
    /// buf.write_struct::<(u8, u32)>()?.fields((42u8, 42u32));
    ///
    /// assert_eq!(buf.signature(), b"(yu)");
    /// assert_eq!(buf.get(), &[42, 0, 0, 0, 42, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn fields(mut self, arguments: E) -> Result<()>
    where
        E: Arguments,
    {
        self.inner.extend(arguments)
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
    /// buf.write_struct::<(ty::Array<u32>,)>()?
    ///     .write_array(|w| {
    ///         w.store(1)?;
    ///         w.store(2)?;
    ///         w.store(3)?;
    ///         w.store(4)?;
    ///         Ok(())
    ///     })?
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), b"(au)");
    /// assert_eq!(buf.get(), &[16, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn write_array<W, T>(mut self, writer: W) -> Result<TypedStructWriter<'a, B, E::Remaining>>
    where
        W: FnOnce(&mut TypedArrayWriter<'_, B, T>) -> Result<()>,
        E: ty::Fields<First = ty::Array<T>>,
        T: ty::Aligned,
    {
        let mut w = TypedArrayWriter::new(self.inner.write_array());
        writer(&mut w)?;
        w.finish();
        Ok(TypedStructWriter::new(self.inner))
    }

    /// Store a value and return the builder for the next value to store.
    ///
    /// See [`BodyBuf::write_struct`].
    ///
    /// [`BodyBuf::write_struct`]: crate::BodyBuf::write_struct
    #[inline]
    pub fn write_struct<W>(mut self, writer: W) -> TypedStructWriter<'a, B, E::Remaining>
    where
        W: FnOnce(&mut TypedStructWriter<'_, B, E::First>),
        E: ty::Fields,
        E::First: ty::Fields,
    {
        let mut w = TypedStructWriter::new(self.inner.write_struct());
        writer(&mut w);
        TypedStructWriter::new(self.inner)
    }
}

impl<B> TypedStructWriter<'_, B, ()>
where
    B: BufMut,
{
    /// Finish writing the struct.
    ///
    /// See [`BodyBuf::write_struct`].
    ///
    /// [`BodyBuf::write_struct`]: crate::BodyBuf::write_struct
    pub fn finish(self) {}
}
