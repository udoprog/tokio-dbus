use crate::signature::{SignatureBuilder, SignatureError};

use super::Marker;

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
pub trait Fields: self::sealed::Sealed {
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
