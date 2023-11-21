use crate::Signature;

use super::{Marker, Sig, Str};

mod sealed {
    use super::{Sig, Str};

    pub trait Sealed {}

    impl Sealed for Str {}
    impl Sealed for Sig {}
}

/// A marker that is unsized.
pub trait Unsized: self::sealed::Sealed + Marker {
    /// The unsized target.
    #[doc(hidden)]
    type Target: ?Sized;
}

impl Unsized for Str {
    type Target = str;
}

impl Unsized for Sig {
    type Target = Signature;
}
