use std::error;
use std::fmt;

/// Detailed errors raised when validation of a [`Signature`] fails.
///
/// [`Signature`]: crate::Signature
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
