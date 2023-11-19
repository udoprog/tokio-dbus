use tokio_dbus::{Client, Message, MessageKind, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut c = Client::session_bus().await?;

    let m = Message::method_call("/org/freedesktop/DBus", "Hello")
        .with_destination("org.freedesktop.DBus");

    let serial = c.write_message(&m)?;

    let message = c.process().await?;

    assert_eq!(
        message.kind(),
        MessageKind::MethodReturn {
            reply_serial: serial
        }
    );

    let mut body = message.body();
    let name = body.read::<str>()?;
    dbg!(message, body.len(), name);
    Ok(())
}
