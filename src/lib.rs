#[macro_use]
mod stack;

#[doc(inline)]
pub use self::write::Write;
mod write;

#[doc(inline)]
pub use self::read::Read;
mod read;

#[doc(inline)]
pub use self::connection::Connection;
mod connection;

#[doc(inline)]
pub use self::error::{Error, Result};
mod error;

#[doc(inline)]
pub use self::protocol::{Endianness, Flags};
pub mod protocol;

pub use self::buf::{OwnedBuf, ReadBuf};
pub mod buf;

pub mod sasl;

#[doc(inline)]
pub use self::signature::{OwnedSignature, Signature, SignatureError};
mod signature;

#[doc(inline)]
pub use self::frame::Frame;
mod frame;

pub use self::message::{Message, MessageKind, OwnedMessage, OwnedMessageKind};
mod message;

#[cfg(feature = "tokio")]
pub use self::client::{Client, RecvBuf, SendBuf};
mod client;

pub use self::client_builder::ClientBuilder;
mod client_builder;

mod lossy_str;

mod utils;
