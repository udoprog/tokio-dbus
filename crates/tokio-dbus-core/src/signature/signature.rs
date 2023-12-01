use std::fmt;
use std::str::from_utf8_unchecked;

use super::{validate, Iter, SignatureBuf, SignatureError};

/// A D-Bus signature.
///
/// # Examples
///
/// ```
/// use tokio_dbus::Signature;
///
/// const SIG: &Signature = Signature::new_const(b"aaaai");
///
/// assert!(Signature::new(b"aai").is_ok());
/// ```
#[derive(Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct Signature([u8]);

impl Signature {
    /// The empty signature.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let body = BodyBuf::new();
    /// assert_eq!(body.signature(), Signature::EMPTY);
    /// ```
    pub const EMPTY: &'static Signature = Signature::new_const(b"");

    /// The signature of a [`Signature`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    /// body.store(Signature::new(b"g")?);
    ///
    /// assert_eq!(body.signature(), Signature::SIGNATURE);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const SIGNATURE: &'static Signature = Signature::new_const(b"g");

    /// The signature of an object path.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature, ObjectPath};
    ///
    /// let mut body = BodyBuf::new();
    /// body.store(ObjectPath::new(b"/org/freedesktop/DBus")?);
    ///
    /// assert_eq!(body.signature(), Signature::OBJECT_PATH);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const OBJECT_PATH: &'static Signature = Signature::new_const(b"o");

    /// The signature of a nul-terminated string.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    /// body.store("Hello World!");
    ///
    /// assert_eq!(body.signature(), Signature::STRING);
    /// ```
    pub const STRING: &'static Signature = Signature::new_const(b"s");

    /// The signature of a variant value.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature, Variant};
    ///
    /// let mut body = BodyBuf::new();
    /// body.store(Variant::U32(10u32));
    ///
    /// assert_eq!(body.signature(), Signature::VARIANT);
    /// ```
    pub const VARIANT: &'static Signature = Signature::new_const(b"v");

    /// A single byte.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u8);
    ///
    /// assert_eq!(body.signature(), Signature::BYTE);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const BYTE: &'static Signature = Signature::new_const(b"y");

    /// Signed (two's complement) 16-bit integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10i16);
    ///
    /// assert_eq!(body.signature(), Signature::INT16);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const INT16: &'static Signature = Signature::new_const(b"n");

    /// Unsigned 16-bit integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u16);
    ///
    /// assert_eq!(body.signature(), Signature::UINT16);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const UINT16: &'static Signature = Signature::new_const(b"q");

    /// Signed (two's complement) 32-bit integer
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10i32);
    ///
    /// assert_eq!(body.signature(), Signature::INT32);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const INT32: &'static Signature = Signature::new_const(b"i");

    /// Unsigned 32-bit integer
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u32);
    ///
    /// assert_eq!(body.signature(), Signature::UINT32);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const UINT32: &'static Signature = Signature::new_const(b"u");

    /// Signed (two's complement) 64-bit integer (mnemonic: x and t are the
    /// first characters in "sixty" not already used for something more common)
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10i64);
    ///
    /// assert_eq!(body.signature(), Signature::INT64);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const INT64: &'static Signature = Signature::new_const(b"x");

    /// Unsigned 64-bit integer
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(10u64);
    ///
    /// assert_eq!(body.signature(), Signature::UINT64);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const UINT64: &'static Signature = Signature::new_const(b"t");

    /// IEEE 754 double-precision floating point
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature};
    ///
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(3.1415f64);
    ///
    /// assert_eq!(body.signature(), Signature::DOUBLE);
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub const DOUBLE: &'static Signature = Signature::new_const(b"d");

    /// Unsigned 32-bit integer representing an index into an out-of-band array
    /// of file descriptors, transferred via some platform-specific mechanism
    /// (mnemonic: h for handle)
    pub const UNIX_FD: &'static Signature = Signature::new_const(b"h");

    /// Construct a new signature with validation inside of a constant context.
    ///
    /// This will panic in case the signature is invalid.
    ///
    /// ```compile_fail
    /// use tokio_dbus::Signature;
    ///
    /// const BAD: &Signature = Signature::new_const(b"(a)");
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::Signature;
    ///
    /// const SIG: &Signature = Signature::new_const(b"i(ai)");
    /// ```
    #[inline]
    #[track_caller]
    pub const fn new_const(signature: &[u8]) -> &Signature {
        if validate(signature).is_err() {
            panic!("Invalid D-Bus signature")
        };

        // SAFETY: The byte slice is repr transparent over this type.
        unsafe { Self::new_unchecked(signature) }
    }

    /// Try to construct a new signature with validation.
    #[inline]
    pub fn new<S>(signature: &S) -> Result<&Signature, SignatureError>
    where
        S: ?Sized + AsRef<[u8]>,
    {
        let signature = signature.as_ref();
        validate(signature)?;
        // SAFETY: The byte slice is repr transparent over this type.
        unsafe { Ok(Self::new_unchecked(signature)) }
    }

    /// Construct a new signature without validation. The caller is responsible
    /// for ensuring that the signature is valid.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the signature is a valid signature.
    #[inline]
    pub const unsafe fn new_unchecked(signature: &[u8]) -> &Self {
        &*(signature as *const _ as *const Signature)
    }

    /// Construct a new empty signature.
    pub const fn empty() -> &'static Self {
        unsafe { Self::new_unchecked(&[]) }
    }

    /// Test if the signature is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the length of the signature in bytes.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }

    /// Get the signature as a string.
    pub fn as_str(&self) -> &str {
        // SAFETY: Validation indirectly ensures that the signature is valid UTF-8.
        unsafe { from_utf8_unchecked(&self.0) }
    }

    /// Get the signature as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Signature {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}

