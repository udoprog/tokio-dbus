//! Core types for the D-Bus protocol.
//!
//! This is split up into a separate crate so it can be used by macros.

#![allow(clippy::module_inception)]

#[macro_use]
mod macros;

#[doc(hidden)]
pub mod signature;

#[doc(hidden)]
pub mod proto;
