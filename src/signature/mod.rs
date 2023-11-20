use std::borrow::Borrow;
use std::error;
use std::fmt;
use std::ops::Deref;
use std::str::from_utf8_unchecked;

#[cfg(test)]
mod tests;

use crate::protocol::Type;
use crate::stack::{Stack, StackValue};
use crate::{Error, OwnedBuf, Read, ReadBuf, Write};

/// The maximum individual container depth.
const MAX_CONTAINER_DEPTH: usize = 32;

/// The maximum total depth of any containers.
const MAX_DEPTH: usize = MAX_CONTAINER_DEPTH * 2;

/// Detailed errors raised when validation of a [`Signature`] fails.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignatureError {
    UnknownTypeCode,
    SignatureTooLong,
    MissingArrayElementType,
    StructEndedButNotStarted,
    DictEndedButNotStarted,
    StructStartedButNotEnded,
    DictStartedButNotEnded,
    StructHasNoFields,
    DictKeyMustBeBasicType,
    DictEntryHasNoFields,
    DictEntryHasOnlyOneField,
    DictEntryNotInsideArray,
    ExceededMaximumArrayRecursion,
    ExceededMaximumStructRecursion,
    ExceededMaximumDictRecursion,
    DictEntryHasTooManyFields,
}

impl fmt::Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bad signature")
    }
}

impl error::Error for SignatureError {}

/// A D-Bus signature.
///
/// This is the owned variant which dereferences to [`Signature`].
#[derive(Clone, PartialEq, Eq)]
pub struct OwnedSignature(Vec<u8>);

impl OwnedSignature {
    /// An empty owned signature.
    pub const EMPTY: Self = OwnedSignature(Vec::new());
}

impl fmt::Debug for OwnedSignature {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("OwnedSignature")
            .field(&self.as_str())
            .finish()
    }
}

impl Deref for OwnedSignature {
    type Target = Signature;

    fn deref(&self) -> &Self::Target {
        // Construction of OwnedSignature ensures that the signature is valid.
        unsafe { Signature::new_unchecked(&self.0) }
    }
}

impl Borrow<Signature> for OwnedSignature {
    #[inline]
    fn borrow(&self) -> &Signature {
        self
    }
}

/// A D-Bus signature.
///
/// # Examples
///
/// ```
/// use tokio_dbus::Signature;
///
/// const SIG: &Signature = Signature::new_const(b"aaaai");
///
/// assert!(Signature::new(b"aai").is_ok());
/// ```
#[derive(Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct Signature([u8]);

impl Signature {
    /// The empty signature.
    pub const EMPTY: &'static Signature = Signature::new_const(b"");
    /// The signature of a stored signature.
    pub const SIGNATURE: &'static Signature = Signature::new_const(b"g");
    /// A simple object path.
    pub const OBJECT_PATH: &'static Signature = Signature::new_const(b"o");
    /// A single string.
    pub const STRING: &'static Signature = Signature::new_const(b"s");
    /// A single uint32.
    pub const UINT32: &'static Signature = Signature::new_const(b"u");

