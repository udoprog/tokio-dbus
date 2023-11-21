use std::num::{NonZeroU32, NonZeroUsize};

use crate::buf::{OwnedBuf, ReadBuf};
use crate::connection::MessageRef;
use crate::error::{Error, ErrorKind, Result};
use crate::protocol;
use crate::{Message, MessageKind, Signature};

/// Buffer used for receiving messages through D-Bus.
pub struct RecvBuf {
    pub(super) buf: OwnedBuf,
    /// The amount the receive buffer needs to be advanced before processing can
    /// continue.
    pub(super) advance: Option<NonZeroUsize>,
    /// The last serial observed. This is used to determine whether a
    /// [`MessageRef`] is valid or not.
    pub(super) last_serial: Option<NonZeroU32>,
}

impl RecvBuf {
    /// Construct a new receive buffer.
    pub fn new() -> Self {
        Self {
            buf: OwnedBuf::new(),
            advance: None,
            last_serial: None,
        }
    }

    /// Read a message out of the receive buffer.
    pub fn message(&self, message_ref: &MessageRef) -> Result<Message<'_>> {
        read_message(
            self.buf
                .peek_buf(message_ref.total)
                .with_endianness(message_ref.header.endianness),
            message_ref.header,
            message_ref.headers,
        )
    }
}

impl Default for RecvBuf {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Read a message out of the buffer.
pub(crate) fn read_message(
    mut buf: ReadBuf<'_>,
    header: protocol::Header,
    headers: usize,
) -> Result<Message<'_>> {
    let serial = NonZeroU32::new(header.serial).ok_or(ErrorKind::ZeroSerial)?;

    let mut path = None;
    let mut interface = None;
    let mut member = None;
    let mut error_name = None;
    let mut reply_serial = None;
    let mut destination = None;
    let mut signature = Signature::empty();
    let mut sender = None;

    let mut header_slice = buf.read_buf(headers);

    while !header_slice.is_empty() {
        // NB: Since these are structs, they're aligned to a 8-byte boundary.
        header_slice.align::<u64>();

        let variant = header_slice.load::<protocol::Variant>()?;
        let sig = header_slice.read::<Signature>()?;

        match (variant, sig.as_bytes()) {
            (protocol::Variant::PATH, b"o") => {
                path = Some(header_slice.read::<str>()?);
            }
            (protocol::Variant::INTERFACE, b"s") => {
                interface = Some(header_slice.read::<str>()?);
            }
            (protocol::Variant::MEMBER, b"s") => {
                member = Some(header_slice.read::<str>()?);
            }
            (protocol::Variant::ERROR_NAME, b"s") => {
                error_name = Some(header_slice.read::<str>()?);
            }
            (protocol::Variant::REPLY_SERIAL, b"u") => {
                let number = header_slice.load::<u32>()?;
                let number = NonZeroU32::new(number).ok_or(ErrorKind::ZeroReplySerial)?;
                reply_serial = Some(number);
            }
            (protocol::Variant::DESTINATION, b"s") => {
                destination = Some(header_slice.read::<str>()?);
            }
            (protocol::Variant::SIGNATURE, b"g") => {
                signature = header_slice.read::<Signature>()?;
            }
            (protocol::Variant::SENDER, b"s") => {
                sender = Some(header_slice.read::<str>()?);
            }
            (_, _) => {
                sig.skip(&mut header_slice)?;
            }
        }
    }

    let kind = match header.message_type {
        protocol::MessageType::METHOD_CALL => {
            let Some(path) = path else {
                return Err(Error::new(ErrorKind::MissingPath));
            };

            let Some(member) = member else {
                return Err(Error::new(ErrorKind::MissingMember));
            };

            MessageKind::MethodCall { path, member }
        }
        protocol::MessageType::METHOD_RETURN => {
            let Some(reply_serial) = reply_serial else {
                return Err(Error::new(ErrorKind::MissingReplySerial));
            };

            MessageKind::MethodReturn { reply_serial }
        }
        protocol::MessageType::ERROR => {
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
        protocol::MessageType::SIGNAL => {
            let Some(member) = member else {
                return Err(Error::new(ErrorKind::MissingMember));
            };

            MessageKind::Signal { member }
        }
        _ => return Err(Error::new(ErrorKind::InvalidProtocol)),
    };

    buf.align::<u64>();

    let body = buf.read_buf(header.body_length as usize);

    Ok(Message {
        kind,
        serial,
        flags: header.flags,
        interface,
        destination,
        sender,
        signature,
        body,
    })
}
