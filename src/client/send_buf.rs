use std::num::NonZeroU32;

use crate::buf::OwnedBuf;
use crate::error::{Error, ErrorKind, Result};
use crate::message::DEFAULT_SERIAL;
use crate::protocol;
use crate::{Message, MessageKind, Signature};

/// Buffer used for sending messages through D-Bus.
pub struct SendBuf {
    pub(super) buf: OwnedBuf,
    serial: NonZeroU32,
}

impl SendBuf {
    /// Construct a new send buffer.
    pub fn new() -> Self {
        Self {
            buf: OwnedBuf::new(),
            serial: DEFAULT_SERIAL,
        }
    }

    fn next_serial(&mut self) -> NonZeroU32 {
        loop {
            let Some(serial) = NonZeroU32::new(self.serial.get().wrapping_add(1)) else {
                self.serial = DEFAULT_SERIAL;
                continue;
            };

            self.serial = serial;
            break serial;
        }
    }

    /// Generate a method call.
    pub fn method_call<'a>(&mut self, path: &'a str, member: &'a str) -> Message<'a> {
        Message::method_call(path, member, self.next_serial())
    }

    /// Write a `message` to the internal buffer and return the serial number
    /// associated with it.
    ///
    /// This can be used to add a message to the internal buffer immediately
    /// without sending it.
    ///
    /// To subsequently send the message you can use [`send_buf()`].
    ///
    /// [`send_buf()`]: Self::send_buf
    pub fn write_message(&mut self, message: &Message<'_>) -> Result<NonZeroU32> {
        self.buf.update_alignment_base();

        let serial = if message.serial == DEFAULT_SERIAL {
            self.next_serial()
        } else {
            message.serial
        };

        let body = message.body();

        let Some(body_length) = u32::try_from(body.len()).ok() else {
            return Err(Error::new(ErrorKind::BodyTooLong(u32::MAX)));
        };

        self.buf.store(protocol::Header {
            endianness: self.buf.endianness(),
            message_type: message.message_type(),
            flags: message.flags,
            version: 1,
            body_length,
            serial: serial.get(),
        });

        let mut array = self.buf.write_array();

        match message.kind {
            MessageKind::MethodCall { path, member } => {
                let mut st = array.write_struct();
                st.store(protocol::Variant::PATH);
                st.write(Signature::OBJECT_PATH);
                st.write(path);

                let mut st = array.write_struct();
                st.store(protocol::Variant::MEMBER);
                st.write(Signature::STRING);
                st.write(member);
            }
            MessageKind::MethodReturn { reply_serial } => {
                let mut st = array.write_struct();
                st.store(protocol::Variant::REPLY_SERIAL);
                st.write(Signature::UINT32);
                st.store(reply_serial.get());
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } => {
                let mut st = array.write_struct();
                st.store(protocol::Variant::ERROR_NAME);
                st.write(Signature::STRING);
                st.write(error_name);

                let mut st = array.write_struct();
                st.store(protocol::Variant::REPLY_SERIAL);
                st.write(Signature::UINT32);
                st.store(reply_serial.get());
            }
            MessageKind::Signal { member } => {
                let mut st = array.write_struct();
                st.store(protocol::Variant::MEMBER);
                st.write(Signature::STRING);
                st.write(member);
            }
        }

        if let Some(interface) = message.interface {
            let mut st = array.write_struct();
            st.store(protocol::Variant::INTERFACE);
            st.write(Signature::STRING);
            st.write(interface);
        }

        if let Some(destination) = message.destination {
            let mut st = array.write_struct();
            st.store(protocol::Variant::DESTINATION);
            st.write(Signature::STRING);
            st.write(destination);
        }

        if let Some(sender) = message.sender {
            let mut st = array.write_struct();
            st.store(protocol::Variant::SENDER);
            st.write(Signature::STRING);
            st.write(sender);
        }

        if !message.signature.is_empty() {
            let mut st = array.write_struct();
            st.store(protocol::Variant::SIGNATURE);
            st.write(Signature::SIGNATURE);
            st.write(message.signature);
        }

        array.finish();
        self.buf.align_mut::<u64>();
        self.buf.extend_from_slice(body.get());
        Ok(serial)
    }
}

impl Default for SendBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
