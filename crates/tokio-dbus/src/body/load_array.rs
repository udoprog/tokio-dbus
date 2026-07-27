use core::marker::PhantomData;

use crate::buf::MAX_ARRAY_LENGTH;
use crate::error::ErrorKind;
use crate::{Body, ty};
use crate::{Error, Frame, Read, Result};

/// Read an array from a buffer.
///
/// See [`Body::load_array`].
///
/// [`Body::load_array`]: crate::Body::load_array
pub struct LoadArray<'de, T> {
    buf: Body<'de>,
    _marker: PhantomData<T>,
}

impl<'de, T> LoadArray<'de, T>
where
    T: ty::Aligned,
{
    #[inline]
    pub(crate) fn from_mut(buf: &mut Body<'de>) -> Result<LoadArray<'de, T>> {
        let bytes = buf.load::<u32>()?;

        if bytes > MAX_ARRAY_LENGTH {
            return Err(Error::new(ErrorKind::ArrayTooLong(bytes)));
        }

        // NB: The length prefix is followed by padding up to the alignment of
        // the element type, which is present even if the array is empty and is
        // not counted towards the encoded length.
        buf.align::<T::Alignment>()?;

        let buf = buf.read_until(bytes as usize);
        Ok(LoadArray::new(buf))
    }
}

impl<'de, T> LoadArray<'de, T> {
    /// Construct a new array reader around a buffer.
    pub(crate) fn new(buf: Body<'de>) -> Self {
        LoadArray {
            buf,
            _marker: PhantomData,
        }
    }

    /// Test if the array has been fully consumed.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::BodyBuf;
    ///
    /// let mut buf = BodyBuf::new();
    /// let mut array = buf.store_array::<u32>()?;
    /// array.store(10u32);
    /// array.finish();
    ///
    /// let mut buf = buf.as_body();
    /// let mut array = buf.load_array::<u32>()?;
    /// assert!(!array.is_empty());
    /// assert_eq!(array.load()?, Some(10));
    /// assert!(array.is_empty());
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Read the next element of the array with the given closure.
    ///
    /// This is an escape hatch for element types which the typed readers cannot
    /// describe, and hands the closure a [`Body`] positioned at the start of the
    /// next element.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Signature};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// let mut array = buf.store_array::<(u32, ty::Variant)>()?;
    ///
    /// array
    ///     .store_struct()
    ///     .store(42u32)
    ///     .store_variant(Signature::new("as")?, |w| {
    ///         w.store_array::<ty::Str>().store("Hello");
    ///     })
    ///     .finish();
    ///
    /// array.finish();
    ///
    /// let mut buf = buf.as_body();
    /// let mut array = buf.load_array::<(u32, ty::Variant)>()?;
    ///
    /// let element = array.load_with(|b| {
    ///     b.load_struct_with(|b| {
    ///         let n = b.load::<u32>()?;
    ///         b.skip_variant()?;
    ///         Ok(n)
    ///     })
    /// })?;
    ///
    /// assert_eq!(element, Some(42));
    /// assert!(array.is_empty());
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn load_with<F, O>(&mut self, f: F) -> Result<Option<O>>
    where
        F: FnOnce(&mut Body<'de>) -> Result<O>,
    {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(f(&mut self.buf)?))
    }
}

impl<T> LoadArray<'_, T>
where
    T: Frame,
{
    /// Load the next value from the array.
    ///
    /// See [`Body::load_array`].
    ///
    /// [`Body::load_array`]: crate::Body::load_array
    pub fn load(&mut self) -> Result<Option<T>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(self.buf.load()?))
    }
}

impl<'de, T> LoadArray<'de, T>
where
    T: ty::Unsized,
    T::Target: Read,
{
    /// Read the next value from the array.
    ///
    /// See [`Body::load_array`].
    ///
    /// [`Body::load_array`]: crate::Body::load_array
    pub fn read(&mut self) -> Result<Option<&'de T::Target>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(T::Target::read_from(&mut self.buf)?))
    }
}

impl<'de, T> LoadArray<'de, ty::Array<T>>
where
    T: ty::Marker,
{
    /// Read an array from within the array.
    ///
    /// See [`Body::load_struct`].
    pub fn load_array(&mut self) -> Result<Option<LoadArray<'de, T>>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(LoadArray::from_mut(&mut self.buf)?))
    }
}

