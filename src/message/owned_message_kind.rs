use std::num::NonZeroU32;

use crate::MessageKind;

/// The kind of a D-Bus message.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OwnedMessageKind {
    MethodCall {
        /// The path being called.
        path: Box<str>,
        /// The member being called.
        member: Box<str>,
    },
    MethodReturn {
        /// The serial this is a reply to.
        reply_serial: NonZeroU32,
    },
    Error {
        /// The name of the error.
        error_name: Box<str>,
        /// The serial this is a reply to.
        reply_serial: NonZeroU32,
    },
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
