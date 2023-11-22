pub(crate) mod sealed {
    use super::super::{Array, Sig, Str, O};
    use super::Aligned;
    use crate::Frame;

    pub trait Sealed {}

    impl Sealed for Sig {}
    impl Sealed for O {}
    impl Sealed for Str {}
    impl<T> Sealed for T where T: Frame {}
    impl<T> Sealed for Array<T> where T: Aligned {}
}

/// An alignment marker.
pub trait Aligned: self::sealed::Sealed {
    /// The type this type is aligned with.
    type Type;
}
