use core::borrow::Borrow;
use core::fmt;
use core::hash;
use core::ops::Deref;
use core::str::FromStr;

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec::Vec;

use super::{ObjectPath, ObjectPathError, validate};

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
/// * Each element must only contain the ASCII characters `[A-Z][a-z][0-9]_`
/// * No element may be the empty string.
/// * Multiple '/' characters cannot occur in sequence.
/// * A trailing '/' character is not allowed unless the path is the root path
///   (a single '/' character).
#[derive(Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct ObjectPathBuf(Vec<u8>);

impl ObjectPathBuf {
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

/// Construct an owned object path from a vector, taking ownership of its
/// allocation.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{ObjectPath, ObjectPathBuf};
///
/// let path = ObjectPathBuf::try_from(b"/org/freedesktop/DBus".to_vec())?;
/// assert_eq!(&*path, ObjectPath::new("/org/freedesktop/DBus")?);
///
/// assert!(ObjectPathBuf::try_from(b"org/freedesktop/DBus".to_vec()).is_err());
/// # Ok::<_, tokio_dbus::ObjectPathError>(())
/// ```
impl TryFrom<Vec<u8>> for ObjectPathBuf {
    type Error = ObjectPathError;

    #[inline]
    fn try_from(path: Vec<u8>) -> Result<Self, Self::Error> {
        if !validate(&path) {
            return Err(ObjectPathError);
        }

        Ok(Self(path))
    }
}

/// Construct an owned object path from a string, taking ownership of its
/// allocation.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{ObjectPath, ObjectPathBuf};
///
/// let path = ObjectPathBuf::try_from(String::from("/org/freedesktop/DBus"))?;
/// assert_eq!(&*path, ObjectPath::new("/org/freedesktop/DBus")?);
/// # Ok::<_, tokio_dbus::ObjectPathError>(())
/// ```
impl TryFrom<String> for ObjectPathBuf {
    type Error = ObjectPathError;

    #[inline]
    fn try_from(path: String) -> Result<Self, Self::Error> {
        Self::try_from(path.into_bytes())
    }
}

/// Construct an owned object path by copying a string.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{ObjectPath, ObjectPathBuf};
///
/// let path: ObjectPathBuf = "/org/freedesktop/DBus".parse()?;
/// assert_eq!(&*path, ObjectPath::new("/org/freedesktop/DBus")?);
/// # Ok::<_, tokio_dbus::ObjectPathError>(())
/// ```
impl FromStr for ObjectPathBuf {
    type Err = ObjectPathError;

    #[inline]
    fn from_str(path: &str) -> Result<Self, Self::Err> {
        Ok(ObjectPath::new(path)?.to_owned())
    }
}

impl hash::Hash for ObjectPathBuf {
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: hash::Hasher,
    {
        hash::Hash::hash(&**self, state);
    }
}

impl fmt::Display for ObjectPathBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl fmt::Debug for ObjectPathBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl Deref for ObjectPathBuf {
    type Target = ObjectPath;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.to_object_path()
    }
}

impl Borrow<ObjectPath> for ObjectPathBuf {
    #[inline]
    fn borrow(&self) -> &ObjectPath {
        self
    }
}

impl AsRef<ObjectPath> for ObjectPathBuf {
    #[inline]
    fn as_ref(&self) -> &ObjectPath {
        self
    }
}
