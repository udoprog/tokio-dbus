use std::io;
use std::num::NonZeroU32;

use tokio::io::unix::AsyncFd;
use tokio::io::{Interest, Ready};

use crate::error::{ErrorKind, Result};
use crate::org_freedesktop_dbus::{self, NameFlag, NameReply};
use crate::sasl::{SaslRequest, SaslResponse};
use crate::{BodyBuf, Error, Message, MessageKind, ObjectPath, RecvBuf, SendBuf};

use super::{sasl_recv, ConnectionBuilder, Transport};

/// The high level state of a client.
pub(crate) enum ConnectionState {
    /// Just initialized.
    Init,
    /// Sent the Hello() message and is awaiting response.
    HelloSent(NonZeroU32),
    /// Connection is in a normal idle state.
    Idle,
}

/// An asynchronous D-Bus client.
pub struct Connection {
    /// Poller for the underlying file descriptor.
    transport: AsyncFd<Transport>,
    /// Hello serial.
    state: ConnectionState,
    /// Receive buffer.
    recv: RecvBuf,
    /// Send buffer.
    send: SendBuf,
    /// Body buffer.
    body: BodyBuf,
    /// The name of the client.
    name: Option<Box<str>>,
}

impl Connection {
    /// Construct a new asynchronous D-Bus client.
    pub(crate) fn new(transport: Transport) -> io::Result<Self> {
        transport.set_nonblocking(true)?;

        Ok(Self {
            transport: AsyncFd::new(transport)?,
            state: ConnectionState::Init,
            recv: RecvBuf::new(),
            send: SendBuf::new(),
            body: BodyBuf::new(),
            name: None,
        })
    }

    /// Shorthand for connecting the client to the system bus using the default
    /// configuration.
    #[inline]
    pub async fn session_bus() -> Result<Self> {
        ConnectionBuilder::new().session_bus().connect().await
    }

    /// Shorthand for connecting the client to the system bus using the default
    /// configuration.
    #[inline]
    pub async fn system_bus() -> Result<Self> {
        ConnectionBuilder::new().system_bus().connect().await
    }

    /// Process the current connection.
    ///
    /// This is the main entry of this connection, and is required to call to
    /// have it make progress when passing D-Bus messages.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::{Connection, Message};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let mut c = Connection::session_bus().await?;
    ///
    /// c.process().await?;
    /// let message: Message<'_> = c.last_message()?;
    /// # Ok(()) }    
    /// ```
    pub async fn process(&mut self) -> Result<(), Error> {
        loop {
            if !self.io(false).await? {
                continue;
            };

            // Read once for internal processing. Avoid this once borrow checker
            // allows returning a reference here directly.
            let message = self.recv.last_message()?;

            if let ConnectionState::HelloSent(serial) = self.state {
                match message.kind {
                    MessageKind::MethodReturn { reply_serial } if reply_serial == serial => {
                        self.name = Some(message.body().read::<str>()?.into());
                        self.state = ConnectionState::Idle;
                        continue;
                    }
                    _ => {}
                }
            }

            // TODO: Ignore freedesktop signals for now, but eventually we might
            // want to handle internally.
            if let (Some(org_freedesktop_dbus::INTERFACE), MessageKind::Signal { .. }) =
                (message.interface, message.kind)
            {
                continue;
            }

            return Ok(());
        }
    }

    /// Flush all outgoing messages and return when the send buffer is empty.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::Connection;
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let mut c = Connection::session_bus().await?;
    /// c.flush().await?;
    /// # Ok(()) }
    /// ```
    pub async fn flush(&mut self) -> Result<(), Error> {
        while self.io(true).await? {}
        Ok(())
    }

    /// Construct a new [`Message`] corresponding to a method call.
    pub fn method_call<'a>(&mut self, path: &'a ObjectPath, member: &'a str) -> Message<'a> {
        self.send.method_call(path, member)
    }

    /// Write a message to the send buffer.
    ///
    /// This can be used to queue messages to be sent during the next call to
    /// [`process()`]. To both receive and send in parallel, see the
    /// [`buffers()`] method.
    ///
    /// [`process()`]: Self::process
    /// [`buffers()`]: Self::buffers
    pub fn write_message(&mut self, message: Message<'_>) -> Result<()> {
        self.send.write_message(message)
    }

