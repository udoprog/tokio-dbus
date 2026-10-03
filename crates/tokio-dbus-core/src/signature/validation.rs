use crate::proto::Type;

use super::stack::{Stack, StackValue};
use super::{
    MAX_CONTAINER_DEPTH, MAX_DEPTH, MAX_SIGNATURE_LEN, SignatureError, SignatureErrorKind,
};

#[derive(Default, Debug, Clone, Copy)]
#[repr(u8)]
pub(super) enum Kind {
    #[default]
    None,
    Array,
    Struct,
    Dict,
}

impl StackValue for (Kind, u8) {
    const DEFAULT: Self = (Kind::None, 0);
}

impl StackValue for Kind {
    const DEFAULT: Self = Kind::None;
}

/// Validate a complete signature.
pub(super) const fn validate(bytes: &[u8]) -> Result<(), SignatureError> {
    if bytes.len() > MAX_SIGNATURE_LEN {
        return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
    }

    let mut validator = Validator::new();
    let mut n = 0;

    while n < bytes.len() {
        if let Err(error) = validator.push(bytes[n]) {
            return Err(error);
        }

        n += 1;
    }

    validator.finish()
}

/// Incremental signature validation, one type code at a time.
///
/// Every prefix accepted by [`Validator::push`] can be completed into a valid
/// signature, length aside, and [`Validator::depth`] is zero exactly when the prefix pushed so
/// far is a sequence of complete types.
#[derive(Clone, Copy)]
pub(super) struct Validator {
    stack: Stack<(Kind, u8), MAX_DEPTH>,
    arrays: usize,
    structs: usize,
}

impl Validator {
    pub(super) const fn new() -> Self {
        Self {
            stack: Stack::new(),
            arrays: 0,
            structs: 0,
        }
    }

    /// The number of containers which are currently open.
    #[inline]
    pub(super) const fn depth(&self) -> usize {
        self.stack.len
    }

    /// Test if the innermost open container is an array.
    #[inline]
    pub(super) const fn in_array(&self) -> bool {
        matches!(stack_peek!(self.stack), Some((Kind::Array, _)))
    }

    /// Push a single type code.
    ///
    /// On error the validator may be left in an inconsistent state and must be
    /// discarded.
    #[allow(unused_assignments)]
    pub(super) const fn push(&mut self, b: u8) -> Result<(), SignatureError> {
        use SignatureErrorKind::*;

        // NB: Reject a dict entry field as soon as it starts, so that a prefix
        // which has been accepted can always be completed.
        if let Some((Kind::Dict, n)) = stack_peek!(self.stack) {
            if *n >= 2 && b != b'}' {
                return Err(SignatureError::new(DictEntryHasTooManyFields));
            }

            if *n == 0 && matches!(b, b'a' | b'(' | b'{' | b'v') {
                return Err(SignatureError::new(DictKeyMustBeBasicType));
            }
        }

        let t = Type::new(b);

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
                if self.arrays == MAX_CONTAINER_DEPTH
                    || !stack_try_push!(self.stack, (Kind::Array, 0))
                {
                    return Err(SignatureError::new(ExceededMaximumArrayRecursion));
                }

                self.arrays += 1;
                return Ok(());
            }
            Type::OPEN_PAREN => {
                if self.structs == MAX_CONTAINER_DEPTH
                    || !stack_try_push!(self.stack, (Kind::Struct, 0))
                {
                    return Err(SignatureError::new(ExceededMaximumStructRecursion));
                }

                self.structs += 1;
                return Ok(());
            }
            Type::CLOSE_PAREN => {
                let n = match stack_pop!(self.stack, (Kind, u8)) {
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

                self.structs -= 1;
                false
            }
            Type::OPEN_BRACE => {
                if !stack_try_push!(self.stack, (Kind::Dict, 0)) {
                    return Err(SignatureError::new(ExceededMaximumDictRecursion));
                }

                return Ok(());
            }
            Type::CLOSE_BRACE => {
                let n = match stack_pop!(self.stack, (Kind, u8)) {
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

                if !self.in_array() {
                    return Err(SignatureError::new(DictEntryNotInsideArray));
                }

                false
            }
            t => return Err(SignatureError::new(UnknownTypeCode(t))),
        };

        // NB: A complete type closes every array directly enclosing it.
        while let Some((Kind::Array, _)) = stack_peek!(self.stack) {
            stack_pop!(self.stack, (Kind, u8));
            self.arrays -= 1;
            is_basic = false;
        }

        if let Some((Kind::Dict, 0)) = stack_peek!(self.stack)
            && !is_basic
        {
            return Err(SignatureError::new(DictKeyMustBeBasicType));
        }

        if let Some((kind, n)) = stack_pop!(self.stack, (Kind, u8)) {
            stack_try_push!(self.stack, (kind, n + 1));
        }

        Ok(())
    }

    /// Check that no container is left open.
    pub(super) const fn finish(&self) -> Result<(), SignatureError> {
        use SignatureErrorKind::*;

        match stack_peek!(self.stack) {
            Some((Kind::Array, _)) => Err(SignatureError::new(MissingArrayElementType)),
            Some((Kind::Struct, _)) => Err(SignatureError::new(StructStartedButNotEnded)),
            Some((Kind::Dict, _)) => Err(SignatureError::new(DictStartedButNotEnded)),
            _ => Ok(()),
        }
    }
}
