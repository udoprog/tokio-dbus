//! Parser for D-Bus interface files.

#[cfg(test)]
mod tests;

pub use self::error::{Error, Result};
mod error;

pub use self::elements::{Argument, Description, Direction, Doc, Interface, Method, Node};
mod elements;

pub use self::parser::parse_interface;
mod parser;
