use core::mem::MaybeUninit;
use core::ops::Deref;
use core::slice::from_raw_parts;

use super::{
    MAX_CONTAINER_DEPTH, MAX_DEPTH, MAX_SIGNATURE, Signature, SignatureBuf, SignatureError,
    SignatureErrorKind,
};

/// A D-Bus signature builder.
///
/// This ensures that the constructed signature doesn't violate maximum
/// requirements imposed by the D-Bus specification.
///
/// This is the owned variant which dereferences to [`Signature`].
#[derive(Clone)]
pub struct SignatureBuilder {
    data: [MaybeUninit<u8>; MAX_SIGNATURE],
    init: usize,
    structs: usize,
    arrays: usize,
}

impl SignatureBuilder {
    /// Construct a new empty signature.
    #[doc(hidden)]
    pub const fn new() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            init: 0,
            structs: 0,
            arrays: 0,
        }
    }

    /// Construct from an owned signature.
    #[doc(hidden)]
    pub fn from_owned_signature(signature: SignatureBuf) -> Self {
        let (data, init) = signature.into_raw_parts();

        Self {
            data,
            init,
            structs: 0,
            arrays: 0,
        }
    }

    /// Coerce into a signature.
    pub fn to_signature(&self) -> &Signature {
        // SAFETY: Construction of SignatureBuf ensures that the signature is
        // valid.
        unsafe { Signature::new_unchecked(self.as_slice()) }
    }

    /// Open an array in the signature.
    pub fn open_array(&mut self) -> Result<(), SignatureError> {
        if self.arrays == MAX_CONTAINER_DEPTH || self.structs + self.arrays == MAX_DEPTH {
            return Err(SignatureError::new(
                SignatureErrorKind::ExceededMaximumArrayRecursion,
            ));
        }

        if !self.push(b'a') {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        self.arrays += 1;
        Ok(())
    }

    /// Open close an array in the signature.
    pub fn close_array(&mut self) {
        self.arrays -= 1;
    }

    /// Open a struct in the signature.
    pub fn open_struct(&mut self) -> Result<(), SignatureError> {
        if self.structs == MAX_CONTAINER_DEPTH || self.structs + self.arrays == MAX_DEPTH {
            return Err(SignatureError::new(
                SignatureErrorKind::ExceededMaximumStructRecursion,
            ));
        }

        if !self.push(b'(') {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        self.structs += 1;
        Ok(())
    }

    /// Close a struct in the signature.
    pub fn close_struct(&mut self) -> Result<(), SignatureError> {
        if !self.push(b')') {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        self.structs -= 1;
        Ok(())
    }

    /// Push a single byte onto the signature.
    fn push(&mut self, byte: u8) -> bool {
        if self.init == MAX_SIGNATURE {
            return false;
        }

        unsafe {
            self.data
                .as_mut_ptr()
                .cast::<u8>()
                .add(self.init)
                .write(byte);
            self.init += 1;
        }

        true
    }

    /// Clear the current signature.
    pub fn clear(&mut self) {
        self.init = 0;
    }

    /// Extend this signature with another.
    #[must_use = "Return value must be observed to indicate an error"]
    pub fn extend_from_signature<S>(&mut self, other: S) -> bool
    where
        S: AsRef<Signature>,
    {
        let bytes = other.as_ref().as_bytes();

        if self.init + bytes.len() > MAX_SIGNATURE {
            return false;
        }

        unsafe {
            self.data
                .as_mut_ptr()
                .cast::<u8>()
                .add(self.init)
                .copy_from(bytes.as_ptr(), bytes.len());
            self.init += bytes.len();
        }

        true
    }

    #[inline]
    fn as_slice(&self) -> &[u8] {
        // SAFETY: init is set to the initialized slice.
        unsafe { from_raw_parts(self.data.as_ptr().cast(), self.init) }
    }
}

impl Deref for SignatureBuilder {
    type Target = Signature;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.to_signature()
    }
}

impl PartialEq<SignatureBuilder> for SignatureBuilder {
    #[inline]
    fn eq(&self, other: &SignatureBuilder) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for SignatureBuilder {}
