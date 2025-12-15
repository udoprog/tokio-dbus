use crate::signature::SignatureBuilder;
use crate::{Signature, Storable, WriteAligned};

/// A variant.
pub enum Variant<'de> {
    /// A string variant.
    String(&'de str),
    /// A u32 variant.
    U32(u32),
    /// A stored signature.
    Signature(&'de Signature),
}

impl crate::storable::sealed::Sealed for Variant<'_> {}

impl Storable for Variant<'_> {
    #[inline]
    fn store_to<B>(self, buf: &mut B)
    where
        B: ?Sized + WriteAligned,
    {
        match self {
            Variant::String(string) => {
                buf.write_only(Signature::STRING);
                buf.write_only(string);
            }
            Variant::Signature(signature) => {
                buf.write_only(Signature::SIGNATURE);
                buf.write_only(signature);
            }
            Variant::U32(number) => {
                buf.write_only(Signature::UINT32);
                buf.store_frame(number);
            }
        }
    }

    #[inline]
    fn write_signature(builder: &mut SignatureBuilder) -> bool {
        builder.extend_from_signature(Signature::VARIANT)
    }
}
