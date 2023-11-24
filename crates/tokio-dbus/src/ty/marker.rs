use crate::signature::{SignatureBuilder, SignatureError};
use crate::ty::Aligned;
use crate::{Body, Result};

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// The trait implementation for a type marker.
pub trait Marker: self::sealed::Sealed + Aligned {
    /// Return type used for the marker.
    #[doc(hidden)]
    type Return<'de>;

    /// Read the value from a structure.
    #[doc(hidden)]
    fn load_struct<'de>(buf: &mut Body<'de>) -> Result<Self::Return<'de>>;

    /// Writing the signature.
    #[doc(hidden)]
    fn write_signature(signature: &mut SignatureBuilder) -> Result<(), SignatureError>;
}
