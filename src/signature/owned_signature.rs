use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

use crate::Signature;

/// A D-Bus signature.
///
/// This is the owned variant which dereferences to [`Signature`].
#[derive(Clone, PartialEq, Eq)]
pub struct OwnedSignature(Vec<u8>);

impl OwnedSignature {
    /// An empty owned signature.
    pub(crate) const EMPTY: Self = OwnedSignature::new();
}

impl OwnedSignature {
    /// Construct a new empty signature.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::OwnedSignature;
    ///
    /// let sig = OwnedSignature::new();
    /// assert!(sig.is_empty());
    /// ```
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Push a single byte onto the signature.
    pub(crate) fn push(&mut self, byte: u8) {
        self.0.push(byte);
    }

    /// Clear the current signature.
    pub(crate) fn clear(&mut self) {
        self.0.clear();
    }

    /// Construct directly from a vector.
    ///
    /// # Safety
    ///
    /// Caller must ensure that this is a valid signature.
    pub(crate) unsafe fn from_vec(signature: Vec<u8>) -> Self {
        Self(signature)
    }

    /// Extend this signature with another.
    pub(crate) fn extend_from_signature<S>(&mut self, other: S)
    where
        S: AsRef<Signature>,
    {
        self.0.extend_from_slice(other.as_ref().as_bytes());
    }
}

impl fmt::Debug for OwnedSignature {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("OwnedSignature")
            .field(&self.as_str())
            .finish()
    }
}

impl Deref for OwnedSignature {
    type Target = Signature;

    fn deref(&self) -> &Self::Target {
        // SAFETY: Construction of OwnedSignature ensures that the signature is
        // valid.
        unsafe { Signature::new_unchecked(&self.0) }
    }
}

impl Borrow<Signature> for OwnedSignature {
    #[inline]
    fn borrow(&self) -> &Signature {
        self
    }
}

impl AsRef<Signature> for OwnedSignature {
    #[inline]
    fn as_ref(&self) -> &Signature {
        self
    }
}

/// Equality check between [`Signature`] and [`OwnedSignature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, OwnedSignature};
///
/// assert_eq!(OwnedSignature::new(), *Signature::EMPTY);
/// assert_eq!(Signature::STRING.to_owned(), *Signature::STRING);
/// ```
impl PartialEq<Signature> for OwnedSignature {
    #[inline]
    fn eq(&self, other: &Signature) -> bool {
        self.0 == other.as_bytes()
    }
}

/// Equality check between a borrowed [`Signature`] and [`OwnedSignature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, OwnedSignature};
///
/// assert_eq!(OwnedSignature::new(), *Signature::EMPTY);
/// assert_eq!(Signature::STRING.to_owned(), *Signature::STRING);
/// ```
impl PartialEq<&Signature> for OwnedSignature {
    #[inline]
    fn eq(&self, other: &&Signature) -> bool {
        self.0 == other.as_bytes()
    }
}
