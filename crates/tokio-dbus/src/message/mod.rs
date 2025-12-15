#[cfg(feature = "alloc")]
pub use self::message_buf::MessageBuf;
#[cfg(feature = "alloc")]
mod message_buf;

#[cfg(feature = "alloc")]
pub(crate) use self::owned_message_kind::OwnedMessageKind;
#[cfg(feature = "alloc")]
mod owned_message_kind;

pub use self::message_kind::MessageKind;
mod message_kind;

pub use self::message::Message;
mod message;

pub use self::serial::Serial;
mod serial;
