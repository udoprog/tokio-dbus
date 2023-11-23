use crate::signature::SignatureBuilder;
use crate::{BodyBuf, Signature};

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Trait used for types which can be stored with a `store()` call.
pub trait Storable: self::sealed::Sealed {
    /// Store a frame into a buffer body.
    #[doc(hidden)]
    fn store_to(self, buf: &mut BodyBuf);

    /// Write a signature.
    #[doc(hidden)]
    fn write_signature(builder: &mut SignatureBuilder) -> bool;
}

impl self::sealed::Sealed for String {}

impl Storable for String {
    #[inline]
    fn store_to(self, buf: &mut BodyBuf) {
        self.as_str().store_to(buf);
    }

    #[inline]
    fn write_signature(builder: &mut SignatureBuilder) -> bool {
        builder.extend_from_signature(Signature::STRING)
    }
}
