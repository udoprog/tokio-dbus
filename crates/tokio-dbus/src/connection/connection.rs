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

    /// Wait for the next incoming message on this connection.
    ///
    /// This is the main entry of this connection, and is required to call to
    /// have it make progress when passing D-Bus messages.
    ///
    /// If you just want to block while sending messages, use [`flush()`]
    /// instead.
    ///
    /// [`flush()`]: Self::flush
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::{Connection, Message};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let mut c = Connection::session_bus().await?;
    /// c.wait().await?;
    /// let message: Message<'_> = c.last_message()?;
    /// # Ok(()) }
    /// ```
    pub async fn wait(&mut self) -> Result<()> {
        // The receive buffer contains deferred messages, so we return one
        // of them.
        if self.recv.take_deferred() {
            return Ok(());
        }

        self.wait_no_deferred().await
    }

    /// Wait for the next incoming message on this connection ignoring messages
    /// that have been deferred through [`RecvBuf::defer`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::{Connection, Message};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let mut c = Connection::session_bus().await?;
    /// c.wait_no_deferred().await?;
    /// let message: Message<'_> = c.last_message()?;
    /// # Ok(()) }
    /// ```
    pub async fn wait_no_deferred(&mut self) -> Result<()> {
        loop {
            if !self.io(false).await? {
                continue;
            };

            if self.handle_internal()? {
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
    pub async fn flush(&mut self) -> Result<()> {
        while self.io(true).await? {
            self.handle_internal()?;
        }

        Ok(())
    }

    /// Handle internal messages, returns `true` if a message was intercepted.
    fn handle_internal(&mut self) -> Result<bool> {
        // Read once for internal processing. Avoid this once borrow checker
        // allows returning a reference here directly.
        let message = self.recv.last_message()?;

        if let ConnectionState::HelloSent(serial) = self.state {
            match message.kind {
                MessageKind::MethodReturn { reply_serial } if reply_serial == serial => {
                    self.name = Some(message.body().read::<str>()?.into());
                    self.state = ConnectionState::Idle;
                    return Ok(true);
                }
                _ => {}
            }
        }

        // TODO: Ignore freedesktop signals for now, but eventually we might
        // want to handle internally.
        if let (Some(org_freedesktop_dbus::INTERFACE), MessageKind::Signal { .. }) =
            (message.interface, message.kind)
        {
            return Ok(true);
        }

        Ok(false)
    }

    /// Construct a new [`Message`] corresponding to a method call.
    pub fn method_call<'a>(&mut self, path: &'a ObjectPath, member: &'a str) -> Message<'a> {
        self.send.method_call(path, member)
    }

    /// Write a message to the send buffer.
    ///
    /// This can be used to queue messages to be sent during the next call to
    /// [`wait()`]. To both receive and send in parallel, see the
    /// [`buffers()`] method.
    ///
    /// [`wait()`]: Self::wait
    /// [`buffers()`]: Self::buffers
    pub fn write_message(&mut self, message: Message<'_>) -> Result<()> {
        self.send.write_message(message)
    }

    /// Read the last message buffered.
    ///
    /// # Errors
    ///
    /// In case there is no message buffered.
    pub fn last_message(&self) -> Result<Message<'_>> {
        self.recv.last_message()
    }

    /// Access the underlying buffers of the connection.
    ///
    /// The [`RecvBuf`] instance is used to access messages received after a
    /// call to [`wait()`], through the [`RecvBuf::last_message()`].
    ///
    /// The returned [`BodyBuf`] is the internal buffer that the client uses to
    /// construct message bodies. It is empty when it's returned.
    ///
    /// [`wait()`]: Self::wait
    ///
    /// This is useful, because it permits using all parts of the connection
    /// without running into borrowing issues.
    ///
    /// For example, this wouldn't work:
    ///
    /// ```compile_fail
    /// use tokio_dbus::{Connection, Message};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let mut c = Connection::session_bus().await?;
    /// c.wait().await?;
    /// let message: Message<'_> = c.last_message()?;
    /// let m = message.method_return();
    /// c.write_message(m);
    /// # Ok(()) }    
    /// ```
    ///
    /// Because calling [`write_message()`] needs mutable access to the
    /// [`Connection`].
    ///
    /// [`write_message()`]: Self::write_message
    ///
    /// We can address this by using [`buffers()`]:
    ///
    /// [`buffers()`]: Self::buffers
    ///
    /// ```no_run
    /// use tokio_dbus::{Connection, Message};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let mut c = Connection::session_bus().await?;
    /// c.wait().await?;
    ///
    /// let (recv, send, body) = c.buffers();
    ///
    /// let message: Message<'_> = recv.last_message()?;
    /// let m = message.method_return(send.next_serial()).with_body(body);
    ///
    /// send.write_message(m);
    /// # Ok(()) }    
    /// ```
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

            match guard.get_inner_mut().sasl_recv(self.send.buf_mut()) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(len) => {
                    return sasl_recv(self.send.buf_mut().read_until(len));
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
        self.body.clear();
        self.body.store(name)?;
        self.body.store(flags)?;

        let m = self
            .send
            .method_call(org_freedesktop_dbus::PATH, "RequestName")
            .with_destination(org_freedesktop_dbus::DESTINATION)
            .with_body(&self.body);

        let serial = m.serial();
        self.send.write_message(m)?;

        loop {
            self.wait_no_deferred().await?;
            let message = self.recv.last_message_no_deferred()?;

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
                    self.recv.defer_last()?;
                }
            }
        }
    }

    async fn io(&mut self, flush: bool) -> Result<bool> {
        loop {
            let mut interest = Interest::READABLE;

            if !self.send.buf().is_empty() {
                interest |= Interest::WRITABLE;
            } else if flush {
                return Ok(false);
            }

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
