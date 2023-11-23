use crate::error::Result;
use crate::BodyBuf;

pub(crate) mod sealed {
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
    fn extend_to(&self, buf: &mut BodyBuf) -> Result<()>;

    #[doc(hidden)]
    fn buf_to(&self, buf: &mut BodyBuf);
}

impl<T: ?Sized> self::sealed::Sealed for &T where T: Arguments {}

impl<T: ?Sized> Arguments for &T
where
    T: Arguments,
{
    #[inline]
    fn extend_to(&self, buf: &mut BodyBuf) -> Result<()> {
        (**self).extend_to(buf)
    }

    #[inline]
    fn buf_to(&self, buf: &mut BodyBuf) {
        (**self).buf_to(buf);
    }
}

macro_rules! impl_tuple {
    ($($ty:ident),*) => {
        impl<$($ty,)*> self::sealed::Sealed for ($($ty,)*) where $($ty: Arguments,)* {}

        impl<$($ty,)*> Arguments for ($($ty,)*) where $($ty: Arguments,)* {
            #[inline]
            #[allow(non_snake_case)]
            fn extend_to(&self, buf: &mut BodyBuf) -> Result<()> {
                let ($($ty,)*) = self;
                $(<$ty as Arguments>::extend_to($ty, buf)?;)*
                Ok(())
            }

            #[inline]
            #[allow(non_snake_case)]
            fn buf_to(&self, buf: &mut BodyBuf) {
                let ($($ty,)*) = self;
                $(<$ty as Arguments>::buf_to($ty, buf);)*
            }
        }
    }
}

repeat!(impl_tuple);
