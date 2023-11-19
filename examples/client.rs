use tokio_dbus::sasl::{Auth, SaslRequest, SaslResponse};
use tokio_dbus::{Client, Connection, Message, MessageKind, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut c = Client::new(Connection::session_bus()?)?;

    let sasl = c
        .sasl_request(&SaslRequest::Auth(Auth::external_from_uid(&mut [0; 32])))
        .await?;

    match sasl {
        SaslResponse::Ok(..) => {}
    }

    // Transition into message mode.
    c.sasl_begin().await?;

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
