//! [<img alt="github" src="https://img.shields.io/badge/github-udoprog/tokio--dbus-8da0cb?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/tokio-dbus)
//! [<img alt="crates.io" src="https://img.shields.io/crates/v/tokio-dbus-codegen.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/tokio-dbus-codegen)
//! [<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-tokio--dbus--codegen-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/tokio-dbus-codegen)
//!
//! Generate asynchronous D-Bus clients and servers from interface files at
//! build time.
//!
//! The generated code speaks in owned Rust types [`String`], [`Vec`], and
//! [`HashMap`] and tuples and is driven by a
//! [`tokio_dbus_runtime::Connection`], which both crates a consumer needs to
//! depend on.
//!
//! [`HashMap`]: std::collections::HashMap
//! [`tokio_dbus_runtime::Connection`]: https://docs.rs/tokio-dbus-runtime
//!
//! # Using it from a build script
//!
//! Add the generator as a build dependency and the runtime as a regular one:
//!
//! ```toml
//! [dependencies]
//! tokio-dbus-runtime = "0.1.0"
//!
//! [build-dependencies]
//! tokio-dbus-codegen = "0.1.0"
//! ```
//!
//! Then write a `build.rs` which names the interface files to read and what to
//! generate for each interface in them:
//!
//! ```no_run
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     tokio_dbus_codegen::Builder::new()
//!         .file("interfaces/org.freedesktop.Notifications.xml")
//!         .client("org.freedesktop.Notifications")
//!         .generate("notifications.rs")?;
//!
//!     Ok(())
//! }
//! ```
//!
//! Finally include the generated file:
//!
//! ```rust,ignore
//! include!(concat!(env!("OUT_DIR"), "/notifications.rs"));
//! ```
//!
//! # What is generated
//!
//! For an interface `com.example.Example`, a module `example` is generated
//! holding:
//!
//! * `INTERFACE` and `MATCH_RULE` constants.
//! * A `Signal` enum with a variant per signal, which can be decoded from an
//!   incoming message and emitted from an outgoing one. Present whenever the
//!   interface has signals.
//! * With [`client()`], an `Example` struct with an `async` method per D-Bus
//!   method, a getter and setter per property, and `all_properties()`.
//! * With [`server()`], an `ExampleServer` trait with an `async` method per
//!   D-Bus method and per property accessor, and a `dispatch()` function which
//!   routes an incoming call to it. `dispatch()` also answers
//!   `org.freedesktop.DBus.Properties` for the interface.
//!
//! [`client()`]: Builder::client
//! [`server()`]: Builder::server
//!
//! # Type mapping
//!
//! | D-Bus       | Rust                            |
//! |-------------|---------------------------------|
//! | `y`         | [`u8`]                          |
//! | `b`         | [`bool`]                        |
//! | `n` / `q`   | [`i16`] / [`u16`]               |
//! | `i` / `u`   | [`i32`] / [`u32`]               |
//! | `x` / `t`   | [`i64`] / [`u64`]               |
//! | `d`         | [`f64`]                         |
//! | `s`         | [`String`]                      |
//! | `o`         | `ObjectPathBuf`                 |
//! | `g`         | `SignatureBuf`                  |
//! | `v`         | `Value`                         |
//! | `aT`        | `Vec<T>`                        |
//! | `a{KV}`     | `HashMap<K, V>`                 |
//! | `(T1 T2)`   | `(T1, T2)`                      |
//!
//! Client methods take the borrowed form of each argument where borrowing is
//! free, so an `s` is passed as a [`&str`] and an `as` as a `&[String]`.
//!
//! Passing a file descriptor (`h`) is not supported.

#![deny(missing_docs)]

#[cfg(test)]
mod tests;

#[doc(inline)]
pub use self::error::{Error, Result};
mod error;

#[doc(inline)]
pub use self::naming::{argument_name, module_name, pascal_case, snake_case};
mod naming;

#[doc(inline)]
pub use self::types::{owned_type, parameter_type};
mod types;

mod generate;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use genco::prelude::*;

use self::error::ErrorKind;

/// What to generate for an interface.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    /// Generate an asynchronous client.
    pub client: bool,
    /// Generate a server trait and a dispatcher.
    pub server: bool,
}

