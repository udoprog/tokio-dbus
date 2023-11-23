use crate::signature::{SignatureBuilder, SignatureError};
use crate::{Body, Result};

use super::{Aligned, Marker};

mod sealed {
    pub trait Sealed {}
    impl Sealed for () {}
}

/// The [`Marker`] for the empty type.
#[non_exhaustive]
pub enum Empty {}

/// Trait indicating the fields of a struct.
///
/// This is implemented by tuples.
pub trait Fields: self::sealed::Sealed + Marker {
    /// The target field.
    #[doc(hidden)]
    type First;

    /// The next struct fields to write.
    #[doc(hidden)]
    type Remaining;
}

impl super::aligned::sealed::Sealed for () {}

impl Aligned for () {
    type Type = u64;
}

impl super::marker::sealed::Sealed for () {}

impl Marker for () {
    type Return<'de> = ();

    #[inline]
    fn read_struct<'de>(_: &mut Body<'de>) -> Result<Self::Return<'de>> {
        Ok(())
    }

    #[inline]
    fn write_signature(_: &mut SignatureBuilder) -> Result<(), SignatureError> {
        Ok(())
    }
}

impl Fields for () {
    type First = Empty;
    type Remaining = ();
}

macro_rules! struct_fields {
    ($first:ident $(, $rest:ident)*) => {
        impl<$first, $($rest),*> self::sealed::Sealed for ($first, $($rest),*)
        where
            $first: Marker,
            $($rest: Marker,)*
        {
        }

        impl<$first, $($rest),*> super::aligned::sealed::Sealed for ($first, $($rest),*) {
        }

        impl<$first, $($rest),*> Aligned for ($first, $($rest),*)
        where
            $first: Marker,
            $($rest: Marker,)*
        {
            type Type = u64;
        }

        impl<$first, $($rest),*> super::marker::sealed::Sealed for ($first, $($rest),*)
        where
            $first: Marker,
            $($rest: Marker,)* {
        }

        impl<$first, $($rest),*> Marker for ($first, $($rest),*)
        where
            $first: Marker,
            $($rest: Marker,)*
        {
            type Return<'de> = ($first::Return<'de>, $($rest::Return<'de>,)*);

            #[inline]
            fn read_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
                Ok((<$first>::read_struct(buf)?, $(<$rest>::read_struct(buf)? ,)*))
            }

            #[inline]
            fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
                signature.open_struct()?;
                <$first>::write_signature(signature)?;
                $(<$rest>::write_signature(signature)?;)*
                signature.close_struct()?;
                Ok(())
            }
        }

        impl<$first, $($rest),*> Fields for ($first, $($rest),*)
        where
            $first: Marker,
            $($rest: Marker,)*
        {
            type First = A;
            type Remaining = ($($rest,)*);
        }
    }
}

repeat!(struct_fields);
