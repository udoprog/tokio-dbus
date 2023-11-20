use std::error;
use std::fmt;

use crate::protocol::Type;

/// Detailed errors raised when validation of a [`Signature`] fails.
///
/// [`Signature`]: crate::Signature
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignatureError {
    UnknownTypeCode(u8),
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
        match *self {
            SignatureError::UnknownTypeCode(code) => {
                write!(f, "Unknown type code: {:?}", Type(code))
            }
            SignatureError::SignatureTooLong => {
                write!(f, "Signature too long")
            }
            SignatureError::MissingArrayElementType => {
                write!(f, "Missing array element type")
            }
            SignatureError::StructEndedButNotStarted => {
                write!(f, "Struct ended but not started")
            }
            SignatureError::DictEndedButNotStarted => {
                write!(f, "Dict ended but not started")
            }
            SignatureError::StructStartedButNotEnded => {
                write!(f, "Struct started but not ended")
            }
            SignatureError::DictStartedButNotEnded => {
                write!(f, "Dict started but not ended")
            }
            SignatureError::StructHasNoFields => {
                write!(f, "Struct has no fields")
            }
            SignatureError::DictKeyMustBeBasicType => {
                write!(f, "Dict key must be basic type")
            }
            SignatureError::DictEntryHasNoFields => {
                write!(f, "Dict entry has no fields")
            }
            SignatureError::DictEntryHasOnlyOneField => {
                write!(f, "Dict entry has only one field")
            }
            SignatureError::DictEntryNotInsideArray => {
                write!(f, "Dict entry not inside array")
            }
            SignatureError::ExceededMaximumArrayRecursion => {
                write!(f, "Exceeded maximum array recursion")
            }
            SignatureError::ExceededMaximumStructRecursion => {
                write!(f, "Exceeded maximum struct recursion")
            }
            SignatureError::ExceededMaximumDictRecursion => {
                write!(f, "Exceeded maximum dict recursion")
            }
            SignatureError::DictEntryHasTooManyFields => {
                write!(f, "Dict entry has too many fields")
            }
        }
    }
}

impl error::Error for SignatureError {}
