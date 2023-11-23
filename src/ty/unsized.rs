use crate::ty::Marker;

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A marker that is unsized.
pub trait Unsized: self::sealed::Sealed + Marker {
    /// The unsized target.
    #[doc(hidden)]
    type Target: ?Sized;
}
