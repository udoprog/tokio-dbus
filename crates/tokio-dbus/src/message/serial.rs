use core::fmt;
use core::num::NonZeroU32;

/// An opaque identifier for a serial.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Serial(NonZeroU32);

impl Serial {
    #[cfg(feature = "alloc")]
    #[inline]
    pub(crate) fn new(serial: NonZeroU32) -> Self {
        Self(serial)
    }

    #[cfg(feature = "alloc")]
    #[inline]
    pub(crate) fn get(self) -> u32 {
        self.0.get()
    }
}

impl fmt::Display for Serial {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for Serial {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
