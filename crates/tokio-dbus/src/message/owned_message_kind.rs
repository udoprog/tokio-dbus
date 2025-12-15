#[cfg(feature = "alloc")]
use alloc::boxed::Box;

use crate::{MessageKind, ObjectPath, Serial};

/// The kind of a D-Bus message.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum OwnedMessageKind {
    /// Method call. This message type may prompt a reply.
    MethodCall {
        /// The path being called.
        path: Box<ObjectPath>,
        /// The member being called.
        member: Box<str>,
    },
    /// Method reply with returned data.
    MethodReturn {
        /// The serial this is a reply to.
        reply_serial: Serial,
    },
    /// Error reply. If the first argument exists and is a string, it is an
    /// error message.
    Error {
        /// The name of the error.
        error_name: Box<str>,
        /// The serial this is a reply to.
        reply_serial: Serial,
    },
    /// Signal emission.
    Signal {
        /// The member being signalled.
        member: Box<str>,
    },
}

impl OwnedMessageKind {
    #[inline]
    pub(crate) fn borrow(&self) -> MessageKind<'_> {
        match *self {
            OwnedMessageKind::MethodCall {
                ref path,
                ref member,
            } => MessageKind::MethodCall { path, member },
            OwnedMessageKind::MethodReturn { reply_serial } => {
                MessageKind::MethodReturn { reply_serial }
            }
            OwnedMessageKind::Error {
                ref error_name,
                reply_serial,
            } => MessageKind::Error {
                error_name,
                reply_serial,
            },
            OwnedMessageKind::Signal { ref member } => MessageKind::Signal { member },
        }
    }
}

impl PartialEq<MessageKind<'_>> for OwnedMessageKind {
    #[inline]
    fn eq(&self, other: &MessageKind<'_>) -> bool {
        *other == *self
    }
}
