#[macro_use]
mod stack;

use self::validation::validate;
mod validation;

pub use self::signature_buf::SignatureBuf;
mod signature_buf;

pub use self::signature::Signature;
mod signature;

pub use self::signature_error::SignatureError;
pub(crate) use self::signature_error::SignatureErrorKind;
mod signature_error;

pub(crate) use self::signature_builder::SignatureBuilder;
mod signature_builder;

#[cfg(test)]
mod tests;

/// The maximum size of a signature.
const MAX_SIGNATURE: usize = 256;

/// The maximum individual container depth.
const MAX_CONTAINER_DEPTH: usize = 32;

/// The maximum total depth of any containers.
const MAX_DEPTH: usize = MAX_CONTAINER_DEPTH * 2;
