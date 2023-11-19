use std::num::NonZeroU32;

use crate::protocol::{Flags, MessageType};
use crate::{ReadBuf, Signature};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageKind<'a> {
    MethodCall {
        /// The path being called.
        path: &'a str,
        /// The member being called.
        member: &'a str,
    },
    MethodReturn {
        /// The serial this is a reply to.
        reply_serial: NonZeroU32,
    },
    Error {
        /// The name of the error.
        error_name: &'a str,
        /// The serial this is a reply to.
        reply_serial: NonZeroU32,
    },
    Signal {
        /// The member being signalled.
        member: &'a str,
    },
}

/// A D-Bus message.
#[derive(Debug)]
pub struct Message<'a> {
    /// The type of the message.
    pub(crate) kind: MessageKind<'a>,
    /// Serial of the emssage.
    pub(crate) serial: Option<NonZeroU32>,
    /// Flags in the message.
    pub(crate) flags: Flags,
    /// The interface of the message.
    pub(crate) interface: Option<&'a str>,
    /// The destination of the message.
    pub(crate) destination: Option<&'a str>,
    /// The sender of the message.
    pub(crate) sender: Option<&'a str>,
    /// The signature of the body.
    pub(crate) signature: &'a Signature,
    /// The body associated with the message.
    pub(crate) body: ReadBuf<'a>,
}

impl<'a> Message<'a> {
    /// Construct a method call.
    pub fn method_call(path: &'a str, member: &'a str) -> Self {
        Self {
            kind: MessageKind::MethodCall { path, member },
            serial: None,
            flags: Flags::EMPTY,
            interface: None,
            destination: None,
            sender: None,
            signature: Signature::empty(),
            body: ReadBuf::empty(),
        }
    }

    /// Get the kind of the message.
    pub fn kind(&self) -> MessageKind<'a> {
        self.kind
    }

    /// Get a buffer to the body of the message.
    pub fn body(&self) -> ReadBuf<'a> {
        self.body.clone()
    }

    /// Get the signature of the message.
    pub fn signature(&self) -> &Signature {
        self.signature
    }

    /// Modify the serial of the message.
    pub fn with_serial(self, serial: NonZeroU32) -> Self {
        Self {
            serial: Some(serial),
            ..self
        }
    }

    /// Modify the flags of the message.
    pub fn with_flags(self, flags: Flags) -> Self {
        Self { flags, ..self }
    }

    /// Modify the interface of the message.
    pub fn with_interface(self, interface: &'a str) -> Self {
        Self {
            interface: Some(interface),
            ..self
        }
    }

    /// Modify the destination of the message.
    pub fn with_destination(self, destination: &'a str) -> Self {
        Self {
            destination: Some(destination),
            ..self
        }
    }

    /// Modify the sender of the message.
    pub fn with_sender(self, sender: &'a str) -> Self {
        Self {
            sender: Some(sender),
            ..self
        }
    }

    /// Modify the signature of the message.
    pub fn with_signature(self, signature: &'a Signature) -> Self {
        Self { signature, ..self }
    }

    pub(crate) fn message_type(&self) -> crate::protocol::MessageType {
        match self.kind {
            MessageKind::MethodCall { .. } => MessageType::METHOD_CALL,
            MessageKind::MethodReturn { .. } => MessageType::METHOD_RETURN,
            MessageKind::Error { .. } => MessageType::ERROR,
            MessageKind::Signal { .. } => MessageType::SIGNAL,
        }
    }
}
