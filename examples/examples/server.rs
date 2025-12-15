use anyhow::{Context, Result, bail};
use tokio_dbus::org_freedesktop_dbus::{self, NameFlag, NameReply};
use tokio_dbus::{BodyBuf, Buffers, Connection, Message, MessageKind, ObjectPath, SendBuf};

const NAME: &str = "se.tedro.DBusExample";
const INTERFACE: &str = "se.tedro.DBusExample.Pingable";
const PATH: &ObjectPath = ObjectPath::new_const(b"/se/tedro/DBusExample");

#[tokio::main]
async fn main() -> Result<()> {
    let mut buf = Buffers::new();
    let mut c = Connection::session_bus()?;

    c.connect(&mut buf).await?;

    let hello_reply = buf.hello()?;
    let name_reply = buf.request_name(NAME, NameFlag::DO_NOT_QUEUE)?;

    loop {
        c.wait(&mut buf).await?;
        let message = buf.recv.last_message()?;

        match message.kind() {
            MessageKind::MethodReturn { reply_serial } if reply_serial == hello_reply => {
                dbg!(message.body().read::<str>()?);
            }
            MessageKind::MethodReturn { reply_serial } if reply_serial == name_reply => {
                let reply = message.body().load::<NameReply>()?;

                if reply != NameReply::PRIMARY_OWNER {
                    bail!("Could not acquire name: {reply:?}");
                }

                dbg!("name acquired");
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } => {
                let message = message.body().read::<str>()?;
                bail!("{error_name}: {reply_serial}: {message}");
            }
            MessageKind::MethodCall { path, member } => {
                buf.body.clear();

                let ret = match handle_method_call(
                    path,
                    member,
                    &message,
                    &mut buf.send,
                    &mut buf.body,
                ) {
                    Ok(m) => m,
                    Err(error) => {
                        // Clear the body in case handler buffered something before
                        // erroring.
                        buf.body.clear();
                        buf.body.store(error.to_string())?;

                        message
                            .error("se.tedro.DBusExample.Error", buf.send.next_serial())
                            .with_body(&buf.body)
                    }
                };

                buf.send.write_message(ret)?;
            }
            MessageKind::Signal { .. }
                if message.interface() == Some(org_freedesktop_dbus::INTERFACE) =>
            {
                // Ignore signals from the bus.
            }
            _ => {
                dbg!(&message);
            }
        }
    }
}

/// Handle a method call.
fn handle_method_call<'a>(
    path: &'a ObjectPath,
    member: &'a str,
    msg: &Message<'a>,
    send: &mut SendBuf,
    body: &'a mut BodyBuf,
) -> Result<Message<'a>> {
    let interface = msg.interface().context("Missing interface")?;

    if path != PATH {
        bail!("Bad path: {path}");
    };

    let m = match interface {
        INTERFACE => match member {
            "Ping" => {
                let value = msg.body().load::<u32>()?;
                body.store(value)?;
                msg.method_return(send.next_serial()).with_body(body)
            }
            method => bail!("Unknown method: {method}"),
        },
        interface => bail!("Unknown interface: {interface}"),
    };

    Ok(m)
}
