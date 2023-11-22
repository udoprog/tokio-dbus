use crate::error::Result;
use crate::frame::Frame;

/// A read-only buffer.
pub trait Buf<'de> {
    /// The mutable reborrow.
    type Reborrow<'this>: Buf<'de, ReadUntil = Self::ReadUntil>
    where
        Self: 'this;

    /// Type returned by `read_until`.
    type ReadUntil: Buf<'de>;

    /// Reborrow the buffer.
    fn reborrow(&mut self) -> Self::Reborrow<'_>;

    /// Return a reader until the given length.
    fn read_until(&mut self, len: usize) -> Self::ReadUntil;

    /// The length of the buffer.
    fn len(&self) -> usize;

    /// Test if the buffer is empty.
    fn is_empty(&self) -> bool;

    /// Align the read buffer by `T`.
    fn align<T>(&mut self) -> Result<()>;

    /// Load a type `T` out of the buffer.
    fn load<T>(&mut self) -> Result<T>
    where
        T: Frame;

    /// Load a slice.
    fn load_slice(&mut self, len: usize) -> Result<&'de [u8]>;

    /// Load a nul-terminated slice.
    fn load_slice_nul(&mut self, len: usize) -> Result<&'de [u8]>;
}

impl<'de, B> Buf<'de> for &mut B
where
    B: ?Sized + Buf<'de>,
{
    type Reborrow<'this> = B::Reborrow<'this> where Self: 'this;
    type ReadUntil = B::ReadUntil;

    #[inline]
    fn reborrow(&mut self) -> Self::Reborrow<'_> {
        (**self).reborrow()
    }

    #[inline]
    fn read_until(&mut self, len: usize) -> Self::ReadUntil {
        (**self).read_until(len)
    }

    #[inline]
    fn len(&self) -> usize {
        (**self).len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        (**self).is_empty()
    }

    #[inline]
    fn align<T>(&mut self) -> Result<()> {
        (**self).align::<T>()
    }

    #[inline]
    fn load<T>(&mut self) -> Result<T>
    where
        T: Frame,
    {
        (**self).load::<T>()
    }

    #[inline]
    fn load_slice(&mut self, len: usize) -> Result<&'de [u8]> {
        (**self).load_slice(len)
    }

    #[inline]
    fn load_slice_nul(&mut self, len: usize) -> Result<&'de [u8]> {
        (**self).load_slice_nul(len)
    }
}