impl<'de, T> LoadArray<'de, T>
where
    T: ty::Fields,
{
    /// Read a struct from within the array.
    ///
    /// See [`Body::load_struct`].
    pub fn load_struct(&mut self) -> Result<Option<T::Return<'de>>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(self.buf.load_struct::<T>()?))
    }
}

impl<'de> LoadArray<'de, ty::Variant> {
    /// Read the next variant from the array.
    ///
    /// See [`StoreArray::store_variant`].
    ///
    /// [`StoreArray::store_variant`]: crate::StoreArray::store_variant
    pub fn load_variant(&mut self) -> Result<Option<crate::Variant<'de>>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(self.buf.read_variant()?))
    }

    /// Skip over the next variant in the array, returning the signature of the
    /// value it contained.
    ///
    /// See [`Body::skip_variant`].
    ///
    /// [`Body::skip_variant`]: crate::Body::skip_variant
    pub fn skip_variant(&mut self) -> Result<Option<&'de crate::Signature>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        Ok(Some(self.buf.skip_variant()?))
    }
}

impl<'de, K, V> LoadArray<'de, ty::Dict<K, V>>
where
    K: ty::Marker,
    V: ty::Marker,
{
    /// Read a dict entry from within the array.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// let mut dict = buf.store_array::<ty::Dict<ty::Str, u32>>()?;
    /// dict.store_entry().store("a").store(1u32).finish();
    /// dict.store_entry().store("b").store(2u32).finish();
    /// dict.finish();
    ///
    /// assert_eq!(buf.signature(), "a{su}");
    ///
    /// let mut buf = buf.as_body();
    /// let mut dict = buf.load_array::<ty::Dict<ty::Str, u32>>()?;
    ///
    /// assert_eq!(dict.load_entry()?, Some(("a", 1)));
    /// assert_eq!(dict.load_entry()?, Some(("b", 2)));
    /// assert_eq!(dict.load_entry()?, None);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn load_entry(&mut self) -> Result<Option<(K::Return<'de>, V::Return<'de>)>> {
        if self.buf.is_empty() {
            return Ok(None);
        }

        // NB: Dict entries are aligned just like structs.
        self.buf.align::<u64>()?;
        Ok(Some((
            K::load_struct(&mut self.buf)?,
            V::load_struct(&mut self.buf)?,
        )))
    }
}

impl<'de, K> LoadArray<'de, ty::Dict<K, ty::Variant>>
where
    K: ty::Marker,
{
    /// Read a dict entry whose value is a variant expected to contain a value
    /// of type `T`.
    ///
    /// This is needed for dictionaries like the ones returned by
    /// `org.freedesktop.DBus.Properties.GetAll`, where the value of an entry is
    /// a container which [`load_entry()`] cannot represent.
    ///
    /// [`load_entry()`]: Self::load_entry
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Signature};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// let mut dict = buf.store_array::<ty::Dict<ty::Str, ty::Variant>>()?;
    ///
    /// dict.store_entry()
    ///     .store("IconThemePath")
    ///     .store_variant(Signature::new("as")?, |w| {
    ///         let mut array = w.store_array::<ty::Str>();
    ///         array.store("/usr/share/icons");
    ///     })
    ///     .finish();
    ///
    /// dict.finish();
    ///
    /// let mut buf = buf.as_body();
    /// let mut dict = buf.load_array::<ty::Dict<ty::Str, ty::Variant>>()?;
    ///
    /// let Some((key, mut value)) = dict.load_entry_as::<ty::Array<ty::Str>>()? else {
    ///     panic!("Missing entry");
    /// };
    ///
    /// assert_eq!(key, "IconThemePath");
    /// assert_eq!(value.read()?, Some("/usr/share/icons"));
    /// assert_eq!(value.read()?, None);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn load_entry_as<T>(&mut self) -> Result<Option<(K::Return<'de>, T::Return<'de>)>>
    where
        T: ty::Marker,
    {
        if self.buf.is_empty() {
            return Ok(None);
        }

        self.buf.align::<u64>()?;
        let key = K::load_struct(&mut self.buf)?;
        let value = self.buf.read_variant_as::<T>()?;
        Ok(Some((key, value)))
    }
}
