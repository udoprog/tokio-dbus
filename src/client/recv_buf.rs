use std::num::NonZeroUsize;

use crate::buf::OwnedBuf;
use crate::connection::{read_message, MessageRef};
use crate::error::Result;
use crate::Message;

/// Buffer used for receiving messages through D-Bus.
pub struct RecvBuf {
    pub(super) buf: OwnedBuf,
    /// The amount the receive buffer needs to be advanced before processing can
    /// continue.
    pub(super) advance: Option<NonZeroUsize>,
}

impl RecvBuf {
    /// Construct a new receive buffer.
    pub fn new() -> Self {
        Self {
            buf: OwnedBuf::new(),
            advance: None,
        }
    }

    /// Read a message out of the receive buffer.
    pub fn message(&self, message_ref: &MessageRef) -> Result<Message<'_>> {
        read_message(
            self.buf.peek_buf(message_ref.total),
            message_ref.header,
            message_ref.headers,
        )
    }
}
