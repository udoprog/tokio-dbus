use crate::{ObjectPath, Signature};

use super::{Marker, Sig, Str, O};

mod sealed {
    use super::{Sig, Str, O};

    pub trait Sealed {}

    impl Sealed for Str {}
    impl Sealed for Sig {}
    impl Sealed for O {}
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

impl Unsized for O {
    type Target = ObjectPath;
}
