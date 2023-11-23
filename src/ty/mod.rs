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
mod marker;

pub use self::aligned::Aligned;
mod aligned;

use std::marker::PhantomData;

/// The [`Marker`] for the [`str`] type.
///
/// [`Signature`]: crate::Signature
#[non_exhaustive]
pub struct Str;

/// The [`Marker`] for the [`Signature`] type.
///
/// [`Signature`]: crate::Signature
#[non_exhaustive]
pub struct Sig;

/// The [`Marker`] for the [`ObjectPath`] type.
///
/// [`ObjectPath`]: crate::ObjectPath
#[non_exhaustive]
pub struct O;

/// The [`Marker`] for an array type, like `[u8]`.
pub struct Array<T>(PhantomData<T>);

/// The [`Marker`] for the [`Variant`] type.
#[non_exhaustive]
pub struct Var;
