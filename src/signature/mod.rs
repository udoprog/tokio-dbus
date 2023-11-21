pub use self::owned_signature::OwnedSignature;
mod owned_signature;

pub use self::signature::Signature;
mod signature;

pub use self::signature_error::SignatureError;
pub(crate) use self::signature_error::SignatureErrorKind;
mod signature_error;

#[cfg(test)]
mod tests;
