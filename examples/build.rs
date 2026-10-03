//! Generate the bindings used by the `*_codegen` examples.
//!
//! Everything written here lands in `OUT_DIR`, and is pulled into an example
//! with an `include!`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A pure client: the notification server is somebody else's process.
    tokio_dbus_codegen::Builder::new()
        .file("interfaces/org.freedesktop.Notifications.xml")
        .client("org.freedesktop.Notifications")
        .generate("notifications.rs")?;

    // A systray is mostly a server, except for the one call which registers it
    // with the watcher.
    tokio_dbus_codegen::Builder::new()
        .file("interfaces/org.kde.StatusNotifierItem.xml")
        .file("interfaces/com.canonical.dbusmenu.xml")
        .server("org.kde.StatusNotifierItem")
        .server("com.canonical.dbusmenu")
        .client("org.kde.StatusNotifierWatcher")
        .generate("systray.rs")?;

    // Both halves of a small interface, for the tests which check how
    // generated code treats arguments of the wrong type.
    tokio_dbus_codegen::Builder::new()
        .file("tests/se.tedro.Checked.xml")
        .both("se.tedro.Checked")
        .generate("checked.rs")?;

    Ok(())
}
