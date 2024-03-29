//! [<img alt="github" src="https://img.shields.io/badge/github-udoprog/tokio--dbus-8da0cb?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/tokio-dbus)
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/tokio-dbus.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/tokio-dbus)
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-tokio--dbus-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/tokio-dbus)
//!
//! An asynchronous D-Bus implementation for the Tokio ecosystem.
//!
//! So far this is a fairly low-level implementation, but is sufficient to write
//! efficient servers without some of the flair associated with other clients
//! (like proxies generated from xml).
//!
//! To currently see how it's used, see:
//! * [examples/client.rs](https://github.com/udoprog/tokio-dbus/blob/main/examples/examples/client.rs)
//! * [examples/server.rs](https://github.com/udoprog/tokio-dbus/blob/main/examples/examples/server.rs)

#![deny(missing_docs)]
#![allow(clippy::module_inception)]

#[macro_use]
mod macros;

#[doc(inline)]
pub use self::proto::{Endianness, Flags};
#[macro_use]
mod proto;

pub mod org_freedesktop_dbus;

#[doc(inline)]
pub use self::write::Write;
mod write;

#[doc(inline)]
pub use self::read::Read;
mod read;

#[doc(inline)]
pub use self::error::{Error, Result};
mod error;

pub(crate) mod buf;

#[doc(inline)]
pub use self::body_buf::{BodyBuf, StoreArray, StoreStruct};
mod body_buf;

#[doc(inline)]
pub use self::body::{AsBody, Body, LoadArray};
mod body;

#[doc(inline)]
pub use self::send_buf::SendBuf;
mod send_buf;

#[doc(inline)]
pub use self::recv_buf::RecvBuf;
mod recv_buf;

mod sasl;

#[doc(inline)]
pub use self::signature::{Signature, SignatureBuf, SignatureError};
mod signature;

#[doc(inline)]
pub use self::frame::Frame;
mod frame;

#[doc(inline)]
pub use self::storable::Storable;
mod storable;

#[doc(inline)]
pub use self::message::{Message, MessageBuf, MessageKind};
mod message;

#[cfg(feature = "tokio")]
#[doc(inline)]
pub use self::connection::{Connection, ConnectionBuilder};
mod connection;

mod lossy_str;

mod utils;

#[doc(inline)]
pub use self::object_path::{ObjectPath, ObjectPathBuf, ObjectPathError};
mod object_path;

#[doc(inline)]
pub use self::variant::Variant;
mod variant;

pub mod ty;

#[doc(inline)]
pub use self::arguments::Arguments;
mod arguments;
