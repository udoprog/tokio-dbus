use std::num::NonZeroU32;

use crate::buf::UnalignedBuf;
use crate::error::{Error, ErrorKind, Result};
use crate::{proto, Endianness};
use crate::{Message, MessageKind, ObjectPath, Signature};

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
    /// assert_eq!(send.next_serial().get(), 1);
    /// assert_eq!(send.next_serial().get(), 2);
    /// ```
    pub fn next_serial(&mut self) -> NonZeroU32 {
        loop {
            let Some(serial) = NonZeroU32::new(self.serial.wrapping_add(1)) else {
                self.serial = 1;
                continue;
            };

            self.serial = serial.get();
            break serial;
        }
    }

    /// Construct a method call [`Message`].
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
    pub fn method_call<'a>(&mut self, path: &'a ObjectPath, member: &'a str) -> Message<'a> {
        Message::method_call(path, member, self.next_serial())
    }

    /// Construct a signal [`Message`].
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
    /// let m = send.signal("Hello").to_owned();
    /// let m2 = OwnedMessage::signal("Hello".into(), m.serial());
    /// assert_eq!(m, m2);
    /// ```
    pub fn signal<'a>(&mut self, member: &'a str) -> Message<'a> {
        Message::signal(member, self.next_serial())
    }

    /// Write a message to the buffer.
    pub fn write_message(&mut self, message: &Message<'_>) -> Result<()> {
        self.buf.update_base_align();

        let body = message.body();

        let Some(body_length) = u32::try_from(body.len()).ok() else {
            return Err(Error::new(ErrorKind::BodyTooLong(u32::MAX)));
        };

        self.buf.store(proto::Header {
            endianness: Endianness::NATIVE,
            message_type: message.message_type(),
            flags: message.flags,
            version: 1,
            body_length,
            serial: message.serial.get(),
        });

        let mut array = self.buf.write_array::<u64>();

        match message.kind {
            MessageKind::MethodCall { path, member } => {
                let mut st = array.write_struct();
                st.store(proto::Variant::PATH);
                st.write(Signature::OBJECT_PATH);
                st.write(path);

                let mut st = array.write_struct();
                st.store(proto::Variant::MEMBER);
                st.write(Signature::STRING);
                st.write(member);
            }
            MessageKind::MethodReturn { reply_serial } => {
                let mut st = array.write_struct();
                st.store(proto::Variant::REPLY_SERIAL);
                st.write(Signature::UINT32);
                st.store(reply_serial.get());
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } => {
                let mut st = array.write_struct();
                st.store(proto::Variant::ERROR_NAME);
                st.write(Signature::STRING);
                st.write(error_name);

                let mut st = array.write_struct();
                st.store(proto::Variant::REPLY_SERIAL);
                st.write(Signature::UINT32);
                st.store(reply_serial.get());
            }
            MessageKind::Signal { member } => {
                let mut st = array.write_struct();
                st.store(proto::Variant::MEMBER);
                st.write(Signature::STRING);
                st.write(member);
            }
        }

        if let Some(interface) = message.interface {
            let mut st = array.write_struct();
            st.store(proto::Variant::INTERFACE);
            st.write(Signature::STRING);
            st.write(interface);
        }

        if let Some(destination) = message.destination {
            let mut st = array.write_struct();
            st.store(proto::Variant::DESTINATION);
            st.write(Signature::STRING);
            st.write(destination);
        }

        if let Some(sender) = message.sender {
            let mut st = array.write_struct();
            st.store(proto::Variant::SENDER);
            st.write(Signature::STRING);
            st.write(sender);
        }

        if !message.signature.is_empty() {
            let mut st = array.write_struct();
            st.store(proto::Variant::SIGNATURE);
            st.write(Signature::SIGNATURE);
            st.write(message.signature);
        }

        array.finish();

        self.buf.extend_from_slice(body.get());
        Ok(())
    }
}

impl Default for SendBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
