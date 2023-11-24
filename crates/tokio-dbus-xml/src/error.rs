use std::error;
use std::fmt;

use tokio_dbus_core::signature::SignatureError;

/// Result alias defaulting to the error type of this alias.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error raised by this crate.
#[derive(Debug)]
pub struct Error {
    path: Box<str>,
    kind: ErrorKind,
}

impl Error {
    pub(crate) fn new<P, K>(path: P, kind: K) -> Self
    where
        Box<str>: From<P>,
        ErrorKind: From<K>,
    {
        Self {
            path: path.into(),
            kind: kind.into(),
        }
    }
}

impl fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.kind)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            ErrorKind::XmlParser(error) => Some(error),
            ErrorKind::Signature(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ErrorKind {
    XmlParser(xmlparser::Error),
    Signature(SignatureError),
    UnsupportedElementStart(Box<str>),
    UnsupportedElementEnd,
    UnsupportedAttribute(Box<str>),
    UnsupportedText,
    MismatchingEnd {
        expected: Box<str>,
        actual: Box<str>,
    },
    MissingMethodName,
    MissingInterfaceName,
    MissingArgumentType,
    UnsupportedArgumentDirection(Box<str>),
    MissingArgumentDirection,
}

impl From<xmlparser::Error> for ErrorKind {
    #[inline]
    fn from(error: xmlparser::Error) -> Self {
        ErrorKind::XmlParser(error)
    }
}

impl From<SignatureError> for ErrorKind {
    #[inline]
    fn from(error: SignatureError) -> Self {
        ErrorKind::Signature(error)
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::XmlParser(error) => error.fmt(f),
            ErrorKind::Signature(error) => error.fmt(f),
            ErrorKind::UnsupportedElementStart(element) => {
                write!(f, "Unsupported element: {element}")
            }
            ErrorKind::UnsupportedElementEnd => {
                write!(f, "Unsupported element end")
            }
            ErrorKind::UnsupportedAttribute(name) => {
                write!(f, "Unsupported attribute: {name}")
            }
            ErrorKind::UnsupportedText => {
                write!(f, "Unsupported text")
            }
            ErrorKind::MismatchingEnd { expected, actual } => {
                write!(f, "Mismatching end: expected {expected}, found {actual}",)
            }
            ErrorKind::MissingMethodName => {
                write!(f, "Missing method name")
            }
            ErrorKind::MissingInterfaceName => {
                write!(f, "Missing interface name")
            }
            ErrorKind::MissingArgumentType => {
                write!(f, "Missing argument type")
            }
            ErrorKind::UnsupportedArgumentDirection(value) => {
                write!(f, "Unsupported argument direction `{value}`")
            }
            ErrorKind::MissingArgumentDirection => {
                write!(f, "Missing argument direction")
            }
        }
    }
}
