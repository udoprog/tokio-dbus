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

use std::marker::PhantomData;

use crate::buf::ArrayReader;
use crate::error::ErrorKind;
use crate::signature::{SignatureBuilder, SignatureErrorKind};
use crate::{Body, Error, ObjectPath, Result, Signature, SignatureError, Variant};

/// The [`Marker`] for the [`str`] type.
///
/// [`Signature`]: crate::Signature
#[non_exhaustive]
pub struct Str;

impl_trait_unsized_marker!(Str, u32, str, STRING);

/// The [`Marker`] for the [`Signature`] type.
///
/// [`Signature`]: crate::Signature
#[non_exhaustive]
pub struct Sig;

impl_trait_unsized_marker!(Sig, u8, Signature, SIGNATURE);

/// The [`Marker`] for the [`ObjectPath`] type.
///
/// [`ObjectPath`]: crate::ObjectPath
#[non_exhaustive]
pub struct O;

impl_trait_unsized_marker!(O, u8, ObjectPath, OBJECT_PATH);

/// The [`Marker`] for an array type, like `[u8]`.
pub struct Array<T>(PhantomData<T>);

impl<T> self::aligned::sealed::Sealed for Array<T> where T: Aligned {}

impl<T> Aligned for Array<T>
where
    T: Aligned,
{
    type Alignment = T;
}

impl<T> self::marker::sealed::Sealed for Array<T> where T: Marker {}

impl<T> Marker for Array<T>
where
    T: Marker,
{
    type Return<'de> = ArrayReader<'de, T>;

    #[inline]
    fn read_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        buf.read_array::<T>()
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        signature.open_array()?;
        T::write_signature(signature)?;
        signature.close_array();
        Ok(())
    }
}

/// The [`Marker`] for the [`Variant`] type.
#[non_exhaustive]
pub struct Var;

impl self::aligned::sealed::Sealed for Var {}

impl Aligned for Var {
    type Alignment = u32;
}

impl self::marker::sealed::Sealed for Var {}

impl Marker for Var {
    type Return<'de> = Variant<'de>;

    #[inline]
    fn read_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        let signature: &Signature = buf.read()?;

        let variant = match signature.as_bytes() {
            b"s" => Variant::String(buf.read()?),
            b"u" => Variant::U32(buf.load()?),
            _ => {
                return Err(Error::new(ErrorKind::UnsupportedVariant(signature.into())));
            }
        };

        Ok(variant)
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        if !signature.extend_from_signature(Signature::VARIANT) {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        Ok(())
    }
}
