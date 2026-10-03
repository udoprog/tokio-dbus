use std::os::unix::net::UnixStream as StdUnixStream;

use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
use tokio_dbus::{BodyBuf, Buffers, ConnectionBuilder, MessageBuf, MessageKind, ObjectPath};

use crate::{Arguments, Connection, Incoming, Result};

const PATH: &ObjectPath = ObjectPath::new_const(b"/se/tedro/Test");
const INTERFACE: &str = "se.tedro.Test";
const BEGIN: &[u8] = b"\0BEGIN\r\n";

/// The bus side of a [`pair()`], scripted through the low level API.
struct Peer {
    connection: tokio_dbus::Connection,
    buf: Buffers,
    hello: MessageBuf,
}

impl Peer {
    async fn recv(&mut self) -> Result<MessageBuf> {
        self.connection.wait(&mut self.buf).await?;
        Ok(self.buf.recv.last_message()?.to_owned())
    }

    fn reply(&mut self, call: &MessageBuf, body: &BodyBuf) -> Result<()> {
        let serial = self.buf.send.next_serial();
        let m = call.borrow().method_return(serial).with_body(body);
        self.buf.send.write_message(m)?;
        Ok(())
    }

    fn ping(&mut self, n: u32) -> Result<()> {
        let mut body = BodyBuf::new();
        body.store(n)?;

        let m = self
            .buf
            .send
            .signal(PATH, "Ping")
            .with_interface(INTERFACE)
            .with_body(&body);

        self.buf.send.write_message(m)?;
        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        self.connection.flush(&mut self.buf).await?;
        Ok(())
    }
}

/// Connect a runtime connection to a scripted peer over socket pairs.
///
/// Neither side authenticates, so each starts by sending `BEGIN`, which a proxy
/// between them strips before passing everything else through.
async fn pair() -> Result<(Connection, Peer)> {
    let (client, a) = StdUnixStream::pair()?;
    let (peer, b) = StdUnixStream::pair()?;
    a.set_nonblocking(true)?;
    b.set_nonblocking(true)?;
    let mut a = UnixStream::from_std(a)?;
    let mut b = UnixStream::from_std(b)?;

    tokio::spawn(async move {
        let mut prefix = [0; BEGIN.len()];
        a.read_exact(&mut prefix).await.unwrap();
        assert_eq!(prefix, BEGIN);
        b.read_exact(&mut prefix).await.unwrap();
        assert_eq!(prefix, BEGIN);
        _ = tokio::io::copy_bidirectional(&mut a, &mut b).await;
    });

    let mut connection = ConnectionBuilder::new().no_auth().build_with_stream(peer)?;
    let mut buf = Buffers::new();
    connection.connect(&mut buf).await?;

    let client = ConnectionBuilder::new()
        .no_auth()
        .build_with_stream(client)?;

    let client = tokio::spawn(Connection::from_connection(client));

    connection.wait(&mut buf).await?;
    let hello = buf.recv.last_message()?.to_owned();
    assert!(matches!(
        hello.kind(),
        MessageKind::MethodCall {
            member: "Hello",
            ..
        }
    ));

    let mut peer = Peer {
        connection,
        buf,
        hello,
    };

    let mut body = BodyBuf::new();
    body.store(":1.1")?;
    let hello = peer.hello.clone();
    peer.reply(&hello, &body)?;
    peer.flush().await?;

    let client = client.await.unwrap()?;
    assert_eq!(client.unique_name(), ":1.1");
    Ok((client, peer))
}

#[track_caller]
fn expect_ping(incoming: Incoming) -> u32 {
    let Incoming::Signal(signal) = incoming else {
        panic!("Expected a signal, got {incoming:?}");
    };

    assert_eq!(signal.member(), "Ping");
    signal.body().load::<u32>().unwrap()
}

#[tokio::test]
async fn flush_does_not_redeliver() -> Result<()> {
    let (mut client, mut peer) = pair().await?;

    for n in 1..=3 {
        peer.ping(n)?;
    }

    peer.flush().await?;

    assert_eq!(expect_ping(client.next().await?), 1);
    client.flush().await?;
    assert_eq!(expect_ping(client.next().await?), 2);

    // A flush with something to write drives I/O, and may receive the next
    // message while doing so.
    client.emit(PATH, INTERFACE, "Pong", &Arguments::empty())?;
    client.flush().await?;
    assert_eq!(expect_ping(client.next().await?), 3);
    client.flush().await?;

    peer.ping(4)?;
    peer.flush().await?;
    assert_eq!(expect_ping(client.next().await?), 4);
    assert_eq!(client.queued(), 0);

    let pong = peer.recv().await?;
    assert!(matches!(
        pong.kind(),
        MessageKind::Signal { member: "Pong", .. }
    ));
    Ok(())
}

#[tokio::test]
async fn stale_replies_are_not_queued() -> Result<()> {
    let (mut client, mut peer) = pair().await?;

    let call = tokio::spawn(async move {
        client
            .call(":1.0", PATH, INTERFACE, "Echo", &Arguments::empty())
            .await?;
        Ok::<_, crate::Error>(client)
    });

    let echo = peer.recv().await?;
    assert!(matches!(
        echo.kind(),
        MessageKind::MethodCall { member: "Echo", .. }
    ));

    // A second reply to `Hello`, which nothing is waiting for.
    let hello = peer.hello.clone();
    peer.reply(&hello, &BodyBuf::new())?;
    peer.ping(1)?;
    peer.reply(&echo, &BodyBuf::new())?;
    peer.flush().await?;

    let mut client = call.await.unwrap()?;
    assert_eq!(client.queued(), 1);
    assert_eq!(expect_ping(client.next().await?), 1);
    Ok(())
}

/// Arguments holding a value which cannot be encoded are not sent.
#[tokio::test]
async fn invalid_arguments_are_not_sent() -> Result<()> {
    use tokio_dbus::Signature;

    use crate::Value;

    let (mut client, mut peer) = pair().await?;

    let mut arguments = Arguments::new(Signature::VARIANT)?;
    arguments.store(Value::Struct(vec![]));

    let error = client
        .call(":1.0", PATH, INTERFACE, "Echo", &arguments)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Arguments hold a value which cannot be encoded"
    );

    assert!(client.emit(PATH, INTERFACE, "Bad", &arguments).is_err());
    client.emit(PATH, INTERFACE, "Pong", &Arguments::empty())?;
    client.flush().await?;

    let pong = peer.recv().await?;
    assert!(matches!(
        pong.kind(),
        MessageKind::Signal { member: "Pong", .. }
    ));
    Ok(())
}
