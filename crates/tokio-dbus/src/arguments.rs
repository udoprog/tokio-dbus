use crate::WriteAligned;
use crate::error::Result;

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Types which can be conveniently used as arguments when extending buffers.
///
/// See for example [`BodyBuf::arguments`].
///
/// [`WriteAligned::arguments`]: crate::WriteAligned::arguments
pub trait Arguments: self::sealed::Sealed {
    /// Write `self` into `buf`.
    #[doc(hidden)]
    fn extend_to<B>(&self, buf: &mut B) -> Result<()>
    where
        B: ?Sized + WriteAligned;

    #[doc(hidden)]
    fn buf_to<B>(&self, buf: &mut B)
    where
        B: ?Sized + WriteAligned;
}

impl<T> self::sealed::Sealed for &T where T: ?Sized + Arguments {}

impl<T> Arguments for &T
where
    T: ?Sized + Arguments,
{
    #[inline]
    fn extend_to<B>(&self, buf: &mut B) -> Result<()>
    where
        B: ?Sized + WriteAligned,
    {
        (**self).extend_to(buf)
    }

    #[inline]
    fn buf_to<B>(&self, buf: &mut B)
    where
        B: ?Sized + WriteAligned,
    {
        (**self).buf_to(buf);
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),*) => {
        impl<$($ty,)*> self::sealed::Sealed for ($($ty,)*) where $($ty: Arguments,)* {}

        impl<$($ty,)*> Arguments for ($($ty,)*) where $($ty: Arguments,)* {
            #[inline]
            #[allow(non_snake_case)]
            fn extend_to<B>(&self, buf: &mut B) -> $crate::error::Result<()>
            where
                B: ?Sized + $crate::WriteAligned,
            {
                let ($($ty,)*) = self;
                $(<$ty as Arguments>::extend_to($ty, buf)?;)*
                Ok(())
            }

            #[inline]
            #[allow(non_snake_case)]
            fn buf_to<B>(&self, buf: &mut B)
            where
                B: ?Sized + $crate::WriteAligned,
            {
                let ($($ty,)*) = self;
                $(<$ty as Arguments>::buf_to($ty, buf);)*
            }
        }
    }
}

repeat!(impl_tuple);
