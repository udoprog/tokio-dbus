use tokio_dbus::{BodyBuf, Client, RecvBuf, Result, SendBuf};

#[tokio::main]
async fn main() -> Result<()> {
    let mut send = SendBuf::new();
    let mut recv = RecvBuf::new();
    let mut _body = BodyBuf::new();

    let mut c = Client::session_bus(&mut send, &mut recv).await?;

    loop {
        let message = c.process(&mut send, &mut recv).await?;
        let message = recv.message(&message)?;
        dbg!(message);
    }
}
