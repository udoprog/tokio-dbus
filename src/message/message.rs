use std::num::NonZeroU32;

use crate::protocol::{Flags, MessageType};
use crate::{BodyBuf, MessageKind, OwnedMessage, ReadBuf, Signature};

/// A D-Bus message.
///
/// This is the borrowed variant, to convert to an [`OwnedMessage`], use
/// [`Message::to_owned`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<'a> {
    /// The type of the message.
    pub(crate) kind: MessageKind<'a>,
    /// Serial of the emssage.
    pub(crate) serial: Option<NonZeroU32>,
    /// Flags in the message.
    pub(crate) flags: Flags,
    /// The interface of the message.
    pub(crate) interface: Option<&'a str>,
    /// The destination of the message.
    pub(crate) destination: Option<&'a str>,
    /// The sender of the message.
    pub(crate) sender: Option<&'a str>,
    /// The signature of the body.
    pub(crate) signature: &'a Signature,
    /// The body associated with the message.
    pub(crate) body: ReadBuf<'a>,
}

impl<'a> Message<'a> {
    /// Convert into an owned [`OwnedMessage`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{OwnedMessage, Message};
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello").to_owned();
    /// let m2 = OwnedMessage::method_call("/org/freedesktop/DBus".into(), "Hello".into());
    /// assert_eq!(m, m2);
    /// ```
    #[inline]
    pub fn to_owned(&self) -> OwnedMessage {
        OwnedMessage {
            kind: self.kind.to_owned(),
            serial: self.serial,
            flags: self.flags,
            interface: self.interface.map(Box::from),
            destination: self.destination.map(Box::from),
            sender: self.sender.map(Box::from),
            signature: self.signature.to_owned(),
            body: self.body.get().into(),
            endianness: self.body.endianness(),
        }
    }

    /// Construct a method call.
    pub fn method_call(path: &'a str, member: &'a str) -> Self {
        Self {
            kind: MessageKind::MethodCall { path, member },
            serial: None,
            flags: Flags::EMPTY,
            interface: None,
            destination: None,
            sender: None,
            signature: Signature::empty(),
            body: ReadBuf::empty(),
        }
    }

    /// Get the kind of the message.
    pub fn kind(&self) -> MessageKind<'a> {
        self.kind
    }

    /// Modify the body and signature of the message to match that of the
    /// provided body buffer.
    pub fn with_body_buf(self, body: &'a BodyBuf) -> Self {
        self.with_signature(body.signature()).with_body(body.read())
    }

    /// Get a buffer to the body of the message.
    pub fn body(&self) -> ReadBuf<'a> {
        self.body.clone()
    }

    /// Modify the body of the message.
    pub fn with_body(self, body: ReadBuf<'a>) -> Self {
        Self { body, ..self }
    }

    /// Get the serial of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::Message;
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.serial(), None);
    ///
    /// let m2 = m.with_serial(NonZeroU32::new(1).unwrap());
    /// assert_eq!(m2.serial(), NonZeroU32::new(1));
    /// ```
    pub fn serial(&self) -> Option<NonZeroU32> {
        self.serial
    }

    /// Modify the serial of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::Message;
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.serial(), None);
    ///
    /// let m2 = m.with_serial(NonZeroU32::new(1).unwrap());
    /// assert_eq!(m2.serial(), NonZeroU32::new(1));
    /// ```
    pub fn with_serial(self, serial: NonZeroU32) -> Self {
        Self {
            serial: Some(serial),
            ..self
        }
    }

    /// Get the flags of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, Flags};
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.flags(), Flags::default());
    ///
    /// let m2 = m.with_flags(Flags::NO_REPLY_EXPECTED);
    /// assert_eq!(m2.flags(), Flags::NO_REPLY_EXPECTED);
    /// ```
    pub fn flags(&self) -> Flags {
        self.flags
    }

    /// Modify the flags of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, Flags};
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.flags(), Flags::default());
    ///
    /// let m2 = m.with_flags(Flags::NO_REPLY_EXPECTED);
    /// assert_eq!(m2.flags(), Flags::NO_REPLY_EXPECTED);
    /// ```
    pub fn with_flags(self, flags: Flags) -> Self {
        Self { flags, ..self }
    }

    /// Get the interface of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::Message;
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.interface(), None);
    ///
    /// let m2 = m.with_interface("org.freedesktop.DBus");
    /// assert_eq!(m2.interface(), Some("org.freedesktop.DBus"));
    /// ```
    pub fn interface(&self) -> Option<&'a str> {
        self.interface
    }

    /// Modify the interface of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::Message;
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.interface(), None);
    ///
    /// let m2 = m.with_interface("org.freedesktop.DBus");
    /// assert_eq!(m2.interface(), Some("org.freedesktop.DBus"));
    /// ```
    pub fn with_interface(self, interface: &'a str) -> Self {
        Self {
            interface: Some(interface),
            ..self
        }
    }

    /// Get the destination of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::Message;
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_destination(":1.131");
    /// assert_eq!(m2.destination(), Some(":1.131"));
    /// ```
    pub fn destination(&self) -> Option<&'a str> {
        self.destination
    }

    /// Modify the destination of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::Message;
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_destination(":1.131");
    /// assert_eq!(m2.destination(), Some(":1.131"));
    /// ```
    pub fn with_destination(self, destination: &'a str) -> Self {
        Self {
            destination: Some(destination),
            ..self
        }
    }

    /// Get the sender of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::Message;
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_sender(":1.131");
    /// assert_eq!(m2.sender(), Some(":1.131"));
    /// ```
    pub fn sender(&self) -> Option<&'a str> {
        self.sender
    }

    /// Modify the sender of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::Message;
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_sender(":1.131");
    /// assert_eq!(m2.sender(), Some(":1.131"));
    /// ```
    pub fn with_sender(self, sender: &'a str) -> Self {
        Self {
            sender: Some(sender),
            ..self
        }
    }

    /// Get the signature of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, Signature};
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.signature(), Signature::EMPTY);
    ///
    /// let m2 = m.with_signature(Signature::STRING);
    /// assert_eq!(m2.signature(), Signature::STRING);
    /// ```
    pub fn signature(&self) -> &Signature {
        self.signature
    }

    /// Modify the signature of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, Signature};
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.signature(), Signature::EMPTY);
    ///
    /// let m2 = m.with_signature(Signature::STRING);
    /// assert_eq!(m2.signature(), Signature::STRING);
    /// ```
    pub fn with_signature(self, signature: &'a Signature) -> Self {
        Self { signature, ..self }
    }

    pub(crate) fn message_type(&self) -> crate::protocol::MessageType {
        match self.kind {
            MessageKind::MethodCall { .. } => MessageType::METHOD_CALL,
            MessageKind::MethodReturn { .. } => MessageType::METHOD_RETURN,
            MessageKind::Error { .. } => MessageType::ERROR,
            MessageKind::Signal { .. } => MessageType::SIGNAL,
        }
    }
}

impl PartialEq<OwnedMessage> for Message<'_> {
    #[inline]
    fn eq(&self, other: &OwnedMessage) -> bool {
        self.kind == other.kind
            && self.serial == other.serial
            && self.flags == other.flags
            && self.interface == other.interface.as_deref()
            && self.destination == other.destination.as_deref()
            && self.sender == other.sender.as_deref()
            && self.signature == other.signature
            && self.body.get() == &*other.body
            && self.body.endianness() == other.endianness
    }
}
