use std::num::NonZeroU32;

use crate::proto;

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
    pub(crate) body_length: usize,
    pub(crate) headers: usize,
}
