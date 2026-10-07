# tokio-dbus

[<img alt="github" src="https://img.shields.io/badge/github-udoprog/tokio--dbus-8da0cb?style=for-the-badge&logo=github" height="20">](https://github.com/udoprog/tokio-dbus)
[<img alt="crates.io" src="https://img.shields.io/crates/v/tokio-dbus.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/tokio-dbus)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-tokio--dbus-66c2a5?style=for-the-badge&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K" height="20">](https://docs.rs/tokio-dbus)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/udoprog/tokio-dbus/ci.yml?branch=main&style=for-the-badge" height="20">](https://github.com/udoprog/tokio-dbus/actions?query=branch%3Amain)

An asynchronous D-Bus implementation for the Tokio ecosystem.

This crate is the low-level, borrowed API: it is sufficient to write efficient
clients and servers by hand. For proxies and servers generated from interface
xml, see [`tokio-dbus-codegen`] and [`tokio-dbus-runtime`].

To currently see how it's used, see:
* [examples/client.rs](https://github.com/udoprog/tokio-dbus/blob/main/examples/examples/client.rs)
* [examples/server.rs](https://github.com/udoprog/tokio-dbus/blob/main/examples/examples/server.rs)
* [examples/systray.rs](https://github.com/udoprog/tokio-dbus/blob/main/examples/examples/systray.rs),
  a systray icon with a menu, implementing `org.kde.StatusNotifierItem` and
  `com.canonical.dbusmenu`.
* [examples/notification.rs](https://github.com/udoprog/tokio-dbus/blob/main/examples/examples/notification.rs),
  sending a desktop notification through `org.freedesktop.Notifications`.

The systray and notification examples have a counterpart which does the same
thing through bindings generated from an interface file by
[`tokio-dbus-codegen`], driven by [`tokio-dbus-runtime`]:
[examples/systray_codegen.rs](https://github.com/udoprog/tokio-dbus/blob/main/examples/examples/systray_codegen.rs)
and
[examples/notification_codegen.rs](https://github.com/udoprog/tokio-dbus/blob/main/examples/examples/notification_codegen.rs).

[`tokio-dbus-codegen`]: https://docs.rs/tokio-dbus-codegen
[`tokio-dbus-runtime`]: https://docs.rs/tokio-dbus-runtime
