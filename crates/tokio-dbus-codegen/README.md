# tokio-dbus-codegen

[<img alt="github" src="https://img.shields.io/badge/github-udoprog/tokio--dbus-8da0cb?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/tokio-dbus)
[<img alt="crates.io" src="https://img.shields.io/crates/v/tokio-dbus-codegen.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/tokio-dbus-codegen)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-tokio--dbus--codegen-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/tokio-dbus-codegen)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/udoprog/tokio-dbus/ci.yml?branch=main&style=for-the-badge" height="20">](https://github.com/udoprog/tokio-dbus/actions?query=branch%3Amain)

Generate asynchronous D-Bus clients and servers from interface files at
build time.

The generated code speaks in owned Rust types [`String`], [`Vec`], and
[`HashMap`] and tuples and is driven by a
[`tokio_dbus_runtime::Connection`], which both crates a consumer needs to
depend on.

[`HashMap`]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
[`tokio_dbus_runtime::Connection`]: https://docs.rs/tokio-dbus-runtime

<br>

## Using it from a build script

Add the generator as a build dependency and the runtime as a regular one:

```toml
[dependencies]
tokio-dbus-runtime = "0.2.2"

[build-dependencies]
tokio-dbus-codegen = "0.2.2"
```

Then write a `build.rs` which names the interface files to read and what to
generate for each interface in them:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tokio_dbus_codegen::Builder::new()
        .file("interfaces/org.freedesktop.Notifications.xml")
        .client("org.freedesktop.Notifications")
        .generate("notifications.rs")?;

    Ok(())
}
```

Finally include the generated file:

```rust
include!(concat!(env!("OUT_DIR"), "/notifications.rs"));
```

<br>

## What is generated

For an interface `com.example.Example`, a module `example` is generated
holding:

* `INTERFACE` and `MATCH_RULE` constants.
* A `Signal` enum with a variant per signal, which can be decoded from an
  incoming message and emitted from an outgoing one. Present whenever the
  interface has signals.
* With [`client()`], an `Example` struct with an `async` method per D-Bus
  method, a getter and setter per property, and `all_properties()`.
* With [`server()`], an `ExampleServer` trait with an `async` method per
  D-Bus method and per property accessor, and a `dispatch()` function which
  routes an incoming call to it. `dispatch()` also answers
  `org.freedesktop.DBus.Properties` for the interface.

[`client()`]: https://docs.rs/tokio-dbus-codegen/latest/tokio_dbus_codegen/struct.Builder.html#method.client
[`server()`]: https://docs.rs/tokio-dbus-codegen/latest/tokio_dbus_codegen/struct.Builder.html#method.server

<br>

## Type mapping

| D-Bus       | Rust                            |
|-------------|---------------------------------|
| `y`         | [`u8`]                          |
| `b`         | [`bool`]                        |
| `n` / `q`   | [`i16`] / [`u16`]               |
| `i` / `u`   | [`i32`] / [`u32`]               |
| `x` / `t`   | [`i64`] / [`u64`]               |
| `d`         | [`f64`]                         |
| `s`         | [`String`]                      |
| `o`         | `ObjectPathBuf`                 |
| `g`         | `SignatureBuf`                  |
| `v`         | `Value`                         |
| `aT`        | `Vec<T>`                        |
| `a{KV}`     | `HashMap<K, V>`                 |
| `(T1 T2)`   | `(T1, T2)`                      |

Client methods take the borrowed form of each argument where borrowing is
free, so an `s` is passed as a [`&str`] and an `as` as a `&[String]`.

Passing a file descriptor (`h`) is not supported.
