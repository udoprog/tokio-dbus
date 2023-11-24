//! Low level details for the D-Bus protocol implementation.

#[doc(inline)]
pub use tokio_dbus_core::proto::{Endianness, Flags, MessageType, Type, Variant};

use crate::{Frame, Signature};

/// A protocol header.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct Header {
    pub(crate) endianness: Endianness,
    pub(crate) message_type: MessageType,
    pub(crate) flags: Flags,
    pub(crate) version: u8,
    pub(crate) body_length: u32,
    pub(crate) serial: u32,
}

impl crate::frame::sealed::Sealed for Header {}

unsafe impl Frame for Header {
    const SIGNATURE: &'static Signature = Signature::new_const(b"yyyyuu");

    fn adjust(&mut self, endianness: Endianness) {
        self.body_length.adjust(endianness);
        self.serial.adjust(endianness);
    }
}

impl_traits_for_frame!(Header);

implement_remote!(Variant, Endianness, MessageType, Flags, Type);
