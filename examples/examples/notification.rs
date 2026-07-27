//! Send a desktop notification through `org.freedesktop.Notifications`, then
//! wait to find out what the user did with it.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example notification
//! ```
//!
//! Unlike the systray example this is a pure client: it calls three methods on
//! the notification server and listens for the two signals the server emits back
//! at it. The interface is implemented by every desktop environment, and by
//! standalone daemons such as `dunst` and `mako`.
//!
//! The interesting part on the wire is the `hints` argument of `Notify`, which
//! is an `a{sv}` whose values range from a single byte to the `(iiibiiay)` of an
//! inline image.

use std::time::Duration;

use anyhow::{Result, bail};
use tokio_dbus::{Buffers, Connection, MessageKind, ObjectPath, Serial, Signature, Variant, ty};

const DESTINATION: &str = "org.freedesktop.Notifications";
const INTERFACE: &str = "org.freedesktop.Notifications";
const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/Notifications");

/// The signature of the `image-data` hint: width, height, rowstride, whether
/// there is an alpha channel, bits per sample, channels and the pixels.
const IMAGE_DATA: &Signature = Signature::new_const(b"(iiibiiay)");

/// The fields of [`IMAGE_DATA`].
type ImageData = (i32, i32, i32, ty::Bool, i32, i32, ty::Array<u8>);

