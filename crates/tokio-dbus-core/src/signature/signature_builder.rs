use core::mem::MaybeUninit;
use core::ops::Deref;
use core::slice::from_raw_parts;

use super::stack::Stack;
use super::validation::Validator;
use super::{
    MAX_CONTAINER_DEPTH, MAX_SIGNATURE, MAX_SIGNATURE_LEN, Signature, SignatureBuf, SignatureError,
    SignatureErrorKind,
};

/// A D-Bus signature builder.
///
/// Every operation checks that the signature being built can still be completed
/// into a valid one, and leaves the builder unchanged when it returns an error.
/// [`to_signature()`] (and dereferencing) gives the complete types written so
/// far, leaving out any container which is still open.
///
/// [`to_signature()`]: Self::to_signature
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, SignatureBuilder};
///
/// let mut builder = SignatureBuilder::new();
/// builder.open_array()?;
/// builder.open_dict()?;
/// builder.try_extend_from_signature(Signature::STRING)?;
/// assert_eq!(builder.to_signature(), "");
/// builder.try_extend_from_signature(Signature::VARIANT)?;
/// builder.close_dict()?;
/// builder.close_array()?;
/// assert_eq!(builder.to_signature(), "a{sv}");
///
/// builder.open_struct()?;
/// assert!(builder.close_struct().is_err());
/// assert_eq!(builder.to_signature(), "a{sv}");
/// # Ok::<_, tokio_dbus::SignatureError>(())
/// ```
#[derive(Clone)]
pub struct SignatureBuilder {
    data: [MaybeUninit<u8>; MAX_SIGNATURE],
    /// The number of bytes written.
    init: usize,
    /// The length of the longest prefix which consists of complete types.
    complete: usize,
    validator: Validator,
    /// The validator depth at which each array which has not been closed
    /// through [`SignatureBuilder::close_array`] was opened.
    arrays: Stack<u8, MAX_CONTAINER_DEPTH>,
}

impl SignatureBuilder {
    /// Construct a new empty signature.
    #[doc(hidden)]
    pub const fn new() -> Self {
        Self {
            data: [MaybeUninit::uninit(); MAX_SIGNATURE],
            init: 0,
            complete: 0,
            validator: Validator::new(),
            arrays: Stack::new(),
        }
    }

    /// Construct from an owned signature.
    #[doc(hidden)]
    pub fn from_owned_signature(signature: SignatureBuf) -> Self {
        let (data, init) = signature.into_raw_parts();

        // NB: A valid signature is a sequence of complete types, which leaves
        // the validator with nothing open.
        Self {
            data,
            init,
            complete: init,
            validator: Validator::new(),
            arrays: Stack::new(),
        }
    }

    /// Coerce into a signature.
    ///
    /// This is made up of the complete types written so far, so a container
    /// which is still open is not included.
    pub fn to_signature(&self) -> &Signature {
        // SAFETY: Every byte written has been accepted by the validator, and
        // `complete` only covers a prefix after which the validator had no open
        // container, which makes it a valid signature.
        unsafe { Signature::new_unchecked(&self.as_slice()[..self.complete]) }
    }

    /// Open an array in the signature.
    ///
    /// It must be closed with [`close_array()`] once its element type has been
    /// written.
    ///
    /// [`close_array()`]: Self::close_array
    pub fn open_array(&mut self) -> Result<(), SignatureError> {
        let Ok(depth) = u8::try_from(self.validator.depth()) else {
            return Err(SignatureError::new(
                SignatureErrorKind::ExceededMaximumArrayRecursion,
            ));
        };

        if self.arrays.len == self.arrays.capacity() {
            return Err(SignatureError::new(
                SignatureErrorKind::ExceededMaximumArrayRecursion,
            ));
        }

        self.push(b"a")?;
        stack_try_push!(self.arrays, depth);
        Ok(())
    }

