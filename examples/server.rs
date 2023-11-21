use anyhow::{bail, Context, Result};
use tokio_dbus::org_freedesktop_dbus::{NameFlag, NameReply};
use tokio_dbus::{BodyBuf, Client, Message, MessageKind};

const NAME: &str = "se.tedro.DBusExample";
const INTERFACE: &str = "se.tedro.DBusExample.Pingable";
const PATH: &str = "/se/tedro/DBusExample";

#[tokio::main]
async fn main() -> Result<()> {
    let mut c = Client::session_bus().await?;

    let reply = c.request_name(NAME, NameFlag::DO_NOT_QUEUE).await?;

    match reply {
        NameReply::PRIMARY_OWNER => {}
        reply => {
            bail!("Could not acquire name: {reply:?}");
        }
    }

    loop {
        let message = c.process().await?;
        let (recv, send, body) = c.buffers();
        let message = recv.message(&message)?;

        dbg!(&message);

        match message.kind() {
            MessageKind::MethodCall { path, member } => {
                let ret = match handle_method_call(path, member, &message, body) {
                    Ok(m) => m,
                    Err(error) => {
                        // Clear the body in case handler buffered something
                        // before erroring.
                        body.clear();
                        body.write(error.to_string().as_str());

                        message
                            .error("se.tedro.JapaneseDictionary.Error")
                            .with_body_buf(body)
                    }
                };

                send.write_message(&ret)?;
            }
            _ => {}
        }
    }
}

/// Handle a method call.
fn handle_method_call<'a>(
    path: &'a str,
    member: &'a str,
    msg: &Message<'a>,
    body: &'a mut BodyBuf,
) -> Result<Message<'a>> {
    let interface = msg.interface().context("Missing interface")?;

    let PATH = path else {
        bail!("Bad path: {}", path);
    };

    let m = match interface {
        INTERFACE => match member {
            "Ping" => {
                let value = msg.body().load::<u32>()?;
                body.store(value);
                msg.method_return().with_body_buf(body)
            }
            method => bail!("Unknown method: {method}"),
        },
        interface => bail!("Unknown interface: {}", interface),
    };

    Ok(m)
}
