//! Type [`Marker`] for writing to type-checked D-Bus bodies.
//!
//! # Examples
//!
//! ```
//! use tokio_dbus::{BodyBuf, Endianness};
//! use tokio_dbus::ty;
//!
//! let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
//! buf.store(10u8);
//!
//! buf.store_struct::<(u16, u32, ty::Array<u8>, ty::Str)>()?
//!     .store(10u16)
//!     .store(10u32)
//!     .store_array(|w| {
//!         w.store(1u8);
//!         w.store(2u8);
//!         w.store(3u8);
//!     })
//!     .store("Hello World")
//!     .finish();
//!
//! assert_eq!(buf.signature(), b"y(quays)");
//! assert_eq!(buf.get(), &[10, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 10, 0, 0, 0, 3, 0, 0, 0, 1, 2, 3, 0, 11, 0, 0, 0, 72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 0]);
//! # Ok::<_, tokio_dbus::Error>(())
//! ```

pub use self::fields::Fields;
mod fields;

pub use self::r#unsized::Unsized;
mod r#unsized;

pub use self::marker::Marker;
pub(crate) mod marker;

pub use self::aligned::Aligned;
pub(crate) mod aligned;

use core::marker::PhantomData;

use crate::error::ErrorKind;
use crate::signature::SignatureBuilder;
use crate::{Body, Error, LoadArray, Result, SignatureError};

/// The [`Marker`] for the [`str`] type.
///
/// [`Signature`]: crate::Signature
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};
/// use tokio_dbus::ty;
///
/// let mut buf = BodyBuf::new();
///
/// buf.store_struct::<(u8, ty::Str)>()?
///     .store(42u8)
///     .store("Hello World!")
///     .finish();
///
/// assert_eq!(buf.signature(), b"(ys)");
///
/// let mut b = buf.as_body();
///
/// let (n, value) = b.load_struct::<(u8, ty::Str)>()?;
///
/// assert_eq!(n, 42u8);
/// assert_eq!(value, "Hello World!");
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
#[non_exhaustive]
pub struct Str;

impl_trait_unsized_marker!(Str, u32, str, STRING);

/// The [`Marker`] for the [`Signature`] type.
///
/// [`Signature`]: crate::Signature
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};
/// use tokio_dbus::ty;
///
/// let mut buf = BodyBuf::new();
///
/// buf.store_struct::<(u8, ty::Signature)>()?
///     .store(42u8)
///     .store(Signature::new("ay")?)
///     .finish();
///
/// assert_eq!(buf.signature(), b"(yg)");
///
/// let mut b = buf.as_body();
///
/// let (n, value) = b.load_struct::<(u8, ty::Signature)>()?;
///
/// assert_eq!(n, 42u8);
/// assert_eq!(value, Signature::new("ay")?);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
#[non_exhaustive]
pub struct Signature;

impl_trait_unsized_marker!(Signature, u8, crate::Signature, SIGNATURE);

/// The [`Marker`] for the [`ObjectPath`] type.
///
/// [`ObjectPath`]: crate::ObjectPath
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, ObjectPath};
/// use tokio_dbus::ty;
///
/// let mut buf = BodyBuf::new();
///
/// buf.store_struct::<(u8, ty::ObjectPath)>()?
///     .store(42u8)
///     .store(ObjectPath::new("/se/tedro/DBusExample")?)
///     .finish();
///
/// assert_eq!(buf.signature(), b"(yo)");
///
/// let mut b = buf.as_body();
///
/// let (n, value) = b.load_struct::<(u8, ty::ObjectPath)>()?;
///
/// assert_eq!(n, 42u8);
/// assert_eq!(value, ObjectPath::new("/se/tedro/DBusExample")?);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
#[non_exhaustive]
pub struct ObjectPath;

impl_trait_unsized_marker!(ObjectPath, u8, crate::ObjectPath, OBJECT_PATH);

/// The [`Marker`] for an array type, like `[u8]`.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};
/// use tokio_dbus::ty;
///
/// let mut buf = BodyBuf::new();
///
/// buf.store_struct::<(u8, ty::Array<ty::Str>)>()?
///     .store(42u8)
///     .store_array(|w| {
///         w.store("Hello");
///         w.store("World");
///     })
///     .finish();
///
/// assert_eq!(buf.signature(), b"(yas)");
///
/// let mut b = buf.as_body();
///
/// let (n, mut array) = b.load_struct::<(u8, ty::Array<ty::Str>)>()?;
///
/// assert_eq!(n, 42u8);
/// assert_eq!(array.read()?, Some("Hello"));
/// assert_eq!(array.read()?, Some("World"));
/// assert_eq!(array.read()?, None);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
pub struct Array<T>(PhantomData<T>);

impl<T> self::aligned::sealed::Sealed for Array<T> {}

impl<T> Aligned for Array<T> {
    // NB: An array starts with a 32-bit length prefix, regardless of the
    // alignment of its elements.
    type Alignment = u32;
}

impl<T> self::marker::sealed::Sealed for Array<T> where T: Marker {}

