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
#[derive(Debug)]
pub struct MessageRef {
    pub(crate) header: proto::Header,
    pub(crate) headers: usize,
    pub(crate) total: usize,
}
