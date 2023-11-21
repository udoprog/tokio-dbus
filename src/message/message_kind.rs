use std::num::NonZeroU32;

use crate::message::OwnedMessageKind;

/// The kind of a D-Bus message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MessageKind<'a> {
    /// Method call. This message type may prompt a reply.
    MethodCall {
        /// The path being called.
        path: &'a str,
        /// The member being called.
        member: &'a str,
    },
    /// Method reply with returned data.
    MethodReturn {
        /// The serial this is a reply to.
        reply_serial: NonZeroU32,
    },
    /// Error reply. If the first argument exists and is a string, it is an
    /// error message.
    Error {
        /// The name of the error.
        error_name: &'a str,
        /// The serial this is a reply to.
        reply_serial: NonZeroU32,
    },
    /// Signal emission.
    Signal {
        /// The member being signalled.
        member: &'a str,
    },
}

impl MessageKind<'_> {
    #[inline]
    pub(crate) fn to_owned(self) -> OwnedMessageKind {
        match self {
            MessageKind::MethodCall { path, member } => OwnedMessageKind::MethodCall {
                path: path.into(),
                member: member.into(),
            },
            MessageKind::MethodReturn { reply_serial } => {
                OwnedMessageKind::MethodReturn { reply_serial }
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } => OwnedMessageKind::Error {
                error_name: error_name.into(),
                reply_serial,
            },
            MessageKind::Signal { member } => OwnedMessageKind::Signal {
                member: member.into(),
            },
        }
    }
}

impl PartialEq<OwnedMessageKind> for MessageKind<'_> {
    fn eq(&self, other: &OwnedMessageKind) -> bool {
        match (*self, other) {
            (
                MessageKind::MethodCall {
                    path: path_left,
                    member: member_left,
                },
                OwnedMessageKind::MethodCall {
                    path: path_right,
                    member: member_right,
                },
            ) => *path_left == **path_right && *member_left == **member_right,
            (
                MessageKind::MethodReturn {
                    reply_serial: reply_serial_left,
                },
                OwnedMessageKind::MethodReturn {
                    reply_serial: reply_serial_right,
                },
            ) => reply_serial_left == *reply_serial_right,
            (
                MessageKind::Error {
                    error_name: error_name_left,
                    reply_serial: reply_serial_left,
                },
                OwnedMessageKind::Error {
                    error_name: error_name_right,
                    reply_serial: reply_serial_right,
                },
            ) => *error_name_left == **error_name_right && reply_serial_left == *reply_serial_right,
            (
                MessageKind::Signal {
                    member: member_left,
                },
                OwnedMessageKind::Signal {
                    member: member_right,
                },
            ) => *member_left == **member_right,
            _ => false,
        }
    }
}
