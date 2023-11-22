use anyhow::{bail, Result};
use tokio_dbus::{Connection, MessageKind, ObjectPath};

const NAME: &str = "se.tedro.DBusExample";
const INTERFACE: &str = "se.tedro.DBusExample.Pingable";
const PATH: &ObjectPath = ObjectPath::new_const(b"/se/tedro/DBusExample");

#[tokio::main]
async fn main() -> Result<()> {
    let mut c = Connection::session_bus().await?;

    let (_, send, body) = c.buffers();

    body.store(42u32)?;

    let m = send
        .method_call(PATH, "Ping")
        .with_destination(NAME)
        .with_interface(INTERFACE)
        .with_body(body);

    let serial = m.serial();

    send.write_message(m)?;

    let reply = loop {
        let message = c.process().await?;
        let message = c.read_message(message)?;

        match message.kind() {
            MessageKind::MethodReturn { reply_serial } if reply_serial == serial => {
                break message.body().load::<u32>()?;
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } if reply_serial == serial => {
                bail!("{error_name}: {}", message.body().read::<str>()?)
            }
            _ => {}
        }
    };

    dbg!(reply);
    Ok(())
}
