use crate::buf::BufMut;
use crate::error::Result;
use crate::{ObjectPath, Signature};

use super::ExtendBuf;

mod sealed {
    pub trait Sealed {}
}

/// Types which can be conveniently used as arguments when extending buffers.
///
/// See for example [`BodyBuf::extend`].
///
/// [`BodyBuf::extend`]: crate::BodyBuf::extend
pub trait Arguments: self::sealed::Sealed {
    /// Write `self` into `buf`.
    #[doc(hidden)]
    fn extend_to<O: ?Sized>(&self, buf: &mut O) -> Result<()>
    where
        O: ExtendBuf;

    #[doc(hidden)]
    fn buf_to<O: ?Sized>(&self, buf: &mut O)
    where
        O: BufMut;
}

macro_rules! impl_store {
    ($($ty:ty),*) => {
        $(
            impl self::sealed::Sealed for $ty {}

            impl Arguments for $ty {
                #[inline]
                fn extend_to<O: ?Sized>(&self, buf: &mut O) -> Result<()>
                where
                    O: ExtendBuf
                {
                    buf.store(*self)
                }

                #[inline]
                fn buf_to<O: ?Sized>(&self, buf: &mut O)
                where
                    O: BufMut
                {
                    buf.store(*self)
                }
            }
        )*
    }
}

macro_rules! impl_write {
    ($($ty:ty),*) => {
        $(
            impl self::sealed::Sealed for $ty {}

            impl Arguments for $ty {
                #[inline]
                fn extend_to<O: ?Sized>(&self, buf: &mut O) -> Result<()>
                where
                    O: ExtendBuf
                {
                    buf.write(self)
                }

                #[inline]
                fn buf_to<O: ?Sized>(&self, buf: &mut O)
                where
                    O: BufMut
                {
                    buf.write(self)
                }
            }
        )*
    }
}

impl_store!(u8, u16, u32, u64, i16, i32, i64, f64);
impl_write!(str, [u8], ObjectPath, Signature);

impl<T: ?Sized> self::sealed::Sealed for &T where T: Arguments {}

impl<T: ?Sized> Arguments for &T
where
    T: Arguments,
{
    #[inline]
    fn extend_to<O: ?Sized>(&self, buf: &mut O) -> Result<()>
    where
        O: ExtendBuf,
    {
        (**self).extend_to(buf)
    }

    #[inline]
    fn buf_to<O: ?Sized>(&self, buf: &mut O)
    where
        O: BufMut,
    {
        (**self).buf_to(buf)
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),*) => {
        impl<$($ty,)*> self::sealed::Sealed for ($($ty,)*) where $($ty: Arguments,)* {}

        impl<$($ty,)*> Arguments for ($($ty,)*) where $($ty: Arguments,)* {
            #[inline]
            #[allow(non_snake_case)]
            fn extend_to<_O: ?Sized>(&self, buf: &mut _O) -> Result<()>
            where
                _O: ExtendBuf
            {
                let ($($ty,)*) = self;
                $(<$ty as Arguments>::extend_to($ty, buf)?;)*
                Ok(())
            }

            #[inline]
            #[allow(non_snake_case)]
            fn buf_to<_O: ?Sized>(&self, buf: &mut _O)
            where
                _O: BufMut
            {
                let ($($ty,)*) = self;
                $(<$ty as Arguments>::buf_to($ty, buf);)*
            }
        }
    }
}

repeat!(impl_tuple);
