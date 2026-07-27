use std::error;
use std::fmt;

use tokio_dbus::{SignatureBuf, SignatureError};

/// Result alias defaulting to the error type of this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error raised while talking to the bus.
#[derive(Debug)]
pub struct Error {
    // NB: Boxed so that a `Result` carrying this error stays small, since every
    // encode and decode threads one through.
    kind: Box<ErrorKind>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            kind: Box::new(kind),
        }
    }

    /// Construct an error which is reported back to a caller as an error reply.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus_runtime::Error;
    ///
    /// let error = Error::remote("com.example.Error.Busy", "Try again later");
    /// assert_eq!(error.name(), Some("com.example.Error.Busy"));
    /// ```
    pub fn remote(name: impl AsRef<str>, message: impl fmt::Display) -> Self {
        Self::new(ErrorKind::Remote {
            name: name.as_ref().into(),
            message: message.to_string().into(),
        })
    }

    /// The D-Bus error name, if this error came from, or is destined for, an
    /// error reply.
    pub fn name(&self) -> Option<&str> {
        match &*self.kind {
            ErrorKind::Remote { name, .. } => Some(name),
            _ => None,
        }
    }
}

impl From<tokio_dbus::Error> for Error {
    #[inline]
    fn from(error: tokio_dbus::Error) -> Self {
        Self::new(ErrorKind::Dbus(error))
    }
}

impl From<SignatureError> for Error {
    #[inline]
    fn from(error: SignatureError) -> Self {
        Self::new(ErrorKind::Signature(error))
    }
}

impl From<std::io::Error> for Error {
    #[inline]
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorKind::Dbus(error.into()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.kind {
            ErrorKind::Dbus(..) => write!(f, "D-Bus error"),
            ErrorKind::Signature(..) => write!(f, "Signature error"),
            ErrorKind::Remote { name, message } => write!(f, "{name}: {message}"),
            ErrorKind::UnsupportedType(signature) => {
                write!(f, "Cannot represent a value of type `{signature}`")
            }
            ErrorKind::UnexpectedSignature(signatures) => {
                let (expected, actual) = &**signatures;
                write!(f, "Expected a value of type `{expected}`, got `{actual}`")
            }
            ErrorKind::MissingUniqueName => {
                write!(f, "The bus did not reply to `Hello` with a unique name")
            }
            ErrorKind::NameTaken(name) => {
                write!(f, "Could not acquire the name `{name}`")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &*self.kind {
            ErrorKind::Dbus(error) => Some(error),
            ErrorKind::Signature(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum ErrorKind {
    Dbus(tokio_dbus::Error),
    Signature(SignatureError),
    Remote { name: Box<str>, message: Box<str> },
    UnsupportedType(Box<SignatureBuf>),
    // NB: Boxed because a `SignatureBuf` is an inline buffer, so a variant with
    // two of them is much larger than any of the others.
    UnexpectedSignature(Box<(SignatureBuf, SignatureBuf)>),
    MissingUniqueName,
    NameTaken(Box<str>),
}
