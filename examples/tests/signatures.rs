//! Generated code checks the signature of every body it decodes against the
//! interface, so that arguments of the wrong type are rejected rather than
//! reinterpreted.

use std::os::unix::net::UnixStream as StdUnixStream;

use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
use tokio_dbus::org_freedesktop_dbus;
use tokio_dbus::{
    BodyBuf, Buffers, ConnectionBuilder, MessageBuf, MessageKind, ObjectPath, Serial,
};
use tokio_dbus_runtime::{Connection, Incoming, Result};

include!(concat!(env!("OUT_DIR"), "/checked.rs"));

use self::checked::{CheckedServer, Signal};

const PATH: &ObjectPath = ObjectPath::new_const(b"/se/tedro/Checked");
const BEGIN: &[u8] = b"\0BEGIN\r\n";

/// The far end of a [`pair()`], scripted through the low level API.
struct Peer {
    connection: tokio_dbus::Connection,
    buf: Buffers,
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

    fn call(&mut self, interface: &str, member: &str, body: &BodyBuf) -> Result<Serial> {
        let m = self
            .buf
            .send
            .method_call(PATH, member)
            .with_interface(interface)
            .with_body(body);

        let serial = m.serial();
        self.buf.send.write_message(m)?;
        Ok(serial)
    }

    fn signal(&mut self, member: &str, body: &BodyBuf) -> Result<()> {
        let m = self
            .buf
            .send
            .signal(PATH, member)
            .with_interface(checked::INTERFACE)
            .with_body(body);

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

    let mut peer = Peer { connection, buf };
    let hello = peer.recv().await?;

    let mut body = BodyBuf::new();
    body.store(":1.1")?;
    peer.reply(&hello, &body)?;
    peer.flush().await?;

    let client = client.await.unwrap()?;
    Ok((client, peer))
}

#[derive(Default)]
struct Handler {
    calls: usize,
    label: String,
}

impl CheckedServer for Handler {
    async fn add(&mut self, a: i32, b: i32) -> Result<i32> {
        self.calls += 1;
        Ok(a + b)
    }

    async fn poke(&mut self) -> Result<()> {
        self.calls += 1;
        Ok(())
    }

    async fn label(&mut self) -> Result<String> {
        Ok(self.label.clone())
    }

    async fn set_label(&mut self, value: String) -> Result<()> {
        self.calls += 1;
        self.label = value;
        Ok(())
    }
}

/// Have the peer make a call, dispatch it on the connection and return what
/// the peer got back.
async fn roundtrip(
    client: &mut Connection,
    peer: &mut Peer,
    handler: &mut Handler,
    interface: &str,
    member: &str,
    body: &BodyBuf,
) -> Result<MessageBuf> {
    let serial = peer.call(interface, member, body)?;
    peer.flush().await?;

    let Incoming::Call(call) = client.next().await? else {
        panic!("Expected a call");
    };

    assert!(checked::dispatch(handler, client, &call).await?);
    client.flush().await?;

    let reply = peer.recv().await?;

    let reply_serial = match reply.kind() {
        MessageKind::MethodReturn { reply_serial } => reply_serial,
        MessageKind::Error { reply_serial, .. } => reply_serial,
        kind => panic!("Expected a reply, got {kind:?}"),
    };

    assert_eq!(reply_serial, serial);
    Ok(reply)
}

#[track_caller]
fn assert_invalid_args(reply: &MessageBuf) {
    match reply.kind() {
        MessageKind::Error { error_name, .. } => {
            assert_eq!(error_name, org_freedesktop_dbus::INVALID_ARGS_ERROR);
        }
        kind => panic!("Expected an error reply, got {kind:?}"),
    }
}

fn body(build: impl FnOnce(&mut BodyBuf) -> tokio_dbus::Result<()>) -> BodyBuf {
    let mut body = BodyBuf::new();
    build(&mut body).unwrap();
    body
}

#[tokio::test]
async fn server_rejects_wrong_arguments() -> Result<()> {
    let (mut client, mut peer) = pair().await?;
    let mut handler = Handler::default();
    let interface = checked::INTERFACE;

    // `u` where `i` is declared has the same size, and would otherwise be
    // reinterpreted.
    let wrong = body(|b| {
        b.store(1u32)?;
        b.store(2i32)
    });

    let reply = roundtrip(
        &mut client,
        &mut peer,
        &mut handler,
        interface,
        "Add",
        &wrong,
    )
    .await?;
    assert_invalid_args(&reply);

    let trailing = body(|b| {
        b.store(1i32)?;
        b.store(2i32)?;
        b.store(3i32)
    });

    let reply = roundtrip(
        &mut client,
        &mut peer,
        &mut handler,
        interface,
        "Add",
        &trailing,
    )
    .await?;
    assert_invalid_args(&reply);

    let reply = roundtrip(
        &mut client,
        &mut peer,
        &mut handler,
        interface,
        "Poke",
        &body(|b| b.store(1u32)),
    )
    .await?;
    assert_invalid_args(&reply);

    // A `Set` whose value is not wrapped in a variant.
    let set = body(|b| {
        b.store(checked::INTERFACE)?;
        b.store("Label")?;
        b.store("hello")
    });

    let properties = org_freedesktop_dbus::PROPERTIES_INTERFACE;
    let reply = roundtrip(
        &mut client,
        &mut peer,
        &mut handler,
        properties,
        "Set",
        &set,
    )
    .await?;
    assert_invalid_args(&reply);

    assert_eq!(handler.calls, 0);
    assert!(handler.label.is_empty());

    let right = body(|b| {
        b.store(1i32)?;
        b.store(2i32)
    });

    let reply = roundtrip(
        &mut client,
        &mut peer,
        &mut handler,
        interface,
        "Add",
        &right,
    )
    .await?;
    assert!(matches!(reply.kind(), MessageKind::MethodReturn { .. }));
    assert_eq!(reply.body().signature(), "i");
    assert_eq!(reply.body().load::<i32>()?, 3);
    assert_eq!(handler.calls, 1);
    Ok(())
}

#[tokio::test]
async fn client_rejects_wrong_reply() -> Result<()> {
    let (mut client, mut peer) = pair().await?;
    let proxy = checked::Checked::new(":1.0", PATH);

    let call = tokio::spawn(async move { proxy.add(&mut client, 1, 2).await });

    let add = peer.recv().await?;
    assert_eq!(add.body().signature(), "ii");
    peer.reply(&add, &body(|b| b.store(3u32)))?;
    peer.flush().await?;

    let error = call.await.unwrap().unwrap_err();
    assert!(!error.is_remote());
    assert_eq!(error.name(), Some(org_freedesktop_dbus::INVALID_ARGS_ERROR));
    Ok(())
}

#[tokio::test]
async fn signal_rejects_wrong_arguments() -> Result<()> {
    let (mut client, mut peer) = pair().await?;

    peer.signal("Changed", &body(|b| b.store(7u32)))?;
    peer.signal("Changed", &body(|b| b.store(7i32)))?;
    peer.flush().await?;

    let Incoming::Signal(wrong) = client.next().await? else {
        panic!("Expected a signal");
    };

    assert!(Signal::decode(&wrong).is_err());

    let Incoming::Signal(right) = client.next().await? else {
        panic!("Expected a signal");
    };

    assert_eq!(Signal::decode(&right)?, Some(Signal::Changed { value: 7 }));
    Ok(())
}
