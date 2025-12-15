use core::num::NonZeroU32;

use crate::buf::UnalignedBuf;
use crate::error::{Error, ErrorKind, Result};
use crate::proto;
use crate::{Endianness, Message, MessageKind, ObjectPath, Serial, Signature};

/// Buffer used for sending messages through D-Bus.
pub struct SendBuf {
    buf: UnalignedBuf,
    serial: u32,
}

impl SendBuf {
    /// Construct a new send buffer.
    pub fn new() -> Self {
        Self {
            buf: UnalignedBuf::new(),
            serial: 0,
        }
    }

    /// Extend the buffer with a slice.
    pub(crate) fn extend_from_slice(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Access the underlying buffer.
    pub(crate) fn buf(&mut self) -> &UnalignedBuf {
        &self.buf
    }

    /// Access the underlying buffer mutably.
    pub(crate) fn buf_mut(&mut self) -> &mut UnalignedBuf {
        &mut self.buf
    }

    /// Get the next serial for this send buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::SendBuf;
    ///
    /// let mut send = SendBuf::new();
    /// assert_ne!(send.next_serial(), send.next_serial());
    /// ```
    pub fn next_serial(&mut self) -> Serial {
        loop {
            let Some(serial) = NonZeroU32::new(self.serial.wrapping_add(1)) else {
                self.serial = 1;
                continue;
            };

            self.serial = serial.get();
            break Serial::new(serial);
        }
    }

    /// Construct a method call [`Message`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Message, MessageBuf, ObjectPath, SendBuf};
    ///
    /// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.method_call(PATH, "Hello").to_owned();
    /// let m2 = MessageBuf::method_call(PATH.into(), "Hello".into(), m.serial());
    /// assert_eq!(m, m2);
    /// ```
    pub fn method_call<'a>(&mut self, path: &'a ObjectPath, member: &'a str) -> Message<'a> {
        Message::method_call(path, member, self.next_serial())
    }

    /// Construct a signal [`Message`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Message, MessageBuf, SendBuf};
    ///
    /// let mut send = SendBuf::new();
    ///
    /// let m = send.signal("Hello").to_owned();
    /// let m2 = MessageBuf::signal("Hello".into(), m.serial());
    /// assert_eq!(m, m2);
    /// ```
    pub fn signal<'a>(&mut self, member: &'a str) -> Message<'a> {
        Message::signal(member, self.next_serial())
    }

    /// Write a message to the buffer.
    pub fn write_message(&mut self, message: Message<'_>) -> Result<Serial> {
        self.buf.update_base_align();

        let body = message.body();

        let Some(body_length) = u32::try_from(body.len()).ok() else {
            return Err(Error::new(ErrorKind::BodyTooLong(u32::MAX)));
        };

        // The following is a section which performs manual header mangling.
        // It's simply easier to do it like this than make sure that all
        // message-writing abstractions are compatible with an unaligned buffer.

        self.buf.store(proto::Header {
            endianness: Endianness::NATIVE,
            message_type: message.message_type(),
            flags: message.flags,
            version: 1,
            body_length,
            serial: message.serial.get(),
        });

        let length = self.buf.alloc::<u32>();
        let start = self.buf.len();

        match message.kind {
            MessageKind::MethodCall { path, member } => {
                self.buf.align_mut::<u64>();
                self.buf.store(proto::Variant::PATH);
                self.buf.write(Signature::OBJECT_PATH);
                self.buf.write(path);

                self.buf.align_mut::<u64>();
                self.buf.store(proto::Variant::MEMBER);
                self.buf.write(Signature::STRING);
                self.buf.write(member);
            }
            MessageKind::MethodReturn { reply_serial } => {
                self.buf.align_mut::<u64>();
                self.buf.store(proto::Variant::REPLY_SERIAL);
                self.buf.write(Signature::UINT32);
                self.buf.store(reply_serial.get());
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } => {
                self.buf.align_mut::<u64>();
                self.buf.store(proto::Variant::ERROR_NAME);
                self.buf.write(Signature::STRING);
                self.buf.write(error_name);

                self.buf.align_mut::<u64>();
                self.buf.store(proto::Variant::REPLY_SERIAL);
                self.buf.write(Signature::UINT32);
                self.buf.store(reply_serial.get());
            }
            MessageKind::Signal { member } => {
                self.buf.align_mut::<u64>();
                self.buf.store(proto::Variant::MEMBER);
                self.buf.write(Signature::STRING);
                self.buf.write(member);
            }
        }

        if let Some(interface) = message.interface {
            self.buf.align_mut::<u64>();
            self.buf.store(proto::Variant::INTERFACE);
            self.buf.write(Signature::STRING);
            self.buf.write(interface);
        }

        if let Some(destination) = message.destination {
            self.buf.align_mut::<u64>();
            self.buf.store(proto::Variant::DESTINATION);
            self.buf.write(Signature::STRING);
            self.buf.write(destination);
        }

        if let Some(sender) = message.sender {
            self.buf.align_mut::<u64>();
            self.buf.store(proto::Variant::SENDER);
            self.buf.write(Signature::STRING);
            self.buf.write(sender);
        }

        if !body.signature().is_empty() {
            self.buf.align_mut::<u64>();
            self.buf.store(proto::Variant::SIGNATURE);
            self.buf.write(Signature::SIGNATURE);
            self.buf.write(body.signature());
        }

        let Ok(header_length) = u32::try_from(self.buf.len().saturating_sub(start)) else {
            return Err(Error::new(ErrorKind::HeaderTooLong(u32::MAX)));
        };

        self.buf.store_at(length, header_length);

        self.buf.align_mut::<u64>();
        self.buf.extend_from_slice(body.get());
        Ok(message.serial)
    }
}

impl Default for SendBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
