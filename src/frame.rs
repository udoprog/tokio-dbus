use crate::protocol::Endianness;

/// A message frame in the protocol.
///
/// # Safety
///
/// This asserts that the implementor is `repr(C)`, and can inhabit any bit
/// pattern.
///
/// Any type implementing `Frame` must have an alignment of at most `8`.
pub(crate) unsafe trait Frame {
    /// Adjust the endianness of the frame.
    fn adjust(&mut self, endianness: Endianness);
}

unsafe impl Frame for u8 {
    #[inline]
    fn adjust(&mut self, _: Endianness) {}
}

unsafe impl Frame for i8 {
    #[inline]
    fn adjust(&mut self, _: Endianness) {}
}

macro_rules! impl_number {
    ($($ty:ty),* $(,)?) => {
        $(
            unsafe impl Frame for $ty {
                #[inline]
                fn adjust(&mut self, endianness: Endianness) {
                    if endianness != Endianness::NATIVE {
                        *self = <$ty>::swap_bytes(*self);
                    }
                }
            }
        )*
    }
}

impl_number!(u16, u32, u64, u128);
impl_number!(i16, i32, i64, i128);
