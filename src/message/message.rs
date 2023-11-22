use std::num::NonZeroU32;

use crate::proto::{Flags, MessageType};
use crate::{BodyBuf, BodyReadBuf, MessageKind, ObjectPath, OwnedMessage, Signature};

/// A borrowed D-Bus message.
///
/// This is the borrowed variant of [`OwnedMessage`], to convert to an
/// [`OwnedMessage`], use [`Message::to_owned`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message<'a> {
    /// The type of the message.
    pub(crate) kind: MessageKind<'a>,
    /// Serial of the emssage.
    pub(crate) serial: NonZeroU32,
    /// Flags in the message.
    pub(crate) flags: Flags,
    /// The interface of the message.
    pub(crate) interface: Option<&'a str>,
    /// The destination of the message.
    pub(crate) destination: Option<&'a str>,
    /// The sender of the message.
    pub(crate) sender: Option<&'a str>,
    /// The body associated with the message.
    pub(crate) body: BodyReadBuf<'a>,
}

impl<'a> Message<'a> {
    /// Construct a method call [`Message`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// let m2 = Message::method_call(PATH, "Hello", m.serial());
    /// assert_eq!(m, m2);
    /// ```
    pub fn method_call(path: &'a ObjectPath, member: &'a str, serial: NonZeroU32) -> Self {
        Self {
            kind: MessageKind::MethodCall { path, member },
            serial,
            flags: Flags::EMPTY,
            interface: None,
            destination: None,
            sender: None,
            body: BodyReadBuf::empty(),
        }
    }

    /// Convert this message into a [`MessageKind::MethodReturn`] message with
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
    /// use tokio_dbus::{Message, MessageKind, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello")
    ///     .with_sender("se.tedro.DBusExample")
    ///     .with_destination("org.freedesktop.DBus");
    ///
    /// let m2 = m.method_return(send.next_serial());
    /// assert!(matches!(m2.kind(), MessageKind::MethodReturn { .. }));
    ///
    /// assert_eq!(m.sender(), m2.destination());
    /// assert_eq!(m.destination(), m2.sender());
    /// ```
    pub fn method_return(&self, serial: NonZeroU32) -> Self {
        Self {
            kind: MessageKind::MethodReturn {
                reply_serial: self.serial,
            },
            serial,
            flags: Flags::EMPTY,
            interface: None,
            destination: self.sender,
            sender: self.destination,
            body: BodyReadBuf::empty(),
        }
    }

    /// Construct a signal [`Message`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.signal("Hello");
    /// let m2 = Message::signal("Hello", m.serial());
    /// assert_eq!(m, m2);
    /// ```
    #[must_use]
    pub fn signal(member: &'a str, serial: NonZeroU32) -> Self {
        Self {
            kind: MessageKind::Signal { member },
            serial,
            flags: Flags::EMPTY,
            interface: None,
            destination: None,
            sender: None,
            body: BodyReadBuf::empty(),
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
    /// use tokio_dbus::{Message, MessageKind, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello")
    ///     .with_sender("se.tedro.DBusExample")
    ///     .with_destination("org.freedesktop.DBus");
    ///
    /// let m2 = m.error("org.freedesktop.DBus.UnknownMethod", send.next_serial());
    /// assert!(matches!(m2.kind(), MessageKind::Error { .. }));
    ///
    /// assert_eq!(m.sender(), m2.destination());
    /// assert_eq!(m.destination(), m2.sender());
    /// ```
    #[must_use]
    pub fn error(&self, error_name: &'a str, serial: NonZeroU32) -> Self {
        Self {
            kind: MessageKind::Error {
                error_name,
                reply_serial: self.serial,
            },
            serial,
            flags: Flags::EMPTY,
            interface: None,
            destination: self.sender,
            sender: self.destination,
            body: BodyReadBuf::empty(),
        }
    }

    /// Convert into an owned [`OwnedMessage`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, OwnedMessage, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello").to_owned();
    /// let m2 = OwnedMessage::method_call(PATH.into(), "Hello".into(), m.serial());
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
            body: BodyBuf::from(self.body.clone()),
        }
    }

