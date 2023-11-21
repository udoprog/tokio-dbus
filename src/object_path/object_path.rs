use std::fmt;
use std::str::from_utf8_unchecked;

use crate::{buf::BufMut, OwnedObjectPath, Read, ReadBuf, Result, Signature, Write};

use super::{validate, ObjectPathError};

/// A validated object path.
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
#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct ObjectPath([u8]);

impl ObjectPath {
    /// The special `"/"` object path.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::ObjectPath;
    ///
    /// assert_eq!(ObjectPath::ROOT, ObjectPath::new(b"/")?);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const ROOT: &'static Self = Self::new_const(b"/");

    /// Construct a new object path.
    #[track_caller]
    pub const fn new_const(path: &[u8]) -> &Self {
        if !validate(path) {
            panic!("Invalid D-Bus object path");
        }

        // SAFETY: The byte slice is repr transparent over this type.
        unsafe { Self::new_unchecked(path) }
    }

    /// Construct a new validated object path.
    pub fn new(path: &[u8]) -> Result<&Self, ObjectPathError> {
        if !validate(path) {
            return Err(ObjectPathError);
        }

        // SAFETY: The byte slice is repr transparent over this type.
        unsafe { Ok(Self::new_unchecked(path)) }
    }

    /// Construct a new unchecked object path.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the path is a valid object path.
    pub(super) const unsafe fn new_unchecked(path: &[u8]) -> &Self {
        &*(path as *const _ as *const Self)
    }

    /// Get the object path as a string.
    pub(crate) fn as_str(&self) -> &str {
        // SAFETY: Validation indirectly ensures that the signature is valid
        // UTF-8.
        unsafe { from_utf8_unchecked(&self.0) }
    }
}

impl fmt::Display for ObjectPath {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl fmt::Debug for ObjectPath {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl ToOwned for ObjectPath {
    type Owned = OwnedObjectPath;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        // SAFETY: Type ensures that it contains a valid object path during
        // construction.
        unsafe { OwnedObjectPath::from_raw_vec(self.0.to_vec()) }
    }
}

impl From<&ObjectPath> for Box<ObjectPath> {
    #[inline]
    fn from(object_path: &ObjectPath) -> Self {
        // SAFETY: ObjectPath is repr(transparent) over [u8].
        unsafe {
            Box::from_raw(Box::into_raw(Box::<[u8]>::from(&object_path.0)) as *mut ObjectPath)
        }
    }
}

impl Clone for Box<ObjectPath> {
    #[inline]
    fn clone(&self) -> Self {
        Box::<ObjectPath>::from(&**self)
    }
}

impl Write for ObjectPath {
    const SIGNATURE: &'static Signature = Signature::OBJECT_PATH;

    #[inline]
    fn write_to<O: ?Sized>(&self, buf: &mut O)
    where
        O: BufMut,
    {
        buf.store(self.0.len() as u32);
        buf.extend_from_slice_nul(&self.0);
    }
}

impl Read for ObjectPath {
    #[inline]
    fn read_from<'de>(buf: &mut ReadBuf<'de>) -> Result<&'de Self> {
        let len = buf.load::<u32>()? as usize;
        let bytes = buf.load_slice_nul(len)?;
        Ok(ObjectPath::new(bytes)?)
    }
}
