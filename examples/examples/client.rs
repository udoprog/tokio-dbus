use anyhow::{Result, bail};
use tokio_dbus::{Buffers, Connection, MessageKind, ObjectPath};

const NAME: &str = "se.tedro.DBusExample";
const INTERFACE: &str = "se.tedro.DBusExample.Pingable";
const PATH: &ObjectPath = ObjectPath::new_const(b"/se/tedro/DBusExample");

#[tokio::main]
async fn main() -> Result<()> {
    let mut buf = Buffers::new();
    let mut c = Connection::session_bus(&mut buf).await?;
    let hello_serial = buf.hello()?;

    buf.body.store(42u32)?;

    let m = buf
        .send
        .method_call(PATH, "Ping")
        .with_destination(NAME)
        .with_interface(INTERFACE)
        .with_body(&buf.body);

    let request_serial = buf.send.write_message(m)?;

    let reply = loop {
        c.wait(&mut buf).await?;

        let message = buf.recv.last_message()?;

        match message.kind() {
            MessageKind::MethodReturn { reply_serial } if reply_serial == hello_serial => {
                dbg!(message.body().read::<str>()?);
            }
            MessageKind::MethodReturn { reply_serial } if reply_serial == request_serial => {
                break message.body().load::<u32>()?;
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } if reply_serial == request_serial => {
                bail!(
                    "{error_name}: {reply_serial}: {}",
                    message.body().read::<str>()?
                )
            }
            _ => {}
        }
    };

    dbg!(reply);
    Ok(())
}
