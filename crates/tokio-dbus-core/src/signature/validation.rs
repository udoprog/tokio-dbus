use crate::proto::Type;

use super::stack::{Stack, StackValue};
use super::{MAX_CONTAINER_DEPTH, MAX_DEPTH, SignatureError, SignatureErrorKind};

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

#[allow(unused_assignments)]
pub(super) const fn validate(bytes: &[u8]) -> Result<(), SignatureError> {
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
            t => return Err(SignatureError::new(UnknownTypeCode(t))),
        };

        while let Some((Kind::Array, _)) = stack_peek!(stack) {
            stack_pop!(stack, (Kind, u8));
            is_basic = false;
        }

        if let Some((Kind::Dict, 0)) = stack_peek!(stack)
            && !is_basic
        {
            return Err(SignatureError::new(DictKeyMustBeBasicType));
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
