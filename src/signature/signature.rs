use std::fmt;
use std::str::from_utf8_unchecked;

use crate::buf::BufMut;
use crate::error::Result;
use crate::protocol::Type;
use crate::signature::SignatureErrorKind;
use crate::stack::{Stack, StackValue};
use crate::OwnedSignature;
use crate::{Read, ReadBuf, Write};

use super::SignatureError;

/// The maximum individual container depth.
const MAX_CONTAINER_DEPTH: usize = 32;

/// The maximum total depth of any containers.
const MAX_DEPTH: usize = MAX_CONTAINER_DEPTH * 2;

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
    pub const EMPTY: &'static Signature = Signature::new_const(b"");

    /// A signature.
    pub const SIGNATURE: &'static Signature = Signature::new_const(b"g");

    /// A object path.
    pub const OBJECT_PATH: &'static Signature = Signature::new_const(b"o");

    /// A string.
    pub const STRING: &'static Signature = Signature::new_const(b"s");

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

    /// Construct a new empty signature.
    pub const fn empty() -> &'static Self {
        unsafe { Self::new_unchecked(&[]) }
    }

    /// Test if the signature is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

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
    pub const fn new(signature: &[u8]) -> Result<&Signature, SignatureError> {
        if let Err(error) = validate(signature) {
            return Err(error);
        };

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
    pub(crate) const unsafe fn new_unchecked(signature: &[u8]) -> &Self {
        &*(signature as *const _ as *const Signature)
    }

    /// Get the signature as a string.
    pub(crate) fn as_str(&self) -> &str {
        // SAFETY: Validation indirectly ensures that the signature is valid UTF-8.
        unsafe { from_utf8_unchecked(&self.0) }
    }

    /// Get the signature as a byte slice.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Skip over the current signature in the read buffer.
    pub(crate) fn skip(&self, read: &mut ReadBuf<'_>) -> Result<()> {
        for &b in self.0.iter() {
            let n = match Type(b) {
                Type::BYTE => 1,
                Type::BOOLEAN => 1,
                Type::INT16 => 2,
                Type::UINT16 => 2,
                Type::INT32 => 4,
                Type::UINT32 => 4,
                Type::INT64 => 8,
                Type::UINT64 => 8,
                Type::DOUBLE => 8,
                Type::STRING => read.load::<u32>()? as usize + 1,
                Type::OBJECT_PATH => read.load::<u32>()? as usize + 1,
                Type::SIGNATURE => read.load::<u8>()? as usize + 1,
                Type::VARIANT => {
                    read.load::<u8>()?;
                    let sig = read.read::<Self>()?;
                    sig.skip(read)?;
                    continue;
                }
                Type::UNIX_FD => 4,
                Type::ARRAY => read.load::<u32>()? as usize,
                Type::OPEN_PAREN | Type::CLOSE_PAREN => {
                    continue;
                }
                // NB: At this stage, the type signature has been validated, so
                // we cannot encounter unknown sequences.
                _ => unreachable!(),
            };

            read.advance(n)?;
        }

        Ok(())
    }
}

impl fmt::Debug for Signature {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Signature").field(&self.as_str()).finish()
    }
}

impl Write for Signature {
    const SIGNATURE: &'static Signature = Signature::SIGNATURE;

    #[inline]
    fn write_to<O: ?Sized>(&self, buf: &mut O)
    where
        O: BufMut,
    {
        buf.store(self.0.len() as u8);
        buf.extend_from_slice_nul(&self.0);
    }
}

impl Read for Signature {
    #[inline]
    fn read_from<'de>(buf: &mut ReadBuf<'de>) -> Result<&'de Self> {
        let len = buf.load::<u8>()? as usize;
        let bytes = buf.load_slice_nul(len)?;
        Ok(Signature::new(bytes)?)
    }
}

impl AsRef<Signature> for Signature {
    #[inline]
    fn as_ref(&self) -> &Signature {
        self
    }
}

impl ToOwned for Signature {
    type Owned = OwnedSignature;

    #[inline]
    fn to_owned(&self) -> Self::Owned {
        // SAFETY: This type ensures that it contains a valid signature.
        unsafe { OwnedSignature::from_vec(self.0.to_vec()) }
    }
}

/// Equality check between [`OwnedSignature`] and [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, OwnedSignature};
///
/// assert_eq!(*Signature::EMPTY, OwnedSignature::new());
/// assert_eq!(*Signature::STRING, Signature::STRING.to_owned());
/// ```
impl PartialEq<OwnedSignature> for Signature {
    #[inline]
    fn eq(&self, other: &OwnedSignature) -> bool {
        self.0 == other.0
    }
}

/// Equality check between [`OwnedSignature`] and a borrowed [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, OwnedSignature};
///
/// assert_eq!(Signature::EMPTY, OwnedSignature::new());
/// assert_eq!(Signature::STRING, Signature::STRING.to_owned());
/// ```
impl PartialEq<OwnedSignature> for &Signature {
    #[inline]
    fn eq(&self, other: &OwnedSignature) -> bool {
        self.0 == other.0
    }
}

