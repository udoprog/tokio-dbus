//! Types for dealing with buffers.

#[cfg(test)]
mod tests;

pub(crate) use self::aligned::Aligned;
mod aligned;

#[cfg(feature = "alloc")]
pub(crate) use self::aligned_buf::AlignedBuf;
#[cfg(feature = "alloc")]
mod aligned_buf;

#[cfg(feature = "alloc")]
pub(crate) use self::unaligned_buf::UnalignedBuf;
#[cfg(feature = "alloc")]
mod unaligned_buf;

#[cfg(feature = "tokio")]
pub(crate) use self::alloc::Alloc;
#[cfg(feature = "tokio")]
mod alloc;

/// The maximum length of an array in bytes.
pub(crate) const MAX_ARRAY_LENGTH: u32 = 1u32 << 26;

/// The maximum length of a body in bytes.
pub(crate) const MAX_BODY_LENGTH: u32 = 1u32 << 27;

use core::mem::align_of;

/// Calculate padding with the assumption that alignment is a power of two.
#[inline(always)]
pub(crate) fn padding_to<T>(len: usize) -> usize {
    // SAFETY: Alignment of `T` is always valid.
    unsafe { padding_to_with(align_of::<T>(), len) }
}

/// Calculate padding with the assumption that alignment is a power of two.
#[inline(always)]
unsafe fn padding_to_with(align: usize, len: usize) -> usize {
    let mask = align - 1;
    (align - (len & mask)) & mask
}

#[inline(always)]
const fn max_size_for_align(align: usize) -> usize {
    isize::MAX as usize - (align - 1)
}
