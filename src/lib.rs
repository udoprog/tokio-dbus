//! # tokio-dbus
//!
//! An asynchronous D-Bus implementation for the Tokio ecosystem.
//!
//! So far this is a fairly low-level implementation, but is sufficient to write
//! efficient servers without some of the flair associated with other clients
//! (like proxies generated from xml).
//!
//! To currently see how it's used, see:
//! * [examples/client.rs](https://github.com/udoprog/tokio-dbus/blob/main/examples/client.rs)
//! * [examples/server.rs](https://github.com/udoprog/tokio-dbus/blob/main/examples/server.rs)

#![allow(clippy::module_inception)]

#[doc(inline)]
pub use self::protocol::{Endianness, Flags};
#[macro_use]
pub mod protocol;

pub mod org_freedesktop_dbus;

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

pub use self::buf::{BodyBuf, ReadBuf};
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

pub mod ty;