    /// Construct a new empty signature.
    pub const fn empty() -> &'static Self {
        unsafe { Self::new_unchecked(&[]) }
    }

    /// Test if the signature is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Construct a new signature with validation inside of a constant context.
    ///
    /// This will panic in case the signature is invalid.
    ///
    /// ```compile_fail
    /// use tokio_dbus::Signature;
    ///
    /// const BAD: &Signature = Signature::new_const(b"(a)");
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::Signature;
    ///
    /// const SIG: &Signature = Signature::new_const(b"i(ai)");
    /// ```
    #[inline]
    #[track_caller]
    pub const fn new_const(signature: &[u8]) -> &Signature {
        if validate(signature).is_err() {
            panic!("Invalid D-Bus signature")
        };

        // SAFETY: The byte slice is repr transparent over this type.
        unsafe { Self::new_unchecked(signature) }
    }

    /// Try to construct a new signature with validation.
    #[inline]
    pub const fn new(signature: &[u8]) -> Result<&Signature, SignatureError> {
        if let Err(error) = validate(signature) {
            return Err(error);
        };

        // SAFETY: The byte slice is repr transparent over this type.
        unsafe { Ok(Self::new_unchecked(signature)) }
    }

    /// Construct a new signature without validation. The caller is responsible
    /// for ensuring that the signature is valid.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the signature is a valid signature.
    #[inline]
    const unsafe fn new_unchecked(signature: &[u8]) -> &Self {
        &*(signature as *const _ as *const Signature)
    }

    /// Get the signature as a string.
    pub(crate) fn as_str(&self) -> &str {
        // SAFETY: Validation indirectly ensures that the signature is valid UTF-8.
        unsafe { from_utf8_unchecked(&self.0) }
    }

    /// Get the signature as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Signature {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Signature").field(&self.as_str()).finish()
    }
}

impl Write for Signature {
    #[inline]
    fn write_to(&self, buf: &mut OwnedBuf) {
        buf.store(self.0.len() as u8);
        buf.extend_from_slice_nul(&self.0);
    }
}

impl Read for Signature {
    #[inline]
    fn read_from<'de>(buf: &mut ReadBuf<'de>) -> Result<&'de Self, Error> {
        let len = buf.load::<u8>()? as usize;
        let bytes = buf.load_slice_nul(len)?;
        Ok(Signature::new(bytes)?)
    }
}

impl ToOwned for Signature {
    type Owned = OwnedSignature;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        OwnedSignature(self.0.to_owned())
    }
}

/// Equality check between [`Signature`] and [`OwnedSignature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, OwnedSignature};
///
/// assert_eq!(OwnedSignature::EMPTY, *Signature::EMPTY);
/// assert_eq!(Signature::STRING.to_owned(), *Signature::STRING);
/// ```
impl PartialEq<Signature> for OwnedSignature {
    #[inline]
    fn eq(&self, other: &Signature) -> bool {
        self.0 == other.0
    }
}

/// Equality check between a borrowed [`Signature`] and [`OwnedSignature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, OwnedSignature};
///
/// assert_eq!(OwnedSignature::EMPTY, *Signature::EMPTY);
/// assert_eq!(Signature::STRING.to_owned(), *Signature::STRING);
/// ```
impl PartialEq<&Signature> for OwnedSignature {
    #[inline]
    fn eq(&self, other: &&Signature) -> bool {
        self.0 == other.0
    }
}

/// Equality check between [`OwnedSignature`] and [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, OwnedSignature};
///
/// assert_eq!(*Signature::EMPTY, OwnedSignature::EMPTY);
/// assert_eq!(*Signature::STRING, Signature::STRING.to_owned());
/// ```
impl PartialEq<OwnedSignature> for Signature {
    #[inline]
    fn eq(&self, other: &OwnedSignature) -> bool {
        self.0 == other.0
    }
}

/// Equality check between [`OwnedSignature`] and a borrowed [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, OwnedSignature};
///
/// assert_eq!(Signature::EMPTY, OwnedSignature::EMPTY);
/// assert_eq!(Signature::STRING, Signature::STRING.to_owned());
/// ```
impl PartialEq<OwnedSignature> for &Signature {
    #[inline]
    fn eq(&self, other: &OwnedSignature) -> bool {
        self.0 == other.0
    }
}

#[derive(Default, Debug, Clone, Copy)]
#[repr(u8)]
enum Kind {
    #[default]
    None,
    Array,
    Struct,
    Dict,
}

impl StackValue for (Kind, u8) {
    const DEFAULT: Self = (Kind::None, 0);
}

