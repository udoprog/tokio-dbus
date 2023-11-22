use std::fmt;
use std::str::from_utf8_unchecked;

use crate::buf::Buf;
use crate::buf::BufMut;
use crate::{OwnedObjectPath, Read, Result, Signature, Write};

use super::{validate, Iter, ObjectPathError};

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
    ///
    /// # Panics
    ///
    /// Panics if the argument is not a valid object.
    ///
    /// See [`ObjectPath`] for more information.
    #[track_caller]
    pub const fn new_const(path: &[u8]) -> &Self {
        if !validate(path) {
            panic!("Invalid D-Bus object path");
        }

        // SAFETY: The byte slice is repr transparent over this type.
        unsafe { Self::new_unchecked(path) }
    }

    /// Construct a new validated object path.
    ///
    /// # Errors
    ///
    /// Errors if the argument is not a valid object.
    ///
    /// See [`ObjectPath`] for more information.
    pub fn new<P>(path: &P) -> Result<&Self, ObjectPathError>
    where
        P: ?Sized + AsRef<[u8]>,
    {
        let path = path.as_ref();

        if !validate(path) {
            return Err(ObjectPathError);
        }

        // SAFETY: The byte slice is repr transparent over this type.
        unsafe { Ok(Self::new_unchecked(path)) }
    }

    /// Construct an iterator over the object path.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::ObjectPath;
    ///
    /// let mut it = ObjectPath::new_const(b"/").iter();
    /// assert!(it.next().is_none());
    ///
    /// let mut it = ObjectPath::new_const(b"/foo").iter();
    /// assert_eq!(it.next(), Some("foo"));
    /// assert!(it.next().is_none());
    ///
    /// let mut it = ObjectPath::new_const(b"/foo/bar").iter();
    /// assert_eq!(it.next_back(), Some("bar"));
    /// assert_eq!(it.next(), Some("foo"));
    /// assert!(it.next().is_none());
    /// ```
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(&self.0)
    }

    /// Test if one part starts with another.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::ObjectPath;
    ///
    /// const FOO: &ObjectPath = ObjectPath::new_const(b"/foo");
    /// const FOO_BAR: &ObjectPath = ObjectPath::new_const(b"/foo/bar");
    ///
    /// assert!(FOO_BAR.starts_with(FOO));
    /// ```
    #[must_use]
    pub fn starts_with(&self, other: &ObjectPath) -> bool {
        self.0.starts_with(&other.0)
    }

    /// Construct a new unchecked object path.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the path is a valid object path.
    #[must_use]
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

impl AsRef<ObjectPath> for ObjectPath {
    #[inline]
    fn as_ref(&self) -> &ObjectPath {
        self
    }
}

impl AsRef<[u8]> for ObjectPath {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
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

/// The [`IntoIterator`] implementation for [`ObjectPath`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::ObjectPath;
///
/// const PATH: &ObjectPath = ObjectPath::new_const(b"/foo/bar");
///
/// let mut values = Vec::new();
///
/// for s in PATH {
///     values.push(s);
/// }
///
/// assert_eq!(values, ["foo", "bar"]);
/// ```
impl<'a> IntoIterator for &'a ObjectPath {
    type Item = &'a str;
    type IntoIter = Iter<'a>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Write for ObjectPath {
    const SIGNATURE: &'static Signature = Signature::OBJECT_PATH;

    #[inline]
    fn write_to<O: ?Sized>(&self, buf: &mut O) -> Result<()>
    where
        O: BufMut,
    {
        buf.store(self.0.len() as u32)?;
        buf.extend_from_slice_nul(&self.0);
        Ok(())
    }
}

impl Read for ObjectPath {
    #[inline]
    fn read_from<'de, B>(mut buf: B) -> Result<&'de Self>
    where
        B: Buf<'de>,
    {
        let len = buf.load::<u32>()? as usize;
        let bytes = buf.load_slice_nul(len)?;
        Ok(ObjectPath::new(bytes)?)
    }
}
