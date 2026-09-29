#[cfg(feature = "std")]
use std::io;

#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use core::error;
use core::fmt;
use core::str::Utf8Error;

#[cfg(feature = "alloc")]
use crate::Signature;
#[cfg(feature = "tokio")]
use crate::connection::Sasl;
use crate::{ObjectPathError, SignatureError};

/// Result alias using an [`Error`] as the error type by default.
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// An error raised by this crate.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    #[inline]
    pub(crate) fn new(kind: ErrorKind) -> Error {
        Self { kind }
    }

    /// Test if the error indicates that the operation would block.
    #[cfg(feature = "tokio")]
    #[inline]
    pub(crate) fn would_block(&self) -> bool {
        matches!(self.kind, ErrorKind::WouldBlock)
    }
}

impl From<SignatureError> for Error {
    #[inline]
    fn from(error: SignatureError) -> Self {
        Self::new(ErrorKind::Signature(error))
    }
}

impl From<ObjectPathError> for Error {
    #[inline]
    fn from(error: ObjectPathError) -> Self {
        Self::new(ErrorKind::ObjectPath(error))
    }
}

#[cfg(feature = "std")]
impl From<io::Error> for Error {
    #[inline]
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::WouldBlock => Self::new(ErrorKind::WouldBlock),
            _ => Self::new(ErrorKind::Io(error)),
        }
    }
}

impl From<Utf8Error> for Error {
    #[inline]
    fn from(error: Utf8Error) -> Self {
        Self::new(ErrorKind::Utf8Error(error))
    }
}

impl From<ErrorKind> for Error {
    #[inline]
    fn from(kind: ErrorKind) -> Self {
        Self::new(kind)
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            #[cfg(feature = "std")]
            ErrorKind::Io(..) => write!(f, "I/O error"),
            ErrorKind::Signature(..) => write!(f, "Signature error"),
            ErrorKind::ObjectPath(..) => write!(f, "ObjectPath error"),
            ErrorKind::Utf8Error(..) => write!(f, "UTF-8 error"),
            #[cfg(all(feature = "tokio", not(feature = "libc")))]
            ErrorKind::UnsupportedAuthUid => {
                write!(
                    f,
                    "Authentication using the current UID requires the `libc` feature to be enabled"
                )
            }
            #[cfg(feature = "std")]
            ErrorKind::WouldBlock => write!(f, "Would block"),
            ErrorKind::BufferUnderflow => write!(f, "Buffer underflow"),
            #[cfg(feature = "tokio")]
            ErrorKind::MissingBus => write!(f, "Missing bus to connect to"),
            #[cfg(feature = "tokio")]
            ErrorKind::InvalidAddress => write!(f, "Invalid d-bus address"),
            #[cfg(feature = "tokio")]
            ErrorKind::InvalidSaslState(state) => write!(f, "Invalid sasl state {state}"),
            #[cfg(feature = "tokio")]
            ErrorKind::InvalidSasl => write!(f, "Invalid SASL message"),
            #[cfg(feature = "tokio")]
            ErrorKind::InvalidSaslResponse => write!(f, "Invalid SASL command"),
            #[cfg(feature = "alloc")]
            ErrorKind::InvalidProtocol => write!(f, "Invalid protocol"),
            #[cfg(feature = "alloc")]
            ErrorKind::MissingPath => write!(f, "Missing required PATH header"),
            #[cfg(feature = "alloc")]
            ErrorKind::MissingMember => write!(f, "Missing required MEMBER header"),
            #[cfg(feature = "alloc")]
            ErrorKind::MissingReplySerial => write!(f, "Missing required REPLY_SERIAL header"),
            #[cfg(feature = "tokio")]
            ErrorKind::ZeroSerial => write!(f, "Zero in header serial"),
            #[cfg(feature = "alloc")]
            ErrorKind::ZeroReplySerial => write!(f, "Zero REPLY_SERIAL header"),
            #[cfg(feature = "alloc")]
            ErrorKind::MissingErrorName => write!(f, "Missing required ERROR_NAME header"),
            ErrorKind::NotNullTerminated => {
                write!(f, "String is not null terminated")
            }
            ErrorKind::ArrayTooLong(length) => {
                write!(f, "Array of length {length} is too long (max is 67108864)")
            }
            #[cfg(feature = "alloc")]
            ErrorKind::BodyTooLong(length) => {
                write!(f, "Body of length {length} is too long (max is 134217728)")
            }
            #[cfg(feature = "alloc")]
            ErrorKind::HeaderTooLong(length) => {
                write!(
                    f,
                    "Header of length {length} is too long (max is 134217728)"
                )
            }
            #[cfg(feature = "alloc")]
            ErrorKind::MissingMessage => {
                write!(f, "No message")
            }
            #[cfg(feature = "alloc")]
            ErrorKind::UnsupportedVariant(signature) => {
                write!(f, "Unsupported variant signature {signature:?}")
            }
            #[cfg(not(feature = "alloc"))]
            ErrorKind::UnsupportedVariantNoAlloc => {
                write!(f, "Unsupported variant signature")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            #[cfg(feature = "std")]
            ErrorKind::Io(error) => Some(error),
            ErrorKind::Signature(error) => Some(error),
            ErrorKind::ObjectPath(error) => Some(error),
            ErrorKind::Utf8Error(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum ErrorKind {
    #[cfg(feature = "std")]
    Io(io::Error),
    Signature(SignatureError),
    ObjectPath(ObjectPathError),
    Utf8Error(Utf8Error),
    #[cfg(all(feature = "tokio", not(feature = "libc")))]
    UnsupportedAuthUid,
    #[cfg(feature = "std")]
    WouldBlock,
    BufferUnderflow,
    #[cfg(feature = "tokio")]
    MissingBus,
    #[cfg(feature = "tokio")]
    InvalidAddress,
    #[cfg(feature = "tokio")]
    InvalidSaslState(Sasl),
    #[cfg(feature = "tokio")]
    InvalidSasl,
    #[cfg(feature = "tokio")]
    InvalidSaslResponse,
    #[cfg(feature = "alloc")]
    InvalidProtocol,
    #[cfg(feature = "alloc")]
    MissingPath,
    #[cfg(feature = "alloc")]
    MissingMember,
    #[cfg(feature = "alloc")]
    MissingReplySerial,
    #[cfg(feature = "tokio")]
    ZeroSerial,
    #[cfg(feature = "alloc")]
    ZeroReplySerial,
    #[cfg(feature = "alloc")]
    MissingErrorName,
    NotNullTerminated,
    ArrayTooLong(u32),
    #[cfg(feature = "alloc")]
    BodyTooLong(u32),
    #[cfg(feature = "alloc")]
    HeaderTooLong(u32),
    #[cfg(feature = "alloc")]
    MissingMessage,
    #[cfg(feature = "alloc")]
    UnsupportedVariant(Box<Signature>),
    #[cfg(not(feature = "alloc"))]
    UnsupportedVariantNoAlloc,
}
