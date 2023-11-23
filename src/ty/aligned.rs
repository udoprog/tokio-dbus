pub(crate) mod sealed {
    pub trait Sealed {}
}

/// An alignment marker.
pub trait Aligned: self::sealed::Sealed {
    /// The type this type is aligned with.
    #[doc(hidden)]
    type Alignment;
}
