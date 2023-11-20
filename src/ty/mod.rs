use std::marker::PhantomData;

use crate::{Frame, OwnedSignature, Signature};

/// A marker that is unsized.
pub trait Unsized {
    /// The unsized target.
    type Target: ?Sized;
}

/// The trait implementation for a type marker.
pub trait Marker {
    /// Writing the signature.
    fn write_signature(signature: &mut OwnedSignature);
}

impl<T> Marker for T
where
    T: Frame,
{
    #[inline]
    fn write_signature(signature: &mut OwnedSignature) {
        signature.extend_from_signature(T::SIGNATURE);
    }
}

/// The [`Marker`] for the empty type.
#[non_exhaustive]
pub enum Empty {}

/// The [`Marker`] for the [`str`] type.
///
/// [`Signature`]: crate::Signature
#[non_exhaustive]
pub struct Str;

impl Unsized for Str {
    type Target = str;
}

impl Marker for Str {
    #[inline]
    fn write_signature(signature: &mut OwnedSignature) {
        signature.extend_from_signature(Signature::STRING);
    }
}

/// The [`Marker`] for the [`Signature`] type.
///
/// [`Signature`]: crate::Signature
#[non_exhaustive]
pub struct Sig;

impl Unsized for Sig {
    type Target = Signature;
}

impl Marker for Sig {
    #[inline]
    fn write_signature(signature: &mut OwnedSignature) {
        signature.extend_from_signature(Signature::SIGNATURE);
    }
}

/// Type marker for the fields in a struct.
///
/// This is implemented by tuples.
pub trait Fields {
    /// The target field.
    type First;

    /// The next struct fields to write.
    type Remaining;

    /// Write signature.
    fn write_signature(signature: &mut OwnedSignature);
}

/// An array marker type.
pub struct Array<T>(PhantomData<T>);

impl<T> Marker for Array<T>
where
    T: Marker,
{
    fn write_signature(signature: &mut OwnedSignature) {
        signature.push(b'a');
        T::write_signature(signature);
    }
}

impl Fields for () {
    type First = Empty;
    type Remaining = ();

    #[inline]
    fn write_signature(_: &mut OwnedSignature) {}
}

macro_rules! struct_fields {
    ($first:ident $(, $rest:ident)*) => {
        impl<$first, $($rest),*> Fields for ($first, $($rest),*)
        where
            $first: Marker,
            $($rest: Marker,)*
        {
            type First = A;
            type Remaining = ($($rest,)*);

            #[inline]
            fn write_signature(signature: &mut OwnedSignature) {
                signature.push(b'(');
                <$first>::write_signature(signature);
                $(<$rest>::write_signature(signature);)*
                signature.push(b')');
            }
        }
    }
}

macro_rules! repeat {
    ($macro:path) => {
        $macro!(A);
        $macro!(A, B);
        $macro!(A, B, C);
        $macro!(A, B, C, D);
        $macro!(A, B, C, D, E);
        $macro!(A, B, C, D, E, F);
        $macro!(A, B, C, D, E, F, G);
        $macro!(A, B, C, D, E, F, G, H);
    };
}

repeat!(struct_fields);
