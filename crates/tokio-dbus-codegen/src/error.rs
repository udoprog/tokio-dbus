use std::error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use tokio_dbus_core::signature::{SignatureBuf, SignatureError};

/// Result alias defaulting to the error type of this crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error raised while generating code.
#[derive(Debug)]
pub struct Error {
    // NB: Boxed so that a `Result` carrying this error stays small, since the
    // generator threads it through every type mapping.
    kind: Box<ErrorKind>,
    context: Option<Box<str>>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self {
            kind: Box::new(kind),
            context: None,
        }
    }

    /// Attach the element the error concerns, so that a failure points at the
    /// interface file rather than at the generator.
    pub(crate) fn context(mut self, context: impl fmt::Display) -> Self {
        if self.context.is_none() {
            self.context = Some(context.to_string().into());
        }

        self
    }

    pub(crate) fn io(path: &Path, error: io::Error) -> Self {
        Self::new(ErrorKind::Io(path.to_owned(), error))
    }
}

impl From<SignatureError> for Error {
    #[inline]
    fn from(error: SignatureError) -> Self {
        Self::new(ErrorKind::Signature(error))
    }
}

impl From<tokio_dbus_xml::Error> for Error {
    #[inline]
    fn from(error: tokio_dbus_xml::Error) -> Self {
        Self::new(ErrorKind::Xml(error))
    }
}

impl From<genco::fmt::Error> for Error {
    #[inline]
    fn from(error: genco::fmt::Error) -> Self {
        Self::new(ErrorKind::Format(error))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(context) = &self.context {
            write!(f, "{context}: ")?;
        }

        match &*self.kind {
            ErrorKind::Io(path, ..) => write!(f, "{}: I/O error", path.display()),
            ErrorKind::Xml(..) => write!(f, "Could not parse interface file"),
            ErrorKind::Signature(..) => write!(f, "Invalid signature"),
            ErrorKind::Format(..) => write!(f, "Could not format generated code"),
            ErrorKind::MissingOutDir => write!(
                f,
                "OUT_DIR is not set, which means this is not running from a build script. \
                 Use `write_to` to name an output path instead"
            ),
            ErrorKind::MissingInterface(name) => {
                write!(f, "No interface named `{name}` in any of the files read")
            }
            ErrorKind::EmptyType => write!(f, "Expected a type, but the signature was empty"),
            ErrorKind::CompoundType(signature) => write!(
                f,
                "Expected a single type, but `{signature}` names more than one"
            ),
            ErrorKind::LooseDictEntry => write!(
                f,
                "A dict entry is only legal as the element type of an array"
            ),
            ErrorKind::UnknownType(signature) => write!(f, "Unknown type `{signature}`"),
            ErrorKind::UnixFd => write!(
                f,
                "Passing a file descriptor (`h`) is not supported, since it requires \
                 sending it out of band"
            ),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &*self.kind {
            ErrorKind::Io(_, error) => Some(error),
            ErrorKind::Xml(error) => Some(error),
            ErrorKind::Signature(error) => Some(error),
            ErrorKind::Format(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum ErrorKind {
    Io(PathBuf, io::Error),
    Xml(tokio_dbus_xml::Error),
    Signature(SignatureError),
    Format(genco::fmt::Error),
    MissingOutDir,
    MissingInterface(Box<str>),
    EmptyType,
    CompoundType(SignatureBuf),
    LooseDictEntry,
    UnknownType(SignatureBuf),
    UnixFd,
}
