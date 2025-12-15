use core::fmt;

/// An error constructing an object path.
#[derive(Debug)]
#[non_exhaustive]
pub struct ObjectPathError;

impl fmt::Display for ObjectPathError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Invalid D-Bus object path")
    }
}

impl core::error::Error for ObjectPathError {}
