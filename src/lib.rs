#[macro_use]
mod stack;

#[doc(inline)]
pub use self::ser::Serialize;
mod ser;

#[doc(inline)]
pub use self::de::Deserialize;
mod de;

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
pub use self::signature::{Signature, SignatureError};
mod signature;

mod frame;

pub use self::message::{Message, MessageKind};
mod message;

#[cfg(feature = "tokio")]
pub use self::client::Client;
mod client;

pub use self::client_builder::ClientBuilder;
mod client_builder;

mod lossy_str;

mod utils;
