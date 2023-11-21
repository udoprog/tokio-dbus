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
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, OwnedMessage, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello", send.next_serial()).to_owned();
    /// let m2 = OwnedMessage::method_call("/org/freedesktop/DBus".into(), "Hello".into(), m.serial());
    /// assert_eq!(m, m2);
    /// ```
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

    /// Get the serial of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello").to_owned();
    /// assert_eq!(m.serial().get(), 1);
    ///
    /// let m2 = m.with_serial(NonZeroU32::new(1000).unwrap());
    /// assert_eq!(m2.serial().get(), 1000);
    /// ```
    pub fn serial(&self) -> NonZeroU32 {
        self.serial
    }

    /// Modify the serial of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello").to_owned();
    /// assert_eq!(m.serial().get(), 1);
    ///
    /// let m2 = m.with_serial(NonZeroU32::new(1000).unwrap());
    /// assert_eq!(m2.serial().get(), 1000);
    /// ```
    pub fn with_serial(self, serial: NonZeroU32) -> Self {
        Self { serial, ..self }
    }
}

impl PartialEq<Message<'_>> for OwnedMessage {
    #[inline]
    fn eq(&self, other: &Message<'_>) -> bool {
        *other == *self
    }
}
