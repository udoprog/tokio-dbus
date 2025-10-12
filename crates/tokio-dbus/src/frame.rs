use crate::Signature;
use crate::proto::Endianness;

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A verbatim frame that can be stored and loaded from a buffer.
///
/// This is implemented for primitives `Copy` types such as `u32`.
///
/// # Safety
///
/// This asserts that the implementor is `repr(C)`, and can inhabit any bit
/// pattern.
///
/// Any type implementing `Frame` must have an alignment of at most `8`.
pub unsafe trait Frame: self::sealed::Sealed {
    /// The signature of the frame.
    #[doc(hidden)]
    const SIGNATURE: &'static Signature;

    /// Adjust the endianness of the frame.
    #[doc(hidden)]
    fn adjust(&mut self, endianness: Endianness);
}

impl self::sealed::Sealed for u8 {}

unsafe impl Frame for u8 {
    const SIGNATURE: &'static Signature = Signature::BYTE;

    #[inline]
    fn adjust(&mut self, _: Endianness) {}
}

impl_traits_for_frame!(u8);

impl self::sealed::Sealed for f64 {}

unsafe impl Frame for f64 {
    const SIGNATURE: &'static Signature = Signature::DOUBLE;

    #[inline]
    fn adjust(&mut self, endianness: Endianness) {
        if endianness != Endianness::NATIVE {
            *self = f64::from_bits(u64::swap_bytes(self.to_bits()));
        }
    }
}

impl_traits_for_frame!(f64);

macro_rules! impl_number {
    ($($ty:ty, $signature:ident),* $(,)?) => {
        $(
            impl self::sealed::Sealed for $ty {}

            unsafe impl Frame for $ty {
                const SIGNATURE: &'static Signature = Signature::$signature;

                #[inline]
                fn adjust(&mut self, endianness: Endianness) {
                    if endianness != Endianness::NATIVE {
                        *self = <$ty>::swap_bytes(*self);
                    }
                }
            }

            impl_traits_for_frame!($ty);
        )*
    }
}

impl_number!(i16, INT16, i32, INT32, i64, INT64);
impl_number!(u16, UINT16, u32, UINT32, u64, UINT64);
