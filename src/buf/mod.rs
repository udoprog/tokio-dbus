//! Types for dealing with buffers.

#[cfg(test)]
mod tests;

pub use self::typed_array_writer::TypedArrayWriter;
mod typed_array_writer;

pub use self::typed_struct_writer::TypedStructWriter;
mod typed_struct_writer;

pub use self::read_buf::ReadBuf;
mod read_buf;

use self::array_writer::ArrayWriter;
mod array_writer;

pub use self::array_reader::ArrayReader;
mod array_reader;

pub use self::struct_writer::StructWriter;
mod struct_writer;

pub(crate) use self::owned_buf::OwnedBuf;
mod owned_buf;

pub use self::struct_reader::StructReader;
mod struct_reader;

pub use self::body_buf::BodyBuf;
mod body_buf;

pub use self::buf_mut::{Alloc, BufMut};
mod buf_mut;

pub use self::send_buf::SendBuf;
mod send_buf;

pub use self::recv_buf::RecvBuf;
mod recv_buf;

/// The maximum length of an array in bytes.
pub(crate) const MAX_ARRAY_LENGTH: u32 = 1u32 << 26;
/// The maximum length of a body in bytes.
pub(crate) const MAX_BODY_LENGTH: u32 = 1u32 << 27;

use core::mem::align_of;

/// Calculate padding with the assumption that alignment is a power of two.
#[inline(always)]
pub(crate) fn padding_to<T>(len: usize) -> usize {
    let mask = align_of::<T>() - 1;
    (align_of::<T>() - (len & mask)) & mask
}

#[inline(always)]
const fn max_size_for_align(align: usize) -> usize {
    isize::MAX as usize - (align - 1)
}
