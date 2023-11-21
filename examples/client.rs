use tokio_dbus::{BodyBuf, Client, Message, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut c = Client::session_bus().await?;

    let m = c
        .method_call("/se/tedro/JapaneseDictionary", "GetPort")
        .with_destination("se.tedro.JapaneseDictionary")
        .with_interface("se.tedro.JapaneseDictionary");

    c.write_message(&m)?;

    let message = c.process().await?;
    let message = c.message(&message)?;
    dbg!(&message);
    dbg!(message.body().load::<u16>()?);
    Ok(())
}
