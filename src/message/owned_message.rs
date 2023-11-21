use std::num::NonZeroU32;

use crate::message::OwnedMessageKind;
use crate::{BodyBuf, Endianness, Flags, Message, MessageKind, OwnedSignature, ReadBuf, Signature};

/// An owned D-Bus message.
///
/// This is the owned variant of a [`Message`], to convert to a [`Message`], use
/// [`OwnedMessage::borrow`].
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Convert this message into a [`MessageKind::MessageReturn`] message with
    /// an empty body where the reply serial matches that of the current
    /// message.
    ///
    /// The `send` argument is used to populate the next serial number.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, MessageKind, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello")
    ///     .with_sender("se.tedro.DBusExample")
    ///     .with_destination("org.freedesktop.DBus")
    ///     .to_owned();
    ///
    /// let m2 = m.clone().method_return(send.next_serial());
    /// assert!(matches!(m2.kind(), MessageKind::MethodReturn { .. }));
    ///
    /// assert_eq!(m.sender(), m2.destination());
    /// assert_eq!(m.destination(), m2.sender());
    /// ```
    pub fn method_return(self, serial: NonZeroU32) -> Self {
        Self {
            kind: OwnedMessageKind::MethodReturn {
                reply_serial: self.serial,
            },
            serial,
            flags: Flags::EMPTY,
            signature: OwnedSignature::EMPTY,
            interface: None,
            destination: self.sender,
            sender: self.destination,
            body: Box::from([]),
            endianness: self.endianness,
        }
    }

    /// Construct a signal [`OwnedMessage`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{OwnedMessage, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.signal("Hello").to_owned();
    /// let m2 = OwnedMessage::signal("Hello".into(), m.serial());
    /// assert_eq!(m, m2);
    /// ```
    pub fn signal(member: Box<str>, serial: NonZeroU32) -> Self {
        Self {
            kind: OwnedMessageKind::Signal { member },
            serial,
            flags: Flags::EMPTY,
            interface: None,
            destination: None,
            sender: None,
            signature: OwnedSignature::empty(),
            body: Box::from([]),
            endianness: Endianness::NATIVE,
        }
    }

    /// Convert this message into a [`MessageKind::Error`] message with
    /// an empty body where the reply serial matches that of the current
    /// message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, MessageKind, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello")
    ///     .with_sender("se.tedro.DBusExample")
    ///     .with_destination("org.freedesktop.DBus");
    ///
    /// let m2 = m.clone().error("org.freedesktop.DBus.UnknownMethod", send.next_serial());
    /// assert!(matches!(m2.kind(), MessageKind::Error { .. }));
    ///
    /// assert_eq!(m.sender(), m2.destination());
    /// assert_eq!(m.destination(), m2.sender());
    /// ```
    pub fn error(self, error_name: Box<str>, serial: NonZeroU32) -> Self {
        Self {
            kind: OwnedMessageKind::Error {
                error_name,
                reply_serial: self.serial,
            },
            serial,
            flags: Flags::EMPTY,
            signature: OwnedSignature::empty(),
            interface: None,
            destination: self.sender,
            sender: self.destination,
            body: Box::from([]),
            endianness: self.endianness,
        }
    }

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

    /// Get the kind of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, MessageKind, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello");
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    ///
    /// let m2 = m.error("org.freedesktop.DBus.UnknownMethod", send.next_serial());
    /// assert!(matches!(m2.kind(), MessageKind::Error { .. }));
    /// ```
    pub fn kind(&self) -> MessageKind<'_> {
        self.kind.borrow()
    }

    /// Modify the body and signature of the message to match that of the
    /// provided body buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{BodyBuf, Message, MessageKind, SendBuf, Signature};
    ///
    /// let mut send = SendBuf::new();
    /// let mut body = BodyBuf::new();
    ///
    /// body.write("Hello World!");
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello")
    ///     .to_owned()
    ///     .with_body_buf(&body);
    ///
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    /// assert_eq!(m.signature(), Signature::STRING);
    /// ```
    pub fn with_body_buf(self, body: &BodyBuf) -> Self {
        self.with_signature(body.signature().to_owned())
            .with_body(body.read().get().into())
    }

    /// Get a buffer to the body of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{BodyBuf, Message, MessageKind, SendBuf, Signature};
    ///
    /// let mut send = SendBuf::new();
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(42u32);
    /// body.write("Hello World!");
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello")
    ///     .to_owned()
    ///     .with_body_buf(&body);
    ///
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    /// assert_eq!(m.signature(), Signature::new(b"us")?);
    ///
    /// let mut r = m.body();
    /// assert_eq!(r.load::<u32>()?, 42);
    /// assert_eq!(r.read::<str>()?, "Hello World!");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn body(&self) -> ReadBuf<'_> {
        ReadBuf::from_slice(&self.body, self.endianness)
    }

    /// Modify the body of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{BodyBuf, Message, MessageKind, SendBuf, Signature};
    ///
    /// let mut send = SendBuf::new();
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(42u32);
    /// body.write("Hello World!");
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello")
    ///     .to_owned()
    ///     .with_body(body.read().get().into())
    ///     .with_signature(body.signature().to_owned());
    ///
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    /// assert_eq!(m.signature(), Signature::new(b"us")?);
    ///
    /// let mut r = m.body();
    /// assert_eq!(r.load::<u32>()?, 42);
    /// assert_eq!(r.read::<str>()?, "Hello World!");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    pub fn with_body(self, body: Box<[u8]>) -> Self {
        Self { body, ..self }
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
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello");
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
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.serial().get(), 1);
    ///
    /// let m2 = m.with_serial(NonZeroU32::new(1000).unwrap());
    /// assert_eq!(m2.serial().get(), 1000);
    /// ```
    pub fn with_serial(self, serial: NonZeroU32) -> Self {
        Self { serial, ..self }
    }

    /// Get the flags of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Flags, Message, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello");
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
    /// use tokio_dbus::{Flags, Message, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello");
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
    /// use tokio_dbus::{Message, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello").to_owned();
    /// assert_eq!(m.interface(), None);
    ///
    /// let m2 = m.with_interface("org.freedesktop.DBus".into());
    /// assert_eq!(m2.interface(), Some("org.freedesktop.DBus"));
    /// ```
    pub fn interface(&self) -> Option<&str> {
        self.interface.as_deref()
    }

    /// Modify the interface of the message.
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
    /// assert_eq!(m.interface(), None);
    ///
    /// let m2 = m.with_interface("org.freedesktop.DBus".into());
    /// assert_eq!(m2.interface(), Some("org.freedesktop.DBus"));
    /// ```
    pub fn with_interface(self, interface: Box<str>) -> Self {
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
    /// use tokio_dbus::{Message, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello").to_owned();
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_destination(":1.131".into());
    /// assert_eq!(m2.destination(), Some(":1.131"));
    /// ```
    pub fn destination(&self) -> Option<&str> {
        self.destination.as_deref()
    }

    /// Modify the destination of the message.
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
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_destination(":1.131".into());
    /// assert_eq!(m2.destination(), Some(":1.131"));
    /// ```
    pub fn with_destination(self, destination: Box<str>) -> Self {
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
    /// use tokio_dbus::{Message, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello").to_owned();
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_sender(":1.131".into());
    /// assert_eq!(m2.sender(), Some(":1.131"));
    /// ```
    pub fn sender(&self) -> Option<&str> {
        self.sender.as_deref()
    }

    /// Modify the sender of the message.
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
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_sender(":1.131".into());
    /// assert_eq!(m2.sender(), Some(":1.131"));
    /// ```
    pub fn with_sender(self, sender: Box<str>) -> Self {
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
    /// use tokio_dbus::{Message, SendBuf, Signature};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello").to_owned();
    /// assert_eq!(m.signature(), Signature::EMPTY);
    ///
    /// let m2 = m.with_signature(Signature::STRING.to_owned());
    /// assert_eq!(m2.signature(), Signature::STRING);
    /// ```
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Modify the signature of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, Signature, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call("/org/freedesktop/DBus", "Hello");
    /// assert_eq!(m.signature(), Signature::EMPTY);
    ///
    /// let m2 = m.with_signature(Signature::STRING);
    /// assert_eq!(m2.signature(), Signature::STRING);
    /// ```
    pub fn with_signature(self, signature: OwnedSignature) -> Self {
        Self { signature, ..self }
    }
}

impl PartialEq<Message<'_>> for OwnedMessage {
    #[inline]
    fn eq(&self, other: &Message<'_>) -> bool {
        *other == *self
    }
}