impl AsRef<Signature> for Signature {
    #[inline]
    fn as_ref(&self) -> &Signature {
        self
    }
}

impl ToOwned for Signature {
    type Owned = SignatureBuf;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        unsafe { SignatureBuf::from_slice_unchecked(&self.0) }
    }
}

/// Equality check between [`SignatureBuf`] and [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(*Signature::EMPTY, SignatureBuf::empty());
/// assert_eq!(*Signature::STRING, SignatureBuf::new(b"s")?);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
impl PartialEq<SignatureBuf> for Signature {
    #[inline]
    fn eq(&self, other: &SignatureBuf) -> bool {
        self.0 == other.0
    }
}

/// Equality check between [`SignatureBuf`] and a borrowed [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(Signature::EMPTY, SignatureBuf::empty());
/// assert_eq!(Signature::STRING, SignatureBuf::new(b"s")?);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
impl PartialEq<SignatureBuf> for &Signature {
    #[inline]
    fn eq(&self, other: &SignatureBuf) -> bool {
        self.0 == other.0
    }
}

/// Equality check between [`[u8]`] and a [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(*Signature::EMPTY, b""[..]);
/// assert_eq!(*Signature::STRING, b"s"[..]);
/// ```
impl PartialEq<[u8]> for Signature {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.0 == *other
    }
}

/// Equality check between [`[u8]`] and a borrowed [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(Signature::EMPTY, b""[..]);
/// assert_eq!(Signature::STRING, b"s"[..]);
/// ```
impl PartialEq<[u8]> for &Signature {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.0 == *other
    }
}

/// Equality check between [`[u8; N]`] and a [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(Signature::EMPTY, b"");
/// assert_eq!(Signature::STRING, b"s");
/// ```
impl<const N: usize> PartialEq<[u8; N]> for Signature {
    #[inline]
    fn eq(&self, other: &[u8; N]) -> bool {
        self.0 == other[..]
    }
}

/// Equality check between [`[u8; N]`] and a borrowed [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(Signature::EMPTY, b"");
/// assert_eq!(Signature::STRING, b"s");
/// ```
impl<const N: usize> PartialEq<[u8; N]> for &Signature {
    #[inline]
    fn eq(&self, other: &[u8; N]) -> bool {
        self.0 == other[..]
    }
}

/// Equality check between [`str`] and a [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(*Signature::EMPTY, *"");
/// assert_eq!(*Signature::STRING, *"s");
/// ```
impl PartialEq<str> for Signature {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.0 == *other.as_bytes()
    }
}

/// Equality check between [`str`] and a borrowed [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(Signature::EMPTY, *"");
/// assert_eq!(Signature::STRING, *"s");
/// ```
impl PartialEq<str> for &Signature {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.0 == *other.as_bytes()
    }
}

/// Equality check between [`str`] and a [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(*Signature::EMPTY, *"");
/// assert_eq!(*Signature::STRING, *"s");
/// ```
impl PartialEq<&str> for Signature {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other.as_bytes()
    }
}

impl From<&Signature> for Box<Signature> {
    #[inline]
    fn from(signature: &Signature) -> Self {
        // SAFETY: ObjectPath is repr(transparent) over [u8].
        unsafe { Box::from_raw(Box::into_raw(Box::<[u8]>::from(&signature.0)) as *mut Signature) }
    }
}
