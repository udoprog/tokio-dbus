//! Types for dealing with buffers.

#[cfg(test)]
mod tests;

pub use self::read_buf::ReadBuf;
mod read_buf;

use self::array_writer::ArrayWriter;
mod array_writer;

pub use self::array_reader::ArrayReader;
mod array_reader;

pub use self::struct_writer::StructWriter;
mod struct_writer;

pub use self::owned_buf::OwnedBuf;
mod owned_buf;

pub use self::struct_reader::StructReader;
mod struct_reader;

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
