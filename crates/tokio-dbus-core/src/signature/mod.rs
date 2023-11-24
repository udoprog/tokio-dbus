#[macro_use]
mod stack;

#[cfg(test)]
mod tests;

use self::validation::validate;
mod validation;

pub use self::signature::Signature;
mod signature;

pub use self::signature_builder::SignatureBuilder;
mod signature_builder;

pub use self::signature_buf::SignatureBuf;
mod signature_buf;

use self::signature_error::SignatureErrorKind;
pub use self::signature_error::SignatureError;
mod signature_error;

/// The maximum size of a signature.
#[doc(hidden)]
pub const MAX_SIGNATURE: usize = 256;

/// The maximum individual container depth.
#[doc(hidden)]
pub const MAX_CONTAINER_DEPTH: usize = 32;

/// The maximum total depth of any containers.
#[doc(hidden)]
pub const MAX_DEPTH: usize = MAX_CONTAINER_DEPTH * 2;
