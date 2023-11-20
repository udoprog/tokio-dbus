use tokio_dbus::{BodyBuf, Client, Message, RecvBuf, Result, SendBuf};

#[tokio::main]
async fn main() -> Result<()> {
    let mut send = SendBuf::new();
    let mut recv = RecvBuf::new();
    let mut _body = BodyBuf::new();

    let mut c = Client::session_bus(&mut send, &mut recv).await?;

    let m = Message::method_call("/se/tedro/JapaneseDictionary", "GetPort")
        .with_destination("se.tedro.JapaneseDictionary")
        .with_interface("se.tedro.JapaneseDictionary");

    send.write_message(&m)?;

    let message = c.process(&mut send, &mut recv).await?;
    let message = recv.message(&message)?;
    dbg!(&message);
    dbg!(message.body().load::<u16>()?);
    Ok(())
}
