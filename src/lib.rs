//! # tokio-dbus
//!
//! An asynchronous D-Bus implementation for the Tokio ecosystem.

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
