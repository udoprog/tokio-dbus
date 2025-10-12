#[cfg(test)]
mod tests;

#[macro_use]
mod stack;

#[doc(inline)]
pub(crate) use tokio_dbus_core::signature::{MAX_DEPTH, SignatureBuilder};
#[doc(inline)]
pub use tokio_dbus_core::signature::{Signature, SignatureBuf, SignatureError};

use crate::buf::UnalignedBuf;
use crate::error::Result;

use crate::{Body, BodyBuf, Read, Write};

impl crate::write::sealed::Sealed for Signature {}

impl Write for Signature {
    const SIGNATURE: &'static Signature = Signature::SIGNATURE;

    #[inline]
    fn write_to(&self, buf: &mut BodyBuf) {
        buf.store_frame(self.len() as u8);
        buf.extend_from_slice_nul(self.as_bytes());
    }

    #[inline]
    fn write_to_unaligned(&self, buf: &mut UnalignedBuf) {
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

/// Return the stride needed to skip over read buffer.
pub(crate) fn skip(this: &Signature, read: &mut Body<'_>) -> Result<()> {
    use crate::proto::Type;

    #[derive(Debug, Clone, Copy)]
    enum Step {
        Fixed(usize),
        StringNul,
        Variant,
        ByteNul,
    }

    let mut stack = self::stack::Stack::<bool, MAX_DEPTH>::new();
    let mut arrays = 0;

    for &b in this.as_bytes() {
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
                if arrays == 0 {
                    let n = read.load::<u32>()? as usize;
                    read.advance(n)?;
                }

                arrays += 1;
                stack.try_push(true);
                continue;
            }
            Type::OPEN_PAREN => {
                stack.try_push(false);
                continue;
            }
            Type::CLOSE_PAREN => {
                stack.pop();
                Step::Fixed(0)
            }
            Type::OPEN_BRACE => {
                stack.try_push(false);
                continue;
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
                let _ = read.load::<u8>()?;
                let sig = read.read::<Signature>()?;
                skip(sig, read)?;
            }
        }
    }

    Ok(())
}
