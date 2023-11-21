use std::num::NonZeroU32;

use crate::{Endianness, Flags, Message, OwnedMessageKind, OwnedSignature, ReadBuf};

/// A D-Bus message.
///
/// This is the owned variant, to convert to a [`Message`], use
/// [`OwnedMessage::borrow`].
#[derive(Debug, PartialEq, Eq)]
pub struct OwnedMessage {
    /// The type of the message.
    pub(super) kind: OwnedMessageKind,
    /// Serial of the emssage.
    pub(super) serial: NonZeroU32,
    /// Flags in the message.
    pub(super) flags: Flags,
    /// The interface of the message.
    pub(super) interface: Option<Box<str>>,
    /// The destination of the message.
    pub(super) destination: Option<Box<str>>,
    /// The sender of the message.
    pub(super) sender: Option<Box<str>>,
    /// The signature of the body.
    pub(super) signature: OwnedSignature,
    /// The body associated with the message.
    pub(super) body: Box<[u8]>,
    /// The endianness of the body.
    pub(super) endianness: Endianness,
}

impl OwnedMessage {
    /// Borrow into a [`Message`].
    pub fn borrow(&self) -> Message<'_> {
        Message {
            kind: self.kind.borrow(),
            serial: self.serial,
            flags: self.flags,
            interface: self.interface.as_deref(),
            destination: self.destination.as_deref(),
            sender: self.sender.as_deref(),
            signature: &self.signature,
            body: ReadBuf::from_slice(self.body.as_ref(), self.endianness),
        }
    }

    /// Construct a method call.
    pub fn method_call(path: Box<str>, member: Box<str>, serial: NonZeroU32) -> Self {
        Self {
            kind: OwnedMessageKind::MethodCall { path, member },
            serial,
            flags: Flags::EMPTY,
            interface: None,
            destination: None,
            sender: None,
            signature: OwnedSignature::EMPTY,
            body: Box::from([]),
            endianness: Endianness::NATIVE,
        }
    }
}

impl PartialEq<Message<'_>> for OwnedMessage {
    #[inline]
    fn eq(&self, other: &Message<'_>) -> bool {
        *other == *self
    }
}