impl<T> Marker for Array<T>
where
    T: Marker,
{
    type Return<'de> = LoadArray<'de, T>;

    #[inline]
    fn load_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        buf.load_array::<T>()
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        signature.open_array()?;
        T::write_signature(signature)?;
        signature.close_array()
    }
}

/// The [`Marker`] for a dict entry, which is only legal as the element type of
/// an [`Array`].
///
/// The key `K` must be a basic type.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{ty, BodyBuf};
///
/// let mut buf = BodyBuf::new();
///
/// let mut dict = buf.store_array::<ty::Dict<ty::Str, u32>>()?;
/// dict.store_entry().store("Hello").store(42u32).finish();
/// dict.finish();
///
/// assert_eq!(buf.signature(), "a{su}");
///
/// let mut buf = buf.as_body();
/// let mut dict = buf.load_array::<ty::Dict<ty::Str, u32>>()?;
///
/// assert_eq!(dict.load_entry()?, Some(("Hello", 42)));
/// assert_eq!(dict.load_entry()?, None);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
pub struct Dict<K, V>(PhantomData<(K, V)>);

impl<K, V> self::aligned::sealed::Sealed for Dict<K, V> {}

impl<K, V> Aligned for Dict<K, V> {
    // NB: Dict entries are aligned just like structs.
    type Alignment = u64;
}

impl<K, V> self::marker::sealed::Sealed for Dict<K, V>
where
    K: Marker,
    V: Marker,
{
}

impl<K, V> Marker for Dict<K, V>
where
    K: Marker,
    V: Marker,
{
    type Return<'de> = (K::Return<'de>, V::Return<'de>);

    #[inline]
    fn load_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        buf.align::<u64>()?;
        Ok((K::load_struct(buf)?, V::load_struct(buf)?))
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        signature.open_dict()?;
        K::write_signature(signature)?;
        V::write_signature(signature)?;
        signature.close_dict()?;
        Ok(())
    }
}

/// The [`Marker`] for the D-Bus `BOOLEAN` type, which is marshalled as a 32-bit
/// integer but read and written as a [`bool`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{ty, BodyBuf};
///
/// let mut buf = BodyBuf::new();
///
/// buf.store_struct::<(ty::Bool, ty::Str)>()?
///     .store(true)
///     .store("Hello World!")
///     .finish();
///
/// assert_eq!(buf.signature(), "(bs)");
///
/// let mut buf = buf.as_body();
/// let (enabled, message) = buf.load_struct::<(ty::Bool, ty::Str)>()?;
///
/// assert!(enabled);
/// assert_eq!(message, "Hello World!");
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
#[non_exhaustive]
pub struct Bool;

impl self::aligned::sealed::Sealed for Bool {}

impl Aligned for Bool {
    type Alignment = u32;
}

impl self::marker::sealed::Sealed for Bool {}

impl Marker for Bool {
    type Return<'de> = bool;

    #[inline]
    fn load_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        Ok(buf.load::<u32>()? != 0)
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        if !signature.extend_from_signature(crate::Signature::BOOLEAN) {
            return Err(SignatureError::too_long());
        }

        Ok(())
    }
}

/// The [`Marker`] for the [`Variant`] type.
///
/// [`Variant`]: crate::Variant
#[non_exhaustive]
pub struct Variant;

impl self::aligned::sealed::Sealed for Variant {}

impl Aligned for Variant {
    // NB: A variant starts with its signature, which is prefixed by a single
    // byte holding its length.
    type Alignment = u8;
}

impl self::marker::sealed::Sealed for Variant {}

impl Marker for Variant {
    type Return<'de> = crate::Variant<'de>;

    #[inline]
    fn load_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        let signature: &crate::Signature = buf.read()?;

        let variant = match signature.as_bytes() {
            b"b" => crate::Variant::Bool(buf.load::<u32>()? != 0),
            b"y" => crate::Variant::U8(buf.load()?),
            b"n" => crate::Variant::I16(buf.load()?),
            b"q" => crate::Variant::U16(buf.load()?),
            b"i" => crate::Variant::I32(buf.load()?),
            b"u" => crate::Variant::U32(buf.load()?),
            b"x" => crate::Variant::I64(buf.load()?),
            b"t" => crate::Variant::U64(buf.load()?),
            b"d" => crate::Variant::F64(buf.load()?),
            b"s" => crate::Variant::String(buf.read()?),
            b"o" => crate::Variant::ObjectPath(buf.read()?),
            b"g" => crate::Variant::Signature(buf.read()?),
            #[cfg(feature = "alloc")]
            _ => {
                return Err(Error::new(ErrorKind::UnsupportedVariant(signature.into())));
            }
            #[cfg(not(feature = "alloc"))]
            _ => {
                return Err(Error::new(ErrorKind::UnsupportedVariantNoAlloc));
            }
        };

        Ok(variant)
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        if !signature.extend_from_signature(crate::Signature::VARIANT) {
            return Err(SignatureError::too_long());
        }

        Ok(())
    }
}
