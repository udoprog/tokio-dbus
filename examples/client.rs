use anyhow::{bail, Result};
use tokio_dbus::{Client, MessageKind};

const NAME: &str = "se.tedro.DBusExample";
const INTERFACE: &str = "se.tedro.DBusExample.Pingable";
const PATH: &str = "/se/tedro/DBusExample";

#[tokio::main]
async fn main() -> Result<()> {
    let mut c = Client::session_bus().await?;

    let (_, send, body) = c.buffers();

    body.store(42u32)?;

    let m = send
        .method_call(PATH, "Ping")
        .with_destination(NAME)
        .with_interface(INTERFACE)
        .with_body_buf(body);

    send.write_message(&m)?;

    let serial = m.serial();

    let reply = loop {
        let message = c.process().await?;
        let message = c.read_message(&message)?;

        match message.kind() {
            MessageKind::MethodReturn { reply_serial } if reply_serial == serial => {
                break message.body().load::<u32>()?;
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } if reply_serial == serial => {
                bail!("Error: {}: {}", error_name, message.body().read::<str>()?)
            }
            _ => {}
        }
    };

    dbg!(reply);
    Ok(())
}
