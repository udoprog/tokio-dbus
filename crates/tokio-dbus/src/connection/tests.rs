use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use crate::{BodyBuf, Buffers, ConnectionBuilder, MessageKind, ObjectPath, Result};

const BEGIN: &[u8] = b"\0BEGIN\r\n";

/// Connect a low level connection without authentication, returning it with
/// the raw other end of its socket after `BEGIN` has been read from it.
async fn connect() -> Result<(crate::Connection, Buffers, UnixStream)> {
    let (stream, mut raw) = UnixStream::pair()?;
    let mut c = ConnectionBuilder::new()
        .no_auth()
        .build_with_stream(stream)?;
    let mut buf = Buffers::new();
    c.connect(&mut buf).await?;

    let mut prefix = [0; BEGIN.len()];
    raw.read_exact(&mut prefix)?;
    assert_eq!(prefix, BEGIN);
    Ok((c, buf, raw))
}

#[tokio::test]
async fn partial_message_is_not_visible() -> Result<()> {
    // Encode a signal by sending it through a connection of our own.
    let (mut encoder, mut buf, mut raw) = connect().await?;
    let mut body = BodyBuf::new();
    body.store(42u32)?;
    let m = buf
        .send
        .signal(ObjectPath::new_const(b"/test"), "Ping")
        .with_body(&body);
    buf.send.write_message(m)?;
    encoder.flush(&mut buf).await?;

    let mut message = [0; 1024];
    let n = raw.read(&mut message)?;
    let message = &message[..n];

    let (mut c, mut buf, mut raw) = connect().await?;

    // The fixed header and part of the rest.
    raw.write_all(&message[..20])?;
    let wait = tokio::time::timeout(Duration::from_millis(100), c.wait(&mut buf)).await;
    assert!(wait.is_err());
    assert!(!buf.recv.has_message());

    raw.write_all(&message[20..])?;
    c.wait(&mut buf).await?;
    let message = buf.recv.last_message()?;
    assert!(matches!(
        message.kind(),
        MessageKind::Signal { member: "Ping", .. }
    ));
    assert_eq!(message.body().load::<u32>()?, 42);
    Ok(())
}
