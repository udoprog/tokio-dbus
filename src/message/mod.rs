pub use self::owned_message::OwnedMessage;
mod owned_message;

pub use self::owned_message_kind::OwnedMessageKind;
mod owned_message_kind;

pub use self::message_kind::MessageKind;
mod message_kind;

pub use self::message::Message;
pub(crate) use self::message::DEFAULT_SERIAL;
mod message;
