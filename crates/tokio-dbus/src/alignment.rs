use crate::{Frame, ty};

/// The alignment of a D-Bus type, as a runtime value.
///
/// D-Bus values are padded to the alignment of their type, which for the static
/// writers in this crate is derived from a [`ty::Marker`]. Code which works with
/// types that are only known at runtime needs to pass it explicitly instead.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{ty, Alignment};
///
/// assert_eq!(Alignment::of::<u8>(), Alignment::BYTE);
/// assert_eq!(Alignment::of::<ty::Str>(), Alignment::U32);
/// assert_eq!(Alignment::of::<(u8, u8)>(), Alignment::U64);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Alignment {
    /// Alignment of one byte, used by `y`, `g` and `v`.
    BYTE,
    /// Alignment of two bytes, used by `n` and `q`.
    U16,
    /// Alignment of four bytes, used by `b`, `i`, `u`, `h`, `s`, `o` and by the
    /// length prefix of an array.
    U32,
    /// Alignment of eight bytes, used by `x`, `t`, `d`, structs and dict
    /// entries.
    U64,
}

impl Alignment {
    /// The alignment of the type described by the [`ty::Marker`] `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{ty, Alignment};
    ///
    /// assert_eq!(Alignment::of::<u64>(), Alignment::U64);
    /// assert_eq!(Alignment::of::<ty::Variant>(), Alignment::BYTE);
    /// ```
    pub const fn of<T>() -> Self
    where
        T: ty::Aligned,
    {
        <T::Alignment as Frame>::ALIGNMENT
    }

    /// The alignment as a number of bytes, which is always a power of two.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::Alignment;
    ///
    /// assert_eq!(Alignment::BYTE.in_bytes(), 1);
    /// assert_eq!(Alignment::U64.in_bytes(), 8);
    /// ```
    pub const fn in_bytes(self) -> usize {
        match self {
            Alignment::BYTE => 1,
            Alignment::U16 => 2,
            Alignment::U32 => 4,
            Alignment::U64 => 8,
        }
    }
}
