use std::error;
use std::fmt;

use crate::proto::Type;

/// Detailed errors raised when validation of a [`Signature`] fails.
///
/// [`Signature`]: crate::signature::Signature
#[derive(Debug, PartialEq, Eq)]
pub struct SignatureError {
    pub(super) kind: SignatureErrorKind,
}

impl SignatureError {
    /// Construct a new signature error.
    #[doc(hidden)]
    pub(crate) const fn new(kind: SignatureErrorKind) -> Self {
        Self { kind }
    }

    /// Indicate that a signature is too long.
    #[inline]
    pub const fn too_long() -> Self {
        Self::new(SignatureErrorKind::SignatureTooLong)
    }
}

#[allow(missing_docs)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SignatureErrorKind {
    UnknownTypeCode(Type),
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
        match self.kind {
            SignatureErrorKind::UnknownTypeCode(code) => {
                write!(f, "Unknown type code: {code:?}")
            }
            SignatureErrorKind::SignatureTooLong => {
                write!(f, "Signature too long")
            }
            SignatureErrorKind::MissingArrayElementType => {
                write!(f, "Missing array element type")
            }
            SignatureErrorKind::StructEndedButNotStarted => {
                write!(f, "Struct ended but not started")
            }
            SignatureErrorKind::DictEndedButNotStarted => {
                write!(f, "Dict ended but not started")
            }
            SignatureErrorKind::StructStartedButNotEnded => {
                write!(f, "Struct started but not ended")
            }
            SignatureErrorKind::DictStartedButNotEnded => {
                write!(f, "Dict started but not ended")
            }
            SignatureErrorKind::StructHasNoFields => {
                write!(f, "Struct has no fields")
            }
            SignatureErrorKind::DictKeyMustBeBasicType => {
                write!(f, "Dict key must be basic type")
            }
            SignatureErrorKind::DictEntryHasNoFields => {
                write!(f, "Dict entry has no fields")
            }
            SignatureErrorKind::DictEntryHasOnlyOneField => {
                write!(f, "Dict entry has only one field")
            }
            SignatureErrorKind::DictEntryNotInsideArray => {
                write!(f, "Dict entry not inside array")
            }
            SignatureErrorKind::ExceededMaximumArrayRecursion => {
                write!(f, "Exceeded maximum array recursion")
            }
            SignatureErrorKind::ExceededMaximumStructRecursion => {
                write!(f, "Exceeded maximum struct recursion")
            }
            SignatureErrorKind::ExceededMaximumDictRecursion => {
                write!(f, "Exceeded maximum dict recursion")
            }
            SignatureErrorKind::DictEntryHasTooManyFields => {
                write!(f, "Dict entry has too many fields")
            }
        }
    }
}

impl error::Error for SignatureError {}
