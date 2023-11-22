use crate::signature::{SignatureBuilder, SignatureError};

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
pub trait Fields: self::sealed::Sealed + Aligned {
    /// The target field.
    #[doc(hidden)]
    type First;

    /// The next struct fields to write.
    #[doc(hidden)]
    type Remaining;

    /// Write signature.
    #[doc(hidden)]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError>;
}

impl super::aligned::sealed::Sealed for () {}

impl Aligned for () {
    type Type = u64;
}

impl Fields for () {
    type First = Empty;
    type Remaining = ();

    #[inline]
    fn write_signature(_: &mut SignatureBuilder) -> Result<(), SignatureError> {
        Ok(())
    }
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

        impl<$first, $($rest),*> Fields for ($first, $($rest),*)
        where
            $first: Marker,
            $($rest: Marker,)*
        {
            type First = A;
            type Remaining = ($($rest,)*);

            #[inline]
            fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
                signature.open_struct()?;
                <$first>::write_signature(signature)?;
                $(<$rest>::write_signature(signature)?;)*
                signature.close_struct()?;
                Ok(())
            }
        }
    }
}

repeat!(struct_fields);