/// Equality check between [`[u8]`] and a [`Signature`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Signature, OwnedSignature};
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
/// use tokio_dbus::{Signature, OwnedSignature};
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
/// use tokio_dbus::{Signature, OwnedSignature};
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
/// use tokio_dbus::{Signature, OwnedSignature};
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

#[derive(Default, Debug, Clone, Copy)]
#[repr(u8)]
enum Kind {
    #[default]
    None,
    Array,
    Struct,
    Dict,
}

impl StackValue for (Kind, u8) {
    const DEFAULT: Self = (Kind::None, 0);
}

const fn validate(bytes: &[u8]) -> Result<(), SignatureError> {
    use SignatureErrorKind::*;

    if bytes.len() > u8::MAX as usize {
        return Err(SignatureError::new(SignatureTooLong));
    }

    let mut stack = Stack::<(Kind, u8), MAX_DEPTH>::new();
    let mut arrays = 0;
    let mut structs = 0;
    let mut n = 0;

    while n < bytes.len() {
        let b = bytes[n];
        n += 1;
        let t = Type(b);

        let mut is_basic = match t {
            Type::BYTE => true,
            Type::BOOLEAN => true,
            Type::INT16 => true,
            Type::UINT16 => true,
            Type::INT32 => true,
            Type::UINT32 => true,
            Type::INT64 => true,
            Type::UINT64 => true,
            Type::DOUBLE => true,
            Type::STRING => true,
            Type::OBJECT_PATH => true,
            Type::SIGNATURE => true,
            Type::VARIANT => true,
            Type::UNIX_FD => true,
            Type::ARRAY => {
                if !stack_try_push!(stack, (Kind::Array, 0)) || arrays == MAX_CONTAINER_DEPTH {
                    return Err(SignatureError::new(ExceededMaximumArrayRecursion));
                }

                arrays += 1;
                continue;
            }
            Type::OPEN_PAREN => {
                if !stack_try_push!(stack, (Kind::Struct, 0)) || structs == MAX_CONTAINER_DEPTH {
                    return Err(SignatureError::new(ExceededMaximumStructRecursion));
                }

                structs += 1;
                continue;
            }
            Type::CLOSE_PAREN => {
                let n = match stack_pop!(stack, (Kind, u8)) {
                    Some((Kind::Struct, n)) => n,
                    Some((Kind::Array, _)) => {
                        return Err(SignatureError::new(MissingArrayElementType));
                    }
                    _ => {
                        return Err(SignatureError::new(StructEndedButNotStarted));
                    }
                };

                if n == 0 {
                    return Err(SignatureError::new(StructHasNoFields));
                }

                structs -= 1;
                false
            }
            Type::OPEN_BRACE => {
                if !stack_try_push!(stack, (Kind::Dict, 0)) {
                    return Err(SignatureError::new(ExceededMaximumDictRecursion));
                }

                continue;
            }
            Type::CLOSE_BRACE => {
                let n = match stack_pop!(stack, (Kind, u8)) {
                    Some((Kind::Dict, n)) => n,
                    Some((Kind::Array, _)) => {
                        return Err(SignatureError::new(MissingArrayElementType));
                    }
                    _ => {
                        return Err(SignatureError::new(DictEndedButNotStarted));
                    }
                };

                match n {
                    0 => {
                        return Err(SignatureError::new(DictEntryHasNoFields));
                    }
                    1 => {
                        return Err(SignatureError::new(DictEntryHasOnlyOneField));
                    }
                    2 => {}
                    _ => {
                        return Err(SignatureError::new(DictEntryHasTooManyFields));
                    }
                }

                if !matches!(stack_peek!(stack), Some((Kind::Array, _))) {
                    return Err(SignatureError::new(DictEntryNotInsideArray));
                }

                false
            }
            t => return Err(SignatureError::new(UnknownTypeCode(t.0))),
        };

        while let Some((Kind::Array, _)) = stack_peek!(stack) {
            stack_pop!(stack, (Kind, u8));
            is_basic = false;
        }

        if let Some((Kind::Dict, 0)) = stack_peek!(stack) {
            if !is_basic {
                return Err(SignatureError::new(DictKeyMustBeBasicType));
            }
        }

        if let Some((kind, n)) = stack_pop!(stack, (Kind, u8)) {
            stack_try_push!(stack, (kind, n + 1));
        }
    }

    match stack_pop!(stack, (Kind, u8)) {
        Some((Kind::Array, _)) => {
            return Err(SignatureError::new(MissingArrayElementType));
        }
        Some((Kind::Struct, _)) => {
            return Err(SignatureError::new(StructStartedButNotEnded));
        }
        Some((Kind::Dict, _)) => {
            return Err(SignatureError::new(DictStartedButNotEnded));
        }
        _ => {}
    }

    Ok(())
}
