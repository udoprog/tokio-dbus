use crate::signature::SignatureBuilder;
use crate::{BodyBuf, Signature, Write};

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

impl<T> self::sealed::Sealed for &T where T: ?Sized + Write {}

impl<T> Storable for &T
where
    T: ?Sized + Write,
{
    #[inline]
    fn store_to(self, buf: &mut BodyBuf) {
        buf.write_only(self);
    }

    #[inline]
    fn write_signature(builder: &mut SignatureBuilder) -> bool {
        builder.extend_from_signature(T::SIGNATURE)
    }
}

impl self::sealed::Sealed for u8 {}

impl Storable for u8 {
    #[inline]
    fn store_to(self, buf: &mut BodyBuf) {
        buf.store_frame(self)
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> bool {
        signature.extend_from_signature(Signature::BYTE)
    }
}

impl self::sealed::Sealed for f64 {}

impl Storable for f64 {
    #[inline]
    fn store_to(self, buf: &mut BodyBuf) {
        buf.store_frame(self)
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> bool {
        signature.extend_from_signature(Signature::DOUBLE)
    }
}

macro_rules! impl_number {
    ($($ty:ty, $signature:ident),* $(,)?) => {
        $(
            impl self::sealed::Sealed for $ty {}

            impl Storable for $ty {
                #[inline]
                fn store_to(self, buf: &mut BodyBuf) {
                    buf.store_frame(self)
                }

                #[inline]
                fn write_signature(signature: &mut SignatureBuilder) -> bool {
                    signature.extend_from_signature(Signature::$signature)
                }
            }
        )*
    }
}

impl_number!(i16, INT16, i32, INT32, i64, INT64);
impl_number!(u16, UINT16, u32, UINT32, u64, UINT64);
