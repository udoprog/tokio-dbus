use crate::ty;
use crate::{BodyBuf, Signature, Storable};

use super::{StoreArray, StoreStruct};

/// Write the contents of a variant.
///
/// The signature of the contained value is written when this is constructed,
/// after which exactly one value matching that signature must be stored.
///
/// Unlike the other writers in this crate a variant is written against a
/// signature which is provided at runtime. This is what makes it possible to
/// write recursive types, such as the `(ia{sv}av)` layout of a
/// `com.canonical.dbusmenu` menu.
///
/// See [`BodyBuf::store_variant`].
///
/// [`BodyBuf::store_variant`]: crate::BodyBuf::store_variant
#[must_use = "Writers must be used to store the contents of the variant"]
pub struct StoreVariant<'a> {
    buf: &'a mut BodyBuf,
}

impl<'a> StoreVariant<'a> {
    pub(crate) fn new(buf: &'a mut BodyBuf, signature: &Signature) -> Self {
        // NB: The signature of the contained value is written verbatim and is
        // not part of the signature of the surrounding buffer.
        buf.write_only(signature);
        Self { buf }
    }

    /// Store a value inside of the variant.
    ///
    /// Note that a [`Variant`] must *not* be stored this way, since it writes
    /// its own signature and would end up writing it twice. Store it directly
    /// instead, with [`BodyBuf::store`] or the `store` of the surrounding
    /// container.
    ///
    /// [`Variant`]: crate::Variant
    /// [`BodyBuf::store`]: crate::BodyBuf::store
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature, Variant};
    ///
    /// let mut buf = BodyBuf::new();
    /// buf.store_variant(Signature::STRING)?.store("Hello World!");
    ///
    /// assert_eq!(buf.signature(), Signature::VARIANT);
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.read_variant()?, Variant::String("Hello World!"));
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn store<T>(self, value: T)
    where
        T: Storable,
    {
        value.store_to(self.buf);
    }

    /// Store an array inside of the variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Signature};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// let mut array = buf.store_variant(Signature::new("as")?)?.store_array::<ty::Str>();
    /// array.store("Hello");
    /// array.store("World");
    /// array.finish();
    ///
    /// assert_eq!(buf.signature(), Signature::VARIANT);
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.skip_variant()?, Signature::new("as")?);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn store_array<E>(self) -> StoreArray<'a, E>
    where
        E: ty::Aligned,
    {
        StoreArray::new(self.buf)
    }

    /// Store a struct inside of the variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, BodyBuf, Signature};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// buf.store_variant(Signature::new("(us)")?)?
    ///     .store_struct::<(u32, ty::Str)>()
    ///     .store(42u32)
    ///     .store("Hello World!")
    ///     .finish();
    ///
    /// assert_eq!(buf.signature(), Signature::VARIANT);
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.skip_variant()?, Signature::new("(us)")?);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn store_struct<F>(self) -> StoreStruct<'a, F>
    where
        F: ty::Fields,
    {
        StoreStruct::new(self.buf)
    }
}
