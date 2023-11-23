use crate::buf::ArrayReader;
use crate::frame::Frame;
use crate::signature::{Signature, SignatureBuilder, SignatureError, SignatureErrorKind};
use crate::{Body, ObjectPath, Result};

use super::{Aligned, Array, Sig, Str, O};

mod sealed {
    use super::{Array, Marker, Sig, Str, O};
    use crate::frame::Frame;
    pub trait Sealed {}
    impl<T> Sealed for T where T: Frame {}
    impl Sealed for Str {}
    impl Sealed for Sig {}
    impl Sealed for O {}
    impl<T> Sealed for Array<T> where T: Marker {}
}

/// The trait implementation for a type marker.
pub trait Marker: self::sealed::Sealed + Aligned {
    /// Return type used for the marker.
    #[doc(hidden)]
    type Return<'de>;

    /// Read the value from a structure.
    #[doc(hidden)]
    fn read_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>>;

    /// Writing the signature.
    #[doc(hidden)]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError>;
}

impl<T> Aligned for T
where
    T: Frame,
{
    type Type = T;
}

impl<T> Marker for T
where
    T: Frame,
{
    type Return<'de> = T;

    #[inline]
    fn read_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        buf.load()
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        if !signature.extend_from_signature(T::SIGNATURE) {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        Ok(())
    }
}

impl Aligned for Str {
    type Type = u32;
}

impl Marker for Str {
    type Return<'de> = &'de str;

    #[inline]
    fn read_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        buf.read()
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        if !signature.extend_from_signature(Signature::STRING) {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        Ok(())
    }
}

impl Aligned for Sig {
    type Type = u8;
}

impl Marker for Sig {
    type Return<'de> = &'de Signature;

    #[inline]
    fn read_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        buf.read()
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        if !signature.extend_from_signature(Signature::SIGNATURE) {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        Ok(())
    }
}

impl Aligned for O {
    type Type = u8;
}

impl Marker for O {
    type Return<'de> = &'de ObjectPath;

    #[inline]
    fn read_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        buf.read()
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        if !signature.extend_from_signature(Signature::OBJECT_PATH) {
            return Err(SignatureError::new(SignatureErrorKind::SignatureTooLong));
        }

        Ok(())
    }
}

impl<T> Aligned for Array<T>
where
    T: Aligned,
{
    type Type = T;
}

impl<T> Marker for Array<T>
where
    T: Marker,
{
    type Return<'de> = ArrayReader<'de, T>;

    #[inline]
    fn read_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>> {
        buf.read_array::<T>()
    }

    #[inline]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError> {
        signature.open_array()?;
        T::write_signature(signature)?;
        signature.close_array();
        Ok(())
    }
}
