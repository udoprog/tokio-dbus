use std::num::NonZeroU32;

use crate::error::{Error, ErrorKind, Result};
use crate::proto;
use crate::{Endianness, Message, MessageKind, ObjectPath, Signature};

use super::{BodyBuf, MessageRef};

/// Buffer used for receiving messages through D-Bus.
pub struct RecvBuf {
    buf: BodyBuf,
    /// The last serial observed. This is used to determine whether a
    /// [`MessageRef`] is valid or not.
    last_message: Option<MessageRef>,
}

impl RecvBuf {
    /// Construct a new receive buffer.
    pub fn new() -> Self {
        Self {
            buf: BodyBuf::new(),
            last_message: None,
        }
    }

    /// Access the underlying buffer mutably.
    #[inline]
    pub(crate) fn buf_mut(&mut self) -> &mut BodyBuf {
        &mut self.buf
    }

    /// Set last serial.
    #[inline]
    pub(crate) fn set_last_message(&mut self, message_ref: MessageRef) {
        self.last_message = Some(message_ref);
    }

    /// Set endianness of buffer content.
    pub(crate) fn set_endianness(&mut self, endianness: Endianness) {
        self.buf.set_endianness(endianness);
    }

    /// Clear the receive buffer.
    pub(crate) fn clear(&mut self) {
        self.buf.clear();
        self.last_message = None;
    }

    /// Read a [`MessageRef`] into a [`Message`].
    ///
    /// Note that if the [`MessageRef`] is outdated by calling process again,
    /// the behavior of this function is not well-defined (but safe).
    ///
    /// # Errors
    ///
    /// Errors if the message reference is out of date, such as if another
    /// message has been received.
    pub fn last_message(&self) -> Result<Message<'_>> {
        let Some(message_ref) = &self.last_message else {
            return Err(Error::new(ErrorKind::InvalidMessageRef));
        };

        let MessageRef {
            serial,
            message_type,
            flags,
            body_length,
            headers,
        } = *message_ref;

        let mut buf = self.buf.peek();

        let mut path = None;
        let mut interface = None;
        let mut member = None;
        let mut error_name = None;
        let mut reply_serial = None;
        let mut destination = None;
        let mut signature = Signature::empty();
        let mut sender = None;

        let mut header_slice = buf.read_until(headers);

        // NB: We perform manual array of struct decoding here.
        while !header_slice.is_empty() {
            // NB: Since these are structs, they're aligned to a 8-byte boundary.
            header_slice.align::<u64>()?;

            let variant = header_slice.load::<proto::Variant>()?;
            let sig = header_slice.read::<Signature>()?;

            match (variant, sig.as_bytes()) {
                (proto::Variant::PATH, b"o") => {
                    path = Some(header_slice.read::<ObjectPath>()?);
                }
                (proto::Variant::INTERFACE, b"s") => {
                    interface = Some(header_slice.read::<str>()?);
                }
                (proto::Variant::MEMBER, b"s") => {
                    member = Some(header_slice.read::<str>()?);
                }
                (proto::Variant::ERROR_NAME, b"s") => {
                    error_name = Some(header_slice.read::<str>()?);
                }
                (proto::Variant::REPLY_SERIAL, b"u") => {
                    let number = header_slice.load::<u32>()?;
                    let number = NonZeroU32::new(number).ok_or(ErrorKind::ZeroReplySerial)?;
                    reply_serial = Some(number);
                }
                (proto::Variant::DESTINATION, b"s") => {
                    destination = Some(header_slice.read::<str>()?);
                }
                (proto::Variant::SIGNATURE, b"g") => {
                    signature = header_slice.read::<Signature>()?;
                }
                (proto::Variant::SENDER, b"s") => {
                    sender = Some(header_slice.read::<str>()?);
                }
                (_, _) => {
                    sig.skip(&mut header_slice)?;
                }
            }
        }

        let kind = match message_type {
            proto::MessageType::METHOD_CALL => {
                let Some(path) = path else {
                    return Err(Error::new(ErrorKind::MissingPath));
                };

                let Some(member) = member else {
                    return Err(Error::new(ErrorKind::MissingMember));
                };

                MessageKind::MethodCall { path, member }
            }
            proto::MessageType::METHOD_RETURN => {
                let Some(reply_serial) = reply_serial else {
                    return Err(Error::new(ErrorKind::MissingReplySerial));
                };

                MessageKind::MethodReturn { reply_serial }
            }
            proto::MessageType::ERROR => {
                let Some(error_name) = error_name else {
                    return Err(Error::new(ErrorKind::MissingErrorName));
                };

                let Some(reply_serial) = reply_serial else {
                    return Err(Error::new(ErrorKind::MissingReplySerial));
                };

                MessageKind::Error {
                    error_name,
                    reply_serial,
                }
            }
            proto::MessageType::SIGNAL => {
                let Some(member) = member else {
                    return Err(Error::new(ErrorKind::MissingMember));
                };

                MessageKind::Signal { member }
            }
            _ => return Err(Error::new(ErrorKind::InvalidProtocol)),
        };

        buf.align::<u64>()?;

        let body = buf.read_until(body_length).with_signature(signature);

        Ok(Message {
            kind,
            serial,
            flags,
            interface,
            destination,
            sender,
            body,
        })
    }
}

impl Default for RecvBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
