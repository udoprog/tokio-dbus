//! Marker types used for writing type-checked D-Bus bodies.
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
//! buf.write_struct::<(u16, u32, ty::Array<u8>, ty::Str)>()?
//!     .store(10u16)
//!     .store(10u32)
//!     .write_array(|w| {
//!         w.store(1u8);
//!         w.store(2u8);
//!         w.store(3u8);
//!     })
//!     .write("Hello World")
//!     .finish();
//!
//! assert_eq!(buf.signature(), b"y(quays)");
//! assert_eq!(buf.get(), &[10, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 10, 0, 0, 0, 3, 0, 0, 0, 1, 2, 3, 0, 11, 0, 0, 0, 72, 101, 108, 108, 111, 32, 87, 111, 114, 108, 100, 0]);
//! # Ok::<_, tokio_dbus::Error>(())
//! ```

use std::marker::PhantomData;

use crate::signature::{SignatureBuilder, SignatureError, SignatureErrorKind};
use crate::{Frame, Signature};

/// A marker that is unsized.
pub trait Unsized {
    /// The unsized target.
    type Target: ?Sized;
}

/// The trait implementation for a type marker.
pub trait Marker {
    /// Writing the signature.
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError>;
}

impl<T> Marker for T
where
    T: Frame,
{
    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        if !signature.extend_from_signature(T::SIGNATURE) {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        Ok(())
    }
}

/// The [`Marker`] for the empty type.
#[non_exhaustive]
pub enum Empty {}

/// The [`Marker`] for the [`str`] type.
///
/// [`Signature`]: crate::Signature
#[non_exhaustive]
pub struct Str;

impl Unsized for Str {
    type Target = str;
}

impl Marker for Str {
    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        if !signature.extend_from_signature(Signature::STRING) {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        Ok(())
    }
}

/// The [`Marker`] for the [`Signature`] type.
///
/// [`Signature`]: crate::Signature
#[non_exhaustive]
pub struct Sig;

impl Unsized for Sig {
    type Target = Signature;
}

impl Marker for Sig {
    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        if !signature.extend_from_signature(Signature::SIGNATURE) {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        Ok(())
    }
}

/// Type marker for the fields in a struct.
///
/// This is implemented by tuples.
pub trait Fields {
    /// The target field.
    type First;

    /// The next struct fields to write.
    type Remaining;

    /// Write signature.
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError>;
}

/// An array marker type.
pub struct Array<T>(PhantomData<T>);

impl<T> Marker for Array<T>
where
    T: Marker,
{
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        signature.open_array()?;
        T::write_signature(signature)?;
        signature.close_array();
        Ok(())
    }
}

impl Fields for () {
    type First = Empty;
    type Remaining = ();

    #[inline]
    fn write_signature(_: &mut SignatureBuilder) -> Result<(), SignatureError> {
        Ok(())
    }
}

macro_rules! struct_fields {
    ($first:ident $(, $rest:ident)*) => {
        impl<$first, $($rest),*> Fields for ($first, $($rest),*)
        where
            $first: Marker,
            $($rest: Marker,)*
        {
            type First = A;
            type Remaining = ($($rest,)*);

            #[inline]
            fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
                signature.open_struct()?;
                <$first>::write_signature(signature)?;
                $(<$rest>::write_signature(signature)?;)*
                signature.close_struct()?;
                Ok(())
            }
        }
    }
}

macro_rules! repeat {
    ($macro:path) => {
        $macro!(A);
        $macro!(A, B);
        $macro!(A, B, C);
        $macro!(A, B, C, D);
        $macro!(A, B, C, D, E);
        $macro!(A, B, C, D, E, F);
        $macro!(A, B, C, D, E, F, G);
        $macro!(A, B, C, D, E, F, G, H);
    };
}

repeat!(struct_fields);