    /// Read a [`MessageRef`] into a [`Message`].
    ///
    /// Note that if the [`MessageRef`] is outdated by calling process again,
    /// the behavior of this function is not well-defined (but safe).
    ///
    /// # Errors
    ///
    /// Errors if the message reference is out of date, such as if another
    /// message has been received.
    pub fn last_message(&self) -> Result<Message<'_>> {
        self.recv.last_message()
    }

    /// Access the underlying buffers of the connection.
    ///
    /// This is usually needed to solve lifetime issues, such as holding onto a
    /// message constructed from a [`MessageRef`] while buffering a response.
    ///
    /// The [`RecvBuf`] is used to translate [`MessageRef`] as returned by
    /// [`process()`] into [`Message`] instances and [`SendBuf`] is used to
    /// queue messages to be sent.
    ///
    /// The returned [`BodyBuf`] is the internal buffer that the client uses to
    /// construct message bodies. It is empty when it's returned.
    ///
    /// [`process()`]: Self::process
    pub fn buffers(&mut self) -> (&RecvBuf, &mut SendBuf, &mut BodyBuf) {
        self.body.clear();
        (&self.recv, &mut self.send, &mut self.body)
    }

    /// Send a SASL message and receive a response.
    pub(crate) async fn sasl_request(
        &mut self,
        sasl: &SaslRequest<'_>,
    ) -> Result<SaslResponse<'_>> {
        loop {
            let mut guard = self.transport.writable_mut().await?;

            match guard.get_inner_mut().sasl_send(self.send.buf_mut(), sasl) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(()) => break,
            }
        }

        loop {
            let mut guard = self.transport.readable_mut().await?;

            match guard.get_inner_mut().sasl_recv(self.recv.buf_mut()) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(len) => {
                    return sasl_recv(self.recv.buf_mut().read_until(len).get());
                }
            }
        }
    }

    /// Send the SASL `BEGIN` message.
    ///
    /// This does not expect a response from the server, instead it is expected
    /// to transition into the binary D-Bus protocol.
    pub(crate) async fn sasl_begin(&mut self) -> Result<()> {
        loop {
            let mut guard = self.transport.writable_mut().await?;

            match guard.get_inner_mut().sasl_begin(self.send.buf_mut()) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(()) => return Ok(()),
            }
        }
    }

    /// Send "Hello" message.
    pub(crate) fn hello(&mut self) -> Result<()> {
        let m = self
            .send
            .method_call(org_freedesktop_dbus::PATH, "Hello")
            .with_destination(org_freedesktop_dbus::DESTINATION);

        let serial = m.serial();
        self.send.write_message(m)?;
        self.state = ConnectionState::HelloSent(serial);
        Ok(())
    }

    /// Request the given well-known name.
    pub async fn request_name(&mut self, name: &str, flags: NameFlag) -> Result<NameReply> {
        self.body.write(name)?;
        self.body.store(flags)?;

        let m = self
            .send
            .method_call(org_freedesktop_dbus::PATH, "RequestName")
            .with_destination(org_freedesktop_dbus::DESTINATION)
            .with_body(&self.body);

        let serial = m.serial();
        self.send.write_message(m)?;
        self.body.clear();

        loop {
            self.process().await?;
            let message = self.recv.last_message()?;

            match message.kind {
                MessageKind::MethodReturn { reply_serial } if reply_serial == serial => {
                    let reply = message.body().load::<NameReply>()?;
                    return Ok(reply);
                }
                MessageKind::Error {
                    error_name,
                    reply_serial,
                } if reply_serial == serial => {
                    let message = message.body().read::<str>()?;

                    return Err(Error::new(ErrorKind::ResponseError(
                        error_name.into(),
                        message.into(),
                    )));
                }
                _ => {
                    // Ignore other messages
                }
            }
        }
    }

    async fn io(&mut self, flush: bool) -> Result<bool, Error> {
        loop {
            let interest = if !flush {
                let mut interest = Interest::READABLE;

                if !self.send.buf().is_empty() {
                    interest |= Interest::WRITABLE;
                }

                interest
            } else if self.send.buf().is_empty() {
                Interest::WRITABLE
            } else {
                return Ok(false);
            };

            let mut guard = self.transport.ready_mut(interest).await?;

            if guard.ready().is_readable() {
                match guard.get_inner_mut().recv_message(&mut self.recv) {
                    Ok(()) => {
                        return Ok(true);
                    }
                    Err(e) if e.would_block() => {
                        guard.clear_ready_matching(Ready::READABLE);
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }

            if guard.ready().is_writable() {
                match guard.get_inner().send_buf(self.send.buf_mut()) {
                    Ok(()) => {}
                    Err(e) if e.would_block() => {
                        guard.clear_ready_matching(Ready::WRITABLE);
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }
}
