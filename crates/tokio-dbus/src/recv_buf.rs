use std::collections::VecDeque;
use std::mem::size_of;
use std::num::NonZeroU32;

use crate::buf::AlignedBuf;
use crate::error::{Error, ErrorKind, Result};
use crate::proto;
use crate::{Body, Endianness, Message, MessageBuf, MessageKind, ObjectPath, Signature};

/// An owned reference to a message in a [`RecvBuf`].
///
/// To convert into a [`Message`], use [`Connection::read_message`] or
/// [`RecvBuf::read_message`].
///
/// [`Message`]: crate::Message
/// [`Connection::read_message`]: crate::Connection::read_message
/// [`RecvBuf::read_message`]: crate::RecvBuf::read_message
/// [`RecvBuf`]: crate::RecvBuf
pub(crate) struct MessageRef {
    pub(crate) serial: NonZeroU32,
    pub(crate) message_type: proto::MessageType,
    pub(crate) flags: proto::Flags,
    pub(crate) headers: usize,
}

/// Buffer used for receiving messages through D-Bus.
pub struct RecvBuf {
    /// Data of the underlying buffer.
    buf: AlignedBuf,
    /// The currently configured endianness of the receive buffer. This changes
    /// in response to the endianness of the messages being received.
    endianness: Endianness,
    /// The last serial observed. This is used to determine whether a
    /// [`MessageRef`] is valid or not.
    last_message: Option<MessageRef>,
    /// If a message has been taken from the receive buffer.
    deferred_taken: bool,
    /// Stored messages.
    deferred: VecDeque<MessageBuf>,
}

impl RecvBuf {
    /// Construct a new receive buffer.
    pub fn new() -> Self {
        Self {
            buf: AlignedBuf::new(),
            endianness: Endianness::NATIVE,
            last_message: None,
            deferred_taken: false,
            deferred: VecDeque::new(),
        }
    }

    /// Defer the given message, causing it to be received again at the next
    /// wait.
    pub fn defer(&mut self, message: MessageBuf) {
        self.deferred.push_back(message);
    }

    /// Defer the last message.
    pub(crate) fn defer_last(&mut self) -> Result<()> {
        let message = last_message(&self.last_message, &self.buf, self.endianness)?;
        self.deferred.push_back(message.to_owned());
        Ok(())
    }

    /// Try to take a single deferred message.
    pub(crate) fn take_deferred(&mut self) -> bool {
        if self.deferred_taken {
            self.deferred.pop_front();
        }

        self.deferred_taken = !self.deferred.is_empty();
        self.deferred_taken
    }

    /// Access the underlying buffer.
    #[inline]
    pub(crate) fn buf(&self) -> &AlignedBuf {
        &self.buf
    }

    /// Access the underlying buffer mutably.
    #[inline]
    pub(crate) fn buf_mut(&mut self) -> &mut AlignedBuf {
        &mut self.buf
    }

    /// Set last serial.
    #[inline]
    pub(crate) fn set_last_message(&mut self, message_ref: MessageRef) {
        self.last_message = Some(message_ref);
    }

    /// Set endianness of buffer content.
    pub(crate) fn set_endianness(&mut self, endianness: Endianness) {
        self.endianness = endianness;
    }

    /// Clear the receive buffer.
    pub(crate) fn clear(&mut self) {
        self.buf.clear();
        self.last_message = None;
    }

    /// Read the last message buffered.
    ///
    /// This will first read any messages that have been deferred through
    /// [`defer()`]. Then the last message which has been buffered.
    ///
    /// [`defer()`]: Self::defer
    ///
    /// This method should primarily be used in combination with [`wait()`] and
    /// [`wait_no_deferred()`].
    ///
    /// [`wait()`]: crate::Connection::wait
    /// [`wait_no_deferred()`]: crate::Connection::wait_no_deferred
    ///
    /// # Errors
    ///
    /// In case there is no message buffered.
    pub fn last_message(&self) -> Result<Message<'_>> {
        if self.deferred_taken {
            let Some(message) = self.deferred.front() else {
                return Err(Error::new(ErrorKind::MissingMessage));
            };

            return Ok(message.borrow());
        }

        self.last_message_no_deferred()
    }

    /// Read the last message buffered.
    ///
    /// This will only read the last message which has been buffered in the
    /// receive buffer.
    ///
    /// This method should primarily be used in combination with
    /// [`wait_no_deferred()`].
    ///
    /// [`wait_no_deferred()`]: crate::Connection::wait_no_deferred
    ///
    /// # Errors
    ///
    /// In case there is no message buffered.
    #[inline]
    pub fn last_message_no_deferred(&self) -> Result<Message<'_>> {
        last_message(&self.last_message, &self.buf, self.endianness)
    }
}

fn last_message<'a>(
    last_message: &Option<MessageRef>,
    buf: &'a AlignedBuf,
    endianness: Endianness,
) -> Result<Message<'a>> {
    let Some(message_ref) = last_message else {
        return Err(Error::new(ErrorKind::MissingMessage));
    };

    let MessageRef {
        serial,
        message_type,
        flags,
        headers,
    } = *message_ref;

    let mut buf = buf.as_aligned();
    buf.advance(size_of::<proto::Header>() + size_of::<u32>())?;

    let mut path = None;
    let mut interface = None;
    let mut member = None;
    let mut error_name = None;
    let mut reply_serial = None;
    let mut destination = None;
    let mut signature = Signature::empty();
    let mut sender = None;

    // Use a `Body` abstraction here, since we need to adjust the headers by
    // the received endianness.
    let mut buf = Body::from_raw_parts(buf, endianness, Signature::empty());

    let mut st = buf.read_until(headers);

    while !st.is_empty() {
        // NB: Structs are aligned to 8 bytes.
        st.align::<u64>()?;
        let variant = st.load::<proto::Variant>()?;
        let sig = st.read::<Signature>()?;

        match (variant, sig.as_bytes()) {
            (proto::Variant::PATH, b"o") => {
                path = Some(st.read::<ObjectPath>()?);
            }
            (proto::Variant::INTERFACE, b"s") => {
                interface = Some(st.read::<str>()?);
            }
            (proto::Variant::MEMBER, b"s") => {
                member = Some(st.read::<str>()?);
            }
            (proto::Variant::ERROR_NAME, b"s") => {
                error_name = Some(st.read::<str>()?);
            }
            (proto::Variant::REPLY_SERIAL, b"u") => {
                let number = st.load::<u32>()?;
                let number = NonZeroU32::new(number).ok_or(ErrorKind::ZeroReplySerial)?;
                reply_serial = Some(number);
            }
            (proto::Variant::DESTINATION, b"s") => {
                destination = Some(st.read::<str>()?);
            }
            (proto::Variant::SIGNATURE, b"g") => {
                signature = st.read::<Signature>()?;
            }
            (proto::Variant::SENDER, b"s") => {
                sender = Some(st.read::<str>()?);
            }
            (_, _) => {
                crate::signature::skip(sig, &mut st)?;
            }
        }
    }

    buf.align::<u64>()?;

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

    Ok(Message {
        kind,
        serial,
        flags,
        interface,
        destination,
        sender,
        body: buf.with_signature(signature),
    })
}

impl Default for RecvBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