/// How long to wait around for the user to act on the notification before giving
/// up and exiting.
const TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> Result<()> {
    let mut buf = Buffers::new();
    let mut c = Connection::session_bus()?;
    c.connect(&mut buf).await?;

    let hello = buf.hello()?;

    // The server reports what the user did through signals, which the bus only
    // routes to us once a rule matching them has been added.
    buf.add_match(&format!(
        "type='signal',sender='{DESTINATION}',interface='{INTERFACE}'"
    ))?;

    let information = method(&mut buf, "GetServerInformation")?;
    let capabilities = method(&mut buf, "GetCapabilities")?;
    let notify = notify(&mut buf)?;

    // Set once the reply to `Notify` arrives, at which point the server has
    // accepted the notification and given it an id.
    let mut id = None;

    loop {
        if tokio::time::timeout(TIMEOUT, c.wait(&mut buf))
            .await
            .is_err()
        {
            println!("Gave up waiting after {TIMEOUT:?}");
            break;
        }

        let message = buf.recv.last_message()?;

        match message.kind() {
            MessageKind::MethodReturn { reply_serial } if reply_serial == hello => {
                println!("Connected as {}", message.body().read::<str>()?);
            }
            // (out s name, out s vendor, out s version, out s specVersion)
            MessageKind::MethodReturn { reply_serial } if reply_serial == information => {
                let mut body = message.body();
                let name = body.read::<str>()?;
                let vendor = body.read::<str>()?;
                let version = body.read::<str>()?;
                let spec = body.read::<str>()?;
                println!("Server: {name} {version} by {vendor}, implementing spec {spec}");
            }
            // (out as capabilities)
            MessageKind::MethodReturn { reply_serial } if reply_serial == capabilities => {
                let mut body = message.body();
                let mut array = body.load_array::<ty::Str>()?;
                let mut names = Vec::new();

                while let Some(name) = array.read()? {
                    names.push(name);
                }

                println!("Capabilities: {}", names.join(", "));

                if !names.contains(&"actions") {
                    println!("This server does not support actions, so the buttons are hidden");
                }
            }
            // (out u id)
            MessageKind::MethodReturn { reply_serial } if reply_serial == notify => {
                let notification = message.body().load::<u32>()?;
                println!("Sent notification {notification}, waiting for it to be acted on");
                id = Some(notification);
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } => {
                let message = message.body().read::<str>()?;
                bail!("{error_name}: {reply_serial}: {message}");
            }
            MessageKind::Signal { member, .. }
                if message.interface() == Some(INTERFACE) && id.is_some() =>
            {
                let mut body = message.body();

                // Both signals start with the id of the notification they are
                // about, and the server broadcasts them for every notification
                // it handles, not just ours.
                if body.load::<u32>()? != id.unwrap_or_default() {
                    continue;
                }

                match member {
                    // (u id, s actionKey)
                    "ActionInvoked" => {
                        println!("Action invoked: {}", body.read::<str>()?);
                    }
                    // (u id, u reason)
                    "NotificationClosed" => {
                        println!("Notification closed: {}", reason(body.load::<u32>()?));
                        break;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Call a method which takes no arguments.
fn method(buf: &mut Buffers, member: &str) -> Result<Serial> {
    let m = buf
        .send
        .method_call(PATH, member)
        .with_destination(DESTINATION)
        .with_interface(INTERFACE);

    let serial = m.serial();
    buf.send.write_message(m)?;
    Ok(serial)
}

/// Send the notification.
///
/// ```text
/// Notify(in s appName, in u replacesId, in s appIcon, in s summary,
///        in s body, in as actions, in a{sv} hints, in i expireTimeout,
///        out u id)
/// ```
fn notify(buf: &mut Buffers) -> Result<Serial> {
    buf.body.clear();

    // A `replacesId` of 0 asks for a new notification rather than an update to
    // an existing one. The icon is left empty because an image is passed through
    // the hints below instead.
    buf.body.arguments((
        "tokio-dbus",
        0u32,
        "",
        "Hello from tokio-dbus",
        "This notification was sent by the <b>notification</b> example.",
    ))?;

    // Actions are pairs of a key, which comes back in `ActionInvoked`, and a
    // label to put on the button. The key `default` is special: it is invoked
    // when the body of the notification itself is clicked.
    let mut actions = buf.body.store_array::<ty::Str>()?;
    actions.store("default");
    actions.store("Open");
    actions.store("later");
    actions.store("Remind me later");
    actions.finish();

    let (width, height, pixels) = image();

    let mut hints = buf.body.store_array::<ty::Dict<ty::Str, ty::Variant>>()?;

    // 0 is low, 1 is normal and 2 is critical. Critical notifications are the
    // ones which do not expire on their own.
    hints
        .store_entry()
        .store("urgency")
        .store(Variant::U8(1))
        .finish();

    hints
        .store_entry()
        .store("category")
        .store(Variant::String("device"))
        .finish();

    hints
        .store_entry()
        .store("desktop-entry")
        .store(Variant::String("tokio-dbus"))
        .finish();

    // An image sent inline, which takes precedence over `appIcon`. Note that the
    // pixels are RGBA in that order, unlike the ARGB32 that a status notifier
    // item wants.
    hints
        .store_entry()
        .store("image-data")
        .store_variant(IMAGE_DATA, |w| {
            w.store_struct::<ImageData>()
                .store(width)
                .store(height)
                // Rowstride is the number of bytes per row, which for tightly
                // packed RGBA is four per pixel.
                .store(width * 4)
                .store(true)
                .store(8)
                .store(4)
                .store_array(|w| w.write_slice(&pixels))
                .finish();
        })
        .finish();

    hints.finish();

    // -1 leaves it to the server to decide when the notification expires. 0
    // would mean that it never does.
    buf.body.store(-1i32)?;

    debug_assert_eq!(buf.body.signature(), "susssasa{sv}i");

    let m = buf
        .send
        .method_call(PATH, "Notify")
        .with_destination(DESTINATION)
        .with_interface(INTERFACE)
        .with_body(&buf.body);

    let serial = m.serial();
    buf.send.write_message(m)?;
    Ok(serial)
}

/// Describe why the server closed the notification.
fn reason(reason: u32) -> &'static str {
    match reason {
        1 => "it expired",
        2 => "it was dismissed by the user",
        3 => "CloseNotification was called",
        _ => "of an undefined reason",
    }
}

/// Generate a 48x48 RGBA image, which is the byte order the notification spec
/// asks for.
fn image() -> (i32, i32, Vec<u8>) {
    const SIZE: i32 = 48;

    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let center = (SIZE - 1) as f32 / 2.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();

            // A ring, so that the transparency in the middle is visible against
            // whatever the notification is drawn on top of.
            let outer = (center - distance + 0.5).clamp(0.0, 1.0);
            let inner = (distance - center * 0.55 + 0.5).clamp(0.0, 1.0);
            let alpha = (outer * inner * 255.0) as u8;

            data.extend_from_slice(&[0x2f, 0x81, 0xf7, alpha]);
        }
    }

    (SIZE, SIZE, data)
}
