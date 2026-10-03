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

#[cfg(feature = "alloc")]
pub(crate) use self::alloc::Alloc;
#[cfg(feature = "alloc")]
mod alloc;

/// The maximum length of an array in bytes.
pub(crate) const MAX_ARRAY_LENGTH: u32 = 1u32 << 26;

/// The maximum length of a body in bytes.
#[cfg(feature = "tokio")]
pub(crate) const MAX_BODY_LENGTH: u32 = 1u32 << 27;

use core::mem::align_of;

use crate::Frame;

/// The alignment of the start of every aligned buffer, which is the largest
/// alignment D-Bus uses. It is fixed rather than taken from `align_of::<u64>()`,
/// which is 4 on some 32-bit targets.
pub(crate) const BUF_ALIGN: usize = 8;

/// Zero-sized type with the alignment of [`BUF_ALIGN`], used for dangling
/// pointers to empty buffers.
#[repr(align(8))]
pub(crate) struct BufAlign;

const _: () = assert!(align_of::<BufAlign>() == BUF_ALIGN);

/// Calculate the padding needed to align `len` to the D-Bus alignment of `T`.
#[inline(always)]
pub(crate) fn padding_to<T>(len: usize) -> usize
where
    T: Frame,
{
    const {
        // Buffers read and write frames in place at offsets padded to
        // `T::ALIGNMENT` from a `BUF_ALIGN`-aligned start, so that must cover
        // the target's own alignment of `T`.
        assert!(T::ALIGNMENT.in_bytes() >= align_of::<T>());
        assert!(T::ALIGNMENT.in_bytes() <= BUF_ALIGN);
    }

    // SAFETY: `Alignment::in_bytes` is always a non-zero power of two.
    unsafe { padding_to_with(T::ALIGNMENT.in_bytes(), len) }
}

/// Calculate padding with the assumption that alignment is a power of two.
///
/// # Safety
///
/// The caller must ensure that `align` is a non-zero power of two.
#[inline(always)]
pub(crate) unsafe fn padding_to_with(align: usize, len: usize) -> usize {
    let mask = align - 1;
    (align - (len & mask)) & mask
}

#[cfg(feature = "alloc")]
#[inline(always)]
const fn max_size_for_align(align: usize) -> usize {
    isize::MAX as usize - (align - 1)
}
