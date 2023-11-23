use std::error;
use std::fmt;
use std::io;
use std::str::Utf8Error;

use crate::connection::TransportState;
use crate::ObjectPathError;
use crate::Signature;
use crate::SignatureError;

/// Result alias using an [`Error`] as the error type by default.
pub type Result<T, E = Error> = std::result::Result<T, E>;

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
            ErrorKind::Io(error) => error.fmt(f),
            ErrorKind::Signature(error) => error.fmt(f),
            ErrorKind::ObjectPath(error) => error.fmt(f),
            ErrorKind::Utf8Error(error) => error.fmt(f),
            ErrorKind::WouldBlock => write!(f, "Would block"),
            ErrorKind::BufferUnderflow => write!(f, "Buffer underflow"),
            ErrorKind::MissingBus => write!(f, "Missing session bus"),
            ErrorKind::InvalidAddress => write!(f, "Invalid d-bus address"),
            ErrorKind::InvalidSasl => write!(f, "Invalid SASL message"),
            ErrorKind::InvalidSaslResponse => write!(f, "Invalid SASL command"),
            ErrorKind::InvalidState(state) => write!(f, "Invalid connection state `{state}`"),
            ErrorKind::InvalidProtocol => write!(f, "Invalid protocol"),
            ErrorKind::MissingPath => write!(f, "Missing required PATH header"),
            ErrorKind::MissingMember => write!(f, "Missing required MEMBER header"),
            ErrorKind::MissingReplySerial => write!(f, "Missing required REPLY_SERIAL header"),
            ErrorKind::ZeroSerial => write!(f, "Zero in header serial"),
            ErrorKind::ZeroReplySerial => write!(f, "Zero REPLY_SERIAL header"),
            ErrorKind::MissingErrorName => write!(f, "Missing required ERROR_NAME header"),
            ErrorKind::NotNullTerminated => {
                write!(f, "String is not null terminated")
            }
            ErrorKind::ArrayTooLong(length) => {
                write!(f, "Array of length {length} is too long (max is 67108864)")
            }
            ErrorKind::BodyTooLong(length) => {
                write!(f, "Body of length {length} is too long (max is 134217728)")
            }
            ErrorKind::MissingMessage => {
                write!(f, "No message")
            }
            ErrorKind::ResponseError(error_name, message) => {
                write!(f, "Response error: {error_name}: {message}")
            }
            ErrorKind::UnsupportedVariant(signature) => {
                write!(f, "Unsupported variant {signature:?}")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
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
    Io(io::Error),
    Signature(SignatureError),
    ObjectPath(ObjectPathError),
    Utf8Error(Utf8Error),
    WouldBlock,
    BufferUnderflow,
    MissingBus,
    InvalidAddress,
    InvalidSasl,
    InvalidSaslResponse,
    InvalidState(TransportState),
    InvalidProtocol,
    MissingPath,
    MissingMember,
    MissingReplySerial,
    ZeroSerial,
    ZeroReplySerial,
    MissingErrorName,
    NotNullTerminated,
    BodyTooLong(u32),
    ArrayTooLong(u32),
    MissingMessage,
    UnsupportedVariant(Box<Signature>),
    ResponseError(Box<str>, Box<str>),
}
