use std::borrow::Borrow;
use std::fmt;
use std::mem::transmute;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::slice::from_raw_parts;

use super::{validate, Signature, SignatureError, MAX_SIGNATURE};

/// A D-Bus signature.
///
/// This is the owned variant which dereferences to [`Signature`].
#[derive(Clone)]
pub struct SignatureBuf {
    data: [MaybeUninit<u8>; MAX_SIGNATURE],
    init: usize,
}

impl SignatureBuf {
    /// Construct a new empty signature.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::SignatureBuf;
    ///
    /// let sig = SignatureBuf::empty();
    /// assert!(sig.is_empty());
    /// ```
    pub const fn empty() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            init: 0,
        }
    }

    /// Coerce an owned signature into its raw parts.
    pub(crate) const fn into_raw_parts(self) -> ([MaybeUninit<u8>; MAX_SIGNATURE], usize) {
        (self.data, 0)
    }

    /// Construct a new signature with validation inside of a constant context.
    ///
    /// This will panic in case the signature is invalid.
    ///
    /// ```compile_fail
    /// use tokio_dbus::SignatureBuf;
    ///
    /// const BAD: SignatureBuf = SignatureBuf::new_const(b"(a)");
    /// ```
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::SignatureBuf;
    ///
    /// const SIG: SignatureBuf = SignatureBuf::new_const(b"i(ai)");
    /// ```
    #[inline]
    #[track_caller]
    pub const fn new_const(signature: &[u8]) -> SignatureBuf {
        if validate(signature).is_err() {
            panic!("Invalid D-Bus signature")
        };

        // SAFETY: The byte slice is repr transparent over this type.
        unsafe { Self::from_slice_const_unchecked(signature) }
    }

    /// Try to construct a new signature with validation.
    #[inline]
    pub fn new(signature: &[u8]) -> Result<Self, SignatureError> {
        validate(signature)?;
        // SAFETY: The byte slice is repr transparent over this type.
        unsafe { Ok(Self::from_slice_unchecked(signature)) }
    }

    /// Construct an owned signature from a slice.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `bytes` is a valid signature.
    const unsafe fn from_slice_const_unchecked(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() <= MAX_SIGNATURE);
        let mut data = [0; MAX_SIGNATURE];

        let mut n = 0;

        while n < bytes.len() {
            data[n] = bytes[n];
            n += 1;
        }

        Self {
            data: transmute(data),
            init: bytes.len(),
        }
    }

    /// Construct an owned signature from a slice.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `bytes` is a valid signature.
    #[inline]
    pub(super) unsafe fn from_slice_unchecked(bytes: &[u8]) -> Self {
        debug_assert!(bytes.len() <= MAX_SIGNATURE);
        let mut this = Self::empty();
        this.data
            .as_mut_ptr()
            .cast::<u8>()
            .copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
        this.init = bytes.len();
        this
    }

    #[inline]
    fn as_slice(&self) -> &[u8] {
        // SAFETY: init is set to the initialized slice.
        unsafe { from_raw_parts(self.data.as_ptr().cast(), self.init) }
    }
}

impl fmt::Debug for SignatureBuf {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SignatureBuf").field(&self.as_str()).finish()
    }
}

impl Deref for SignatureBuf {
    type Target = Signature;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Construction of SignatureBuf ensures that the signature is
        // valid.
        unsafe { Signature::new_unchecked(self.as_slice()) }
    }
}

impl Borrow<Signature> for SignatureBuf {
    #[inline]
    fn borrow(&self) -> &Signature {
        self
    }
}

impl AsRef<Signature> for SignatureBuf {
    #[inline]
    fn as_ref(&self) -> &Signature {
        self
    }
}

/// Equality check between [`SignatureBuf`] and [`SignatureBuf`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(SignatureBuf::empty(), Signature::EMPTY.to_owned());
/// assert_eq!(SignatureBuf::new(b"s")?, Signature::STRING.to_owned());
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
impl PartialEq<SignatureBuf> for SignatureBuf {
    #[inline]
    fn eq(&self, other: &SignatureBuf) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Eq for SignatureBuf {}

/// Equality check between [`Signature`] and [`SignatureBuf`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(SignatureBuf::empty(), *Signature::EMPTY);
/// assert_eq!(SignatureBuf::new(b"s")?, *Signature::STRING);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
impl PartialEq<Signature> for SignatureBuf {
    #[inline]
    fn eq(&self, other: &Signature) -> bool {
        self.as_slice() == other.as_bytes()
    }
}

/// Equality check between a borrowed [`Signature`] and [`SignatureBuf`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuf};
///
/// assert_eq!(SignatureBuf::empty(), *Signature::EMPTY);
/// assert_eq!(SignatureBuf::new(b"s")?, *Signature::STRING);
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
impl PartialEq<&Signature> for SignatureBuf {
    #[inline]
    fn eq(&self, other: &&Signature) -> bool {
        self.as_slice() == other.as_bytes()
    }
}