    /// Close an array in the signature.
    ///
    /// This fails if the innermost open container is not an array whose
    /// element type has been written in full.
    pub fn close_array(&mut self) -> Result<(), SignatureError> {
        let Some(&depth) = stack_peek!(self.arrays) else {
            return Err(SignatureError::new(
                SignatureErrorKind::ArrayEndedButNotStarted,
            ));
        };

        // NB: The validator closes an array as soon as its element type is
        // complete, along with every array directly enclosing it, which brings
        // it back to or below the depth the array was opened at.
        if self.validator.depth() > usize::from(depth) {
            return Err(SignatureError::new(
                SignatureErrorKind::MissingArrayElementType,
            ));
        }

        stack_pop!(self.arrays, u8);
        Ok(())
    }

    /// Open a struct in the signature.
    ///
    /// A struct must have at least one field.
    pub fn open_struct(&mut self) -> Result<(), SignatureError> {
        self.push(b"(")
    }

    /// Close a struct in the signature.
    pub fn close_struct(&mut self) -> Result<(), SignatureError> {
        self.push(b")")
    }

    /// Open a dict entry in the signature.
    ///
    /// A dict entry is only legal directly inside of an array, and must be
    /// populated with exactly two fields where the first one is a basic type.
    pub fn open_dict(&mut self) -> Result<(), SignatureError> {
        if !self.validator.in_array() {
            return Err(SignatureError::new(
                SignatureErrorKind::DictEntryNotInsideArray,
            ));
        }

        self.push(b"{")
    }

    /// Close a dict entry in the signature.
    pub fn close_dict(&mut self) -> Result<(), SignatureError> {
        self.push(b"}")
    }

    /// Push type codes onto the signature, leaving it unchanged on error.
    fn push(&mut self, bytes: &[u8]) -> Result<(), SignatureError> {
        let mut validator = self.validator;

        for &b in bytes {
            validator.push(b)?;
        }

        if bytes.len() > MAX_SIGNATURE_LEN - self.init {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        // SAFETY: The check above ensures that `init + bytes.len()` is within
        // the `MAX_SIGNATURE` bytes of `data`.
        unsafe {
            self.data
                .as_mut_ptr()
                .cast::<u8>()
                .add(self.init)
                .copy_from_nonoverlapping(bytes.as_ptr(), bytes.len());
        }

        self.init += bytes.len();
        self.validator = validator;

        if validator.depth() == 0 {
            self.complete = self.init;
        }

        Ok(())
    }

    /// Clear the current signature.
    pub fn clear(&mut self) {
        self.init = 0;
        self.complete = 0;
        self.validator = Validator::new();
        self.arrays = Stack::new();
    }

    /// Extend this signature with another.
    ///
    /// Returns `false` if the signature would grow too long, or if `other` is
    /// not allowed where it is being written, such as a dict entry key which is
    /// not a basic type. See [`try_extend_from_signature()`] for the reason.
    ///
    /// [`try_extend_from_signature()`]: Self::try_extend_from_signature
    #[must_use = "Return value must be observed to indicate an error"]
    pub fn extend_from_signature<S>(&mut self, other: S) -> bool
    where
        S: AsRef<Signature>,
    {
        self.try_extend_from_signature(other).is_ok()
    }

    /// Extend this signature with another, returning why that is not possible
    /// if it fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Signature, SignatureBuilder};
    ///
    /// let mut builder = SignatureBuilder::new();
    /// builder.open_array()?;
    /// builder.open_dict()?;
    /// assert!(builder.try_extend_from_signature(Signature::new("ai")?).is_err());
    /// # Ok::<_, tokio_dbus::SignatureError>(())
    /// ```
    pub fn try_extend_from_signature<S>(&mut self, other: S) -> Result<(), SignatureError>
    where
        S: AsRef<Signature>,
    {
        self.push(other.as_ref().as_bytes())
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
        // NB: The rest of the state is determined by the bytes written, apart
        // from which arrays have been closed through `close_array`.
        self.as_slice() == other.as_slice()
    }
}

impl Eq for SignatureBuilder {}