/// Reads interface files and generates bindings for the interfaces in them.
///
/// # Examples
///
/// ```no_run
/// tokio_dbus_codegen::Builder::new()
///     .file("interfaces/org.kde.StatusNotifierItem.xml")
///     .file("interfaces/com.canonical.dbusmenu.xml")
///     .server("org.kde.StatusNotifierItem")
///     .server("com.canonical.dbusmenu")
///     .client("org.kde.StatusNotifierWatcher")
///     .generate("systray.rs")?;
/// # Ok::<_, tokio_dbus_codegen::Error>(())
/// ```
#[derive(Debug, Default)]
pub struct Builder {
    files: Vec<PathBuf>,
    modes: Vec<(String, Mode)>,
}

impl Builder {
    /// Construct an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Read an interface file.
    ///
    /// Every interface in the file becomes available to [`client()`],
    /// [`server()`] and [`both()`], including the ones in child nodes.
    ///
    /// [`client()`]: Self::client
    /// [`server()`]: Self::server
    /// [`both()`]: Self::both
    pub fn file(mut self, path: impl AsRef<Path>) -> Self {
        self.files.push(path.as_ref().to_owned());
        self
    }

    /// Generate an asynchronous client for the named interface.
    pub fn client(self, interface: impl AsRef<str>) -> Self {
        self.mode(interface, |mode| mode.client = true)
    }

    /// Generate a server trait and dispatcher for the named interface.
    pub fn server(self, interface: impl AsRef<str>) -> Self {
        self.mode(interface, |mode| mode.server = true)
    }

    /// Generate both a client and a server for the named interface.
    pub fn both(self, interface: impl AsRef<str>) -> Self {
        self.mode(interface, |mode| {
            mode.client = true;
            mode.server = true;
        })
    }

    fn mode(mut self, interface: impl AsRef<str>, apply: impl FnOnce(&mut Mode)) -> Self {
        let interface = interface.as_ref();

        if let Some((_, mode)) = self.modes.iter_mut().find(|(name, _)| name == interface) {
            apply(mode);
            return self;
        }

        let mut mode = Mode::default();
        apply(&mut mode);
        self.modes.push((interface.to_owned(), mode));
        self
    }

    /// Generate into `$OUT_DIR/<name>`, which is where a build script should
    /// put it.
    ///
    /// This also emits a `cargo::rerun-if-changed` line for every interface file
    /// which was read.
    pub fn generate(self, name: impl AsRef<Path>) -> Result<PathBuf> {
        let Some(out_dir) = env::var_os("OUT_DIR") else {
            return Err(Error::new(ErrorKind::MissingOutDir));
        };

        for file in &self.files {
            println!("cargo::rerun-if-changed={}", file.display());
        }

        let path = PathBuf::from(out_dir).join(name.as_ref());
        self.write_to(&path)?;
        Ok(path)
    }

    /// Generate into an explicit path.
    pub fn write_to(self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let output = self.to_string()?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| Error::io(parent, error))?;
        }

        fs::write(path, output).map_err(|error| Error::io(path, error))?;
        Ok(())
    }

    /// Generate the bindings and return them as a string.
    ///
    /// This is what [`generate()`] and [`write_to()`] use, and is useful for
    /// inspecting the output in a test.
    ///
    /// [`generate()`]: Self::generate
    /// [`write_to()`]: Self::write_to
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> Result<String> {
        let mut sources = Vec::new();

        for file in &self.files {
            let source = fs::read_to_string(file).map_err(|error| Error::io(file, error))?;
            sources.push((file.clone(), source));
        }

        let mut interfaces = HashMap::new();

        for (path, source) in &sources {
            let node = tokio_dbus_xml::parse_interface(source)
                .map_err(|error| Error::from(error).context(path.display()))?;

            for interface in node.all_interfaces() {
                interfaces.insert(interface.name.to_owned(), interface.clone());
            }
        }

        let mut tokens = rust::Tokens::new();

        tokens.append(quote! {
            $("// This file is generated by tokio-dbus-codegen. Do not edit it by hand.")
        });

        tokens.line();

        for (name, mode) in &self.modes {
            let Some(interface) = interfaces.get(name.as_str()) else {
                return Err(Error::new(ErrorKind::MissingInterface(
                    name.as_str().into(),
                )));
            };

            let generated = self::generate::interface(interface, *mode)
                .map_err(|error| error.context(format!("interface {name}")))?;

            tokens.append(generated);
            tokens.line();
        }

        Ok(tokens.to_file_string()?)
    }
}
