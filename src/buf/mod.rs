//! Types for dealing with buffers.

#[cfg(test)]
mod tests;

pub use self::body::{
    ArrayReader, ArrayWriter, StructReader, StructWriter, TypedArrayWriter, TypedStructWriter,
};
mod body;

pub use self::body_read_buf::BodyReadBuf;
mod body_read_buf;

pub use self::read_buf::ReadBuf;
mod read_buf;

pub(crate) use self::aligned_buf::AlignedBuf;
mod aligned_buf;

pub(crate) use self::unaligned_buf::UnalignedBuf;
mod unaligned_buf;

pub use self::body_buf::BodyBuf;
mod body_buf;

pub use self::buf::Buf;
mod buf;

pub use self::buf_mut::{Alloc, BufMut};
mod buf_mut;

pub use self::send_buf::SendBuf;
mod send_buf;

pub use self::recv_buf::RecvBuf;
mod recv_buf;

pub(crate) use self::message_ref::MessageRef;
mod message_ref;

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