    /// Get the kind of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, MessageKind, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    ///
    /// let m2 = m.error("org.freedesktop.DBus.UnknownMethod", send.next_serial());
    /// assert!(matches!(m2.kind(), MessageKind::Error { .. }));
    /// ```
    #[must_use]
    pub fn kind(&self) -> MessageKind<'a> {
        self.kind
    }

    /// Modify the body and signature of the message to match that of the
    /// provided body buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{BodyBuf, Message, MessageKind, ObjectPath, SendBuf, Signature};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    /// let mut body = BodyBuf::new();
    ///
    /// body.write("Hello World!");
    ///
    /// let m = send.method_call(PATH, "Hello")
    ///     .with_body(&body);
    ///
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    /// assert_eq!(m.signature(), Signature::STRING);
    /// ```
    #[must_use]
    pub fn with_body(self, body: &'a BodyBuf) -> Self {
        Self {
            body: body.peek(),
            ..self
        }
    }

    /// Get a buffer to the body of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{BodyBuf, Message, MessageKind, ObjectPath, SendBuf, Signature};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    /// let mut body = BodyBuf::new();
    ///
    /// body.store(42u32);
    /// body.write("Hello World!");
    ///
    /// let m = send.method_call(PATH, "Hello")
    ///     .with_body(&body);
    ///
    /// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
    /// assert_eq!(m.signature(), Signature::new(b"us")?);
    ///
    /// let mut r = m.body();
    /// assert_eq!(r.load::<u32>()?, 42);
    /// assert_eq!(r.read::<str>()?, "Hello World!");
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[must_use]
    pub fn body(&self) -> BodyReadBuf<'a> {
        self.body.clone()
    }

    /// Get the serial of the message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use tokio_dbus::{Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.serial().get(), 1);
    ///
    /// let m2 = m.with_serial(NonZeroU32::new(1000).unwrap());
    /// assert_eq!(m2.serial().get(), 1000);
    /// ```
    #[must_use]
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
    /// use tokio_dbus::{Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.serial().get(), 1);
    ///
    /// let m2 = m.with_serial(NonZeroU32::new(1000).unwrap());
    /// assert_eq!(m2.serial().get(), 1000);
    /// ```
    #[must_use]
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
    /// use tokio_dbus::{Flags, Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.flags(), Flags::default());
    ///
    /// let m2 = m.with_flags(Flags::NO_REPLY_EXPECTED);
    /// assert_eq!(m2.flags(), Flags::NO_REPLY_EXPECTED);
    /// ```
    #[must_use]
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
    /// use tokio_dbus::{Flags, Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.flags(), Flags::default());
    ///
    /// let m2 = m.with_flags(Flags::NO_REPLY_EXPECTED);
    /// assert_eq!(m2.flags(), Flags::NO_REPLY_EXPECTED);
    /// ```
    #[must_use]
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
    /// use tokio_dbus::{Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.interface(), None);
    ///
    /// let m2 = m.with_interface("org.freedesktop.DBus");
    /// assert_eq!(m2.interface(), Some("org.freedesktop.DBus"));
    /// ```
    #[must_use]
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
    /// use tokio_dbus::{Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.interface(), None);
    ///
    /// let m2 = m.with_interface("org.freedesktop.DBus");
    /// assert_eq!(m2.interface(), Some("org.freedesktop.DBus"));
    /// ```
    #[must_use]
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
    /// use tokio_dbus::{Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_destination(":1.131");
    /// assert_eq!(m2.destination(), Some(":1.131"));
    /// ```
    #[must_use]
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
    /// use tokio_dbus::{Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_destination(":1.131");
    /// assert_eq!(m2.destination(), Some(":1.131"));
    /// ```
    #[must_use]
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
    /// use tokio_dbus::{Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_sender(":1.131");
    /// assert_eq!(m2.sender(), Some(":1.131"));
    /// ```
    #[must_use]
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
    /// use tokio_dbus::{Message, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.destination(), None);
    ///
    /// let m2 = m.with_sender(":1.131");
    /// assert_eq!(m2.sender(), Some(":1.131"));
    /// ```
    #[must_use]
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
    /// use tokio_dbus::{BodyBuf, Message, ObjectPath, SendBuf, Signature};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello");
    /// assert_eq!(m.signature(), Signature::EMPTY);
    ///
    /// let mut body = BodyBuf::new();
    /// body.write("Hello World!");
    ///
    /// let m2 = m.with_body(&body);
    /// assert_eq!(m2.signature(), Signature::STRING);
    /// ```
    #[must_use]
    pub fn signature(&self) -> &Signature {
        self.body.signature()
    }

    pub(crate) fn message_type(&self) -> crate::proto::MessageType {
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
            && self.body == other.body
    }
}