const fn validate(bytes: &[u8]) -> Result<(), SignatureError> {
    if bytes.len() > u8::MAX as usize {
        return Err(SignatureError::SignatureTooLong);
    }

    let mut stack = Stack::<(Kind, u8), MAX_DEPTH>::new();
    let mut arrays = 0;
    let mut structs = 0;
    let mut n = 0;

    while n < bytes.len() {
        let b = bytes[n];
        n += 1;
        let t = Type(b);

        let mut is_basic = match t {
            Type::BYTE => true,
            Type::BOOLEAN => true,
            Type::INT16 => true,
            Type::UINT16 => true,
            Type::INT32 => true,
            Type::UINT32 => true,
            Type::INT64 => true,
            Type::UINT64 => true,
            Type::DOUBLE => true,
            Type::STRING => true,
            Type::OBJECT_PATH => true,
            Type::SIGNATURE => true,
            Type::VARIANT => true,
            Type::UNIX_FD => true,
            Type::ARRAY => {
                if !stack_try_push!(stack, (Kind::Array, 0)) || arrays == MAX_CONTAINER_DEPTH {
                    return Err(SignatureError::ExceededMaximumArrayRecursion);
                }

                arrays += 1;
                continue;
            }
            Type::OPEN_PAREN => {
                if !stack_try_push!(stack, (Kind::Struct, 0)) || structs == MAX_CONTAINER_DEPTH {
                    return Err(SignatureError::ExceededMaximumStructRecursion);
                }

                structs += 1;
                continue;
            }
            Type::CLOSE_PAREN => {
                let n = match stack_pop!(stack, (Kind, u8)) {
                    Some((Kind::Struct, n)) => n,
                    Some((Kind::Array, _)) => {
                        return Err(SignatureError::MissingArrayElementType);
                    }
                    _ => {
                        return Err(SignatureError::StructEndedButNotStarted);
                    }
                };

                if n == 0 {
                    return Err(SignatureError::StructHasNoFields);
                }

                structs -= 1;
                false
            }
            Type::OPEN_BRACE => {
                if !stack_try_push!(stack, (Kind::Dict, 0)) {
                    return Err(SignatureError::ExceededMaximumDictRecursion);
                }

                continue;
            }
            Type::CLOSE_BRACE => {
                let n = match stack_pop!(stack, (Kind, u8)) {
                    Some((Kind::Dict, n)) => n,
                    Some((Kind::Array, _)) => {
                        return Err(SignatureError::MissingArrayElementType);
                    }
                    _ => {
                        return Err(SignatureError::DictEndedButNotStarted);
                    }
                };

                match n {
                    0 => {
                        return Err(SignatureError::DictEntryHasNoFields);
                    }
                    1 => {
                        return Err(SignatureError::DictEntryHasOnlyOneField);
                    }
                    2 => {}
                    _ => {
                        return Err(SignatureError::DictEntryHasTooManyFields);
                    }
                }

                if !matches!(stack_peek!(stack), Some((Kind::Array, _))) {
                    return Err(SignatureError::DictEntryNotInsideArray);
                }

                false
            }
            _ => return Err(SignatureError::UnknownTypeCode),
        };

        while let Some((Kind::Array, _)) = stack_peek!(stack) {
            stack_pop!(stack, (Kind, u8));
            is_basic = false;
        }

        if let Some((Kind::Dict, 0)) = stack_peek!(stack) {
            if !is_basic {
                return Err(SignatureError::DictKeyMustBeBasicType);
            }
        }

        if let Some((kind, n)) = stack_pop!(stack, (Kind, u8)) {
            stack_try_push!(stack, (kind, n + 1));
        }
    }

    match stack_pop!(stack, (Kind, u8)) {
        Some((Kind::Array, _)) => {
            return Err(SignatureError::MissingArrayElementType);
        }
        Some((Kind::Struct, _)) => {
            return Err(SignatureError::StructStartedButNotEnded);
        }
        Some((Kind::Dict, _)) => {
            return Err(SignatureError::DictStartedButNotEnded);
        }
        _ => {}
    }

    Ok(())
}
