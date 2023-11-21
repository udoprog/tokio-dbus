use std::{borrow::Borrow, ops::Deref};

use super::ObjectPath;

/// A validated owned object path.
///
/// The following rules define a [valid object path]. Implementations must not
/// send or accept messages with invalid object paths.
///
/// [valid object path]: https://dbus.freedesktop.org/doc/dbus-specification.html#message-protocol-marshaling-object-path
///
/// * The path may be of any length.
/// * The path must begin with an ASCII '/' (integer 47) character, and must
///   consist of elements separated by slash characters.
/// * Each element must only contain the ASCII characters "[A-Z][a-z][0-9]_"
/// * No element may be the empty string.
/// * Multiple '/' characters cannot occur in sequence.
/// * A trailing '/' character is not allowed unless the path is the root path
///   (a single '/' character).
#[derive(Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct OwnedObjectPath(Vec<u8>);

impl OwnedObjectPath {
    /// Construct an owned object path from its raw underlying vector.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the vector contains a valid object path.
    #[inline]
    pub(super) unsafe fn from_raw_vec(data: Vec<u8>) -> Self {
        Self(data)
    }

    #[inline]
    fn to_object_path(&self) -> &ObjectPath {
        // SAFETY: This type ensures during construction that the object path it
        // contains is valid.
        unsafe { ObjectPath::new_unchecked(&self.0) }
    }
}

impl Deref for OwnedObjectPath {
    type Target = ObjectPath;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.to_object_path()
    }
}

impl Borrow<ObjectPath> for OwnedObjectPath {
    #[inline]
    fn borrow(&self) -> &ObjectPath {
        self
    }
}

impl AsRef<ObjectPath> for OwnedObjectPath {
    #[inline]
    fn as_ref(&self) -> &ObjectPath {
        self
    }
}
