//! The same desktop notification as [`notification.rs`], sent through bindings
//! generated from an interface file instead of assembled by hand.
//!
//! [`notification.rs`]: https://github.com/udoprog/tokio-dbus/blob/main/examples/examples/notification.rs
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example notification_codegen
//! ```
//!
//! `build.rs` reads `interfaces/org.freedesktop.Notifications.xml` and asks
//! [`tokio_dbus_codegen`] for a client, which lands in `OUT_DIR` and is pulled
//! in by the `include!` below. Comparing the two examples side by side shows
//! what the generated layer takes over: signatures, argument order and the
//! decoding of replies and signals.
//!
//! The tradeoff is that everything crossing the boundary is an owned Rust value,
//! so a `a{sv}` of hints becomes a `HashMap<String, Value>` which is built up
//! and then encoded, rather than written straight into the message buffer.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use tokio_dbus::ObjectPath;
use tokio_dbus_runtime::{Connection, Incoming, Value};

include!(concat!(env!("OUT_DIR"), "/notifications.rs"));

use self::notifications::{Notifications, Signal};

const DESTINATION: &str = "org.freedesktop.Notifications";
const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/Notifications");

/// How long to wait for the user to act on the notification before giving up.
const TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> Result<()> {
    let mut conn = Connection::session_bus().await?;
    println!("Connected as {}", conn.unique_name());

    // Every generated module carries a match rule which selects the signals of
    // its interface.
    conn.add_match(notifications::MATCH_RULE).await?;

    let server = Notifications::new(DESTINATION, PATH);

    let (name, vendor, version, spec) = server.get_server_information(&mut conn).await?;
    println!("Server: {name} {version} by {vendor}, implementing spec {spec}");

    let capabilities = server.get_capabilities(&mut conn).await?;
    println!("Capabilities: {}", capabilities.join(", "));

    if !capabilities.iter().any(|c| c == "actions") {
        println!("This server does not support actions, so the buttons are hidden");
    }

    let id = server
        .notify(
            &mut conn,
            "tokio-dbus",
            // A `replaces_id` of 0 asks for a new notification rather than an
            // update to an existing one.
            0,
            "",
            "Hello from tokio-dbus",
            "This notification was sent by the <b>notification_codegen</b> example.",
            &actions(),
            &hints(),
            // -1 leaves it to the server to decide when the notification
            // expires.
            -1,
        )
        .await?;

    println!("Sent notification {id}, waiting for it to be acted on");

    loop {
        let Ok(incoming) = tokio::time::timeout(TIMEOUT, conn.next()).await else {
            println!("Gave up waiting after {TIMEOUT:?}");
            break;
        };

        let Incoming::Signal(message) = incoming? else {
            continue;
        };

        // The server broadcasts these for every notification it handles, so the
        // id has to be checked against the one which was handed out above.
        match Signal::decode(&message)? {
            Some(Signal::ActionInvoked {
                id: signal,
                action_key,
            }) if signal == id => {
                println!("Action invoked: {action_key}");
            }
            Some(Signal::NotificationClosed { id: signal, reason }) if signal == id => {
                println!("Notification closed: {}", describe(reason));
                break;
            }
            _ => {}
        }
    }

    conn.flush().await?;
    Ok(())
}

/// Actions are pairs of a key, which comes back in `ActionInvoked`, and a label
/// to put on the button.
fn actions() -> Vec<String> {
    ["default", "Open", "later", "Remind me later"]
        .into_iter()
        .map(String::from)
        .collect()
}

/// The hints, which are where a notification carries everything that is not one
/// of the fixed arguments.
fn hints() -> HashMap<String, Value> {
    let mut hints = HashMap::new();

    // 0 is low, 1 is normal and 2 is critical.
    hints.insert(String::from("urgency"), Value::U8(1));
    hints.insert(String::from("category"), Value::String("device".into()));
    hints.insert(
        String::from("desktop-entry"),
        Value::String("tokio-dbus".into()),
    );

    let (width, height, pixels) = image();

    // `image-data` is a `(iiibiiay)` of width, height, rowstride, whether there
    // is an alpha channel, bits per sample, channels and the pixels. Note that
    // they are RGBA in that order, unlike the ARGB32 a status notifier item
    // wants.
    hints.insert(
        String::from("image-data"),
        Value::Struct(vec![
            Value::I32(width),
            Value::I32(height),
            Value::I32(width * 4),
            Value::Bool(true),
            Value::I32(8),
            Value::I32(4),
            Value::Array {
                element: tokio_dbus::Signature::BYTE.to_owned(),
                values: pixels.into_iter().map(Value::U8).collect(),
            },
        ]),
    );

    hints
}

/// Describe why the server closed the notification.
fn describe(reason: u32) -> &'static str {
    match reason {
        1 => "it expired",
        2 => "it was dismissed by the user",
        3 => "CloseNotification was called",
        _ => "of an undefined reason",
    }
}

/// Generate a 48x48 RGBA ring.
fn image() -> (i32, i32, Vec<u8>) {
    const SIZE: i32 = 48;

    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let center = (SIZE - 1) as f32 / 2.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();

            let outer = (center - distance + 0.5).clamp(0.0, 1.0);
            let inner = (distance - center * 0.55 + 0.5).clamp(0.0, 1.0);
            let alpha = (outer * inner * 255.0) as u8;

            data.extend_from_slice(&[0x2f, 0x81, 0xf7, alpha]);
        }
    }

    (SIZE, SIZE, data)
}
