#[cfg(test)]
mod tests;

mod stack;

#[doc(inline)]
pub(crate) use tokio_dbus_core::signature::MAX_DEPTH;
#[doc(inline)]
pub use tokio_dbus_core::signature::{Signature, SignatureBuf, SignatureBuilder, SignatureError};

use crate::error::Result;

use crate::{Alignment, Body, Read, Write, WriteAligned, WriteUnaligned};

impl crate::write::sealed::Sealed for Signature {}

impl Write for Signature {
    const SIGNATURE: &'static Signature = Signature::SIGNATURE;

    #[inline]
    fn write_to<B>(&self, buf: &mut B)
    where
        B: ?Sized + WriteAligned,
    {
        buf.store_frame(self.len() as u8);
        buf.extend_from_slice_nul(self.as_bytes());
    }

    #[inline]
    fn write_to_unaligned<B>(&self, buf: &mut B)
    where
        B: ?Sized + WriteUnaligned,
    {
        buf.store(self.len() as u8);
        buf.extend_from_slice_nul(self.as_bytes());
    }
}

impl_traits_for_write!(Signature, Signature::new("us")?, "qg", Signature);

impl crate::read::sealed::Sealed for Signature {}

impl Read for Signature {
    #[inline]
    fn read_from<'de>(buf: &mut Body<'de>) -> Result<&'de Self> {
        let len = buf.load::<u8>()? as usize;
        let bytes = buf.load_slice_nul(len)?;
        Ok(Signature::new(bytes)?)
    }
}

/// The alignment of the type introduced by the given type code.
///
/// An absent type code cannot legally occur in a validated signature, but is
/// treated as being byte-aligned so that the caller doesn't have to handle it.
fn alignment_of(byte: Option<u8>) -> Alignment {
    use crate::proto::Type;

    let Some(byte) = byte else {
        return Alignment::BYTE;
    };

    match Type::new(byte) {
        Type::BYTE | Type::SIGNATURE | Type::VARIANT => Alignment::BYTE,
        Type::INT16 | Type::UINT16 => Alignment::U16,
        Type::INT64 | Type::UINT64 | Type::DOUBLE | Type::OPEN_PAREN | Type::OPEN_BRACE => {
            Alignment::U64
        }
        // NB: Covers `bihu`, `so` and nested arrays, all of which are aligned
        // to 4 bytes.
        _ => Alignment::U32,
    }
}

/// Skip over the values described by `this` in the read buffer.
///
/// `depth` is the number of containers (variants included) enclosing the
/// values, which together with the containers inside of them may not exceed
/// [`MAX_DEPTH`]. This bounds the recursion through nested variants.
pub(crate) fn skip(this: &Signature, read: &mut Body<'_>, depth: usize) -> Result<()> {
    use crate::error::{Error, ErrorKind};
    use crate::proto::Type;

    use self::stack::Stack;

    #[derive(Debug, Clone, Copy)]
    enum Step {
        Fixed(usize),
        StringNul,
        Variant,
        ByteNul,
    }

    let mut stack = Stack::<bool, MAX_DEPTH>::new();
    let mut arrays = 0;

    let bytes = this.as_bytes();

    for (n, &b) in bytes.iter().enumerate() {
        let t = Type::new(b);

        let step = match t {
            Type::BYTE => Step::Fixed(1),
            Type::BOOLEAN => Step::Fixed(1),
            Type::INT16 => Step::Fixed(2),
            Type::UINT16 => Step::Fixed(2),
            Type::INT32 => Step::Fixed(4),
            Type::UINT32 => Step::Fixed(4),
            Type::INT64 => Step::Fixed(8),
            Type::UINT64 => Step::Fixed(8),
            Type::DOUBLE => Step::Fixed(8),
            Type::STRING => Step::StringNul,
            Type::OBJECT_PATH => Step::StringNul,
            Type::SIGNATURE => Step::ByteNul,
            Type::VARIANT => Step::Variant,
            Type::UNIX_FD => Step::Fixed(4),
            Type::ARRAY => {
                if depth + stack.len >= MAX_DEPTH {
                    return Err(Error::new(ErrorKind::NestingTooDeep));
                }

                if arrays == 0 {
                    let len = read.load::<u32>()? as usize;
                    // The length prefix is followed by padding up to the
                    // alignment of the element type, which is not counted
                    // towards the length.
                    read.align_to(alignment_of(bytes.get(n + 1).copied()))?;
                    read.advance(len)?;
                }

                arrays += 1;
                stack.try_push(true);
                continue;
            }
            Type::OPEN_PAREN | Type::OPEN_BRACE => {
                if depth + stack.len >= MAX_DEPTH {
                    return Err(Error::new(ErrorKind::NestingTooDeep));
                }

                // NB: Structs and dict entries are aligned to 8 bytes. When
                // we're inside of an array the whole array has already been
                // skipped over, so there is nothing to align.
                if arrays == 0 {
                    read.align::<u64>()?;
                }

                stack.try_push(false);
                continue;
            }
            Type::CLOSE_PAREN => {
                stack.pop();
                Step::Fixed(0)
            }
            Type::CLOSE_BRACE => {
                stack.pop();
                Step::Fixed(0)
            }
            _ => unreachable!(),
        };

        let in_array = arrays > 0;

        // Unwind arrays.
        while let Some(true) = stack.peek() {
            arrays -= 1;
            stack.pop();
        }

        if in_array {
            continue;
        }

        match step {
            Step::Fixed(n) => {
                read.advance(n)?;
            }
            Step::StringNul => {
                let n = read.load::<u32>()? as usize;
                read.advance(n.saturating_add(1))?;
            }
            Step::ByteNul => {
                let n = read.load::<u8>()? as usize;
                read.advance(n.saturating_add(1))?;
            }
            Step::Variant => {
                if depth + stack.len >= MAX_DEPTH {
                    return Err(Error::new(ErrorKind::NestingTooDeep));
                }

                // NB: Reading the signature consumes the length prefix.
                let sig = read.read::<Signature>()?;
                skip(sig, read, depth + stack.len + 1)?;
            }
        }
    }

    Ok(())
}
