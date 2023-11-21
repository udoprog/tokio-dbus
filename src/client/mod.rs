pub use self::send_buf::SendBuf;
mod send_buf;

pub use self::recv_buf::RecvBuf;
mod recv_buf;

use std::io;
use std::num::{NonZeroU32, NonZeroUsize};

use tokio::io::unix::AsyncFd;
use tokio::io::{Interest, Ready};

use crate::connection::{sasl_recv, MessageRef};
use crate::error::{ErrorKind, Result};
use crate::org_freedesktop_dbus::{self, NameFlag, NameReply};
use crate::sasl::{SaslRequest, SaslResponse};
use crate::{BodyBuf, ClientBuilder, Connection, Error, Message, MessageKind};

/// The high level state of a client.
pub(crate) enum ClientState {
    /// Just initialized.
    Init,
    /// Sent the Hello() message and is awaiting response.
    HelloSent(NonZeroU32),
    /// Client is in a normal idle state.
    Idle,
}

/// An asynchronous D-Bus client.
pub struct Client {
    /// Poller for the underlying file descriptor.
    connection: AsyncFd<Connection>,
    /// Hello serial.
    state: ClientState,
    /// Receive buffer.
    recv: RecvBuf,
    /// Send buffer.
    send: SendBuf,
    /// Body buffer.
    body: BodyBuf,
    /// The name of the client.
    name: Option<Box<str>>,
}

impl Client {
    /// Shorthand for connecting the client to the system bus using the default
    /// configuration.
    #[inline]
    pub async fn session_bus() -> Result<Self> {
        ClientBuilder::new().session_bus().connect().await
    }

    /// Shorthand for connecting the client to the system bus using the default
    /// configuration.
    #[inline]
    pub async fn system_bus() -> Result<Self> {
        ClientBuilder::new().system_bus().connect().await
    }

    /// Process the current connection.
    ///
    /// This is the main entry of this connection, and is required to call to
    /// have it make progress when passing D-Bus messages.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::sasl::{Auth, SaslRequest, SaslResponse};
    /// use tokio_dbus::{Client, SendBuf, RecvBuf, Connection, Message, MessageKind, Result};
    ///
    /// # #[tokio::main] async fn main() -> Result<()> {
    /// let mut c = Client::session_bus().await?;
    ///
    /// let m = c.method_call("/org/freedesktop/DBus", "Hello")
    ///     .with_destination("org.freedesktop.DBus");
    ///
    /// let serial = m.serial();
    ///
    /// c.write_message(&m)?;
    ///
    /// let message = c.process().await?;
    /// let message = c.read_message(&message)?;
    ///
    /// assert_eq!(
    ///     message.kind(),
    ///     MessageKind::MethodReturn {
    ///         reply_serial: serial
    ///     }
    /// );
    ///
    /// let mut body = message.body();
    /// let name = body.read::<str>()?;
    /// dbg!(message, body.len(), name);
    /// # Ok(()) }    
    /// ```
    pub async fn process(&mut self) -> Result<MessageRef, Error> {
        loop {
            let message_ref = self.io().await?;
            self.recv.last_serial = NonZeroU32::new(message_ref.header.serial);

            // Read once for internal processing. Avoid this once borrow checker
            // allows returning a reference here directly.
            let message = self.recv.read_message(&message_ref)?;

            if let ClientState::HelloSent(serial) = self.state {
                match message.kind {
                    MessageKind::MethodReturn { reply_serial } if reply_serial == serial => {
                        self.name = Some(message.body().read::<str>()?.into());
                        self.state = ClientState::Idle;
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

            return Ok(message_ref);
        }
    }

    /// Construct a new [`Message`] corresponding to a method call.
    pub fn method_call<'a>(&mut self, path: &'a str, member: &'a str) -> Message<'a> {
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
    pub fn write_message(&mut self, message: &Message<'_>) -> Result<()> {
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
    pub fn read_message(&self, message_ref: &MessageRef) -> Result<Message<'_>> {
        self.recv.read_message(message_ref)
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

    /// Construct a new asynchronous D-Bus client.
    pub(crate) fn new(connection: Connection) -> io::Result<Self> {
        connection.set_nonblocking(true)?;

        Ok(Self {
            connection: AsyncFd::new(connection)?,
            state: ClientState::Init,
            recv: RecvBuf::new(),
            send: SendBuf::new(),
            body: BodyBuf::new(),
            name: None,
        })
    }

    /// Send a SASL message and receive a response.
    pub(crate) async fn sasl_request(
        &mut self,
        sasl: &SaslRequest<'_>,
    ) -> Result<SaslResponse<'_>> {
        loop {
            let mut guard = self.connection.writable_mut().await?;

            match guard.get_inner_mut().sasl_send(&mut self.send.buf, sasl) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(()) => break,
            }
        }

        loop {
            let mut guard = self.connection.readable_mut().await?;

            match guard.get_inner_mut().sasl_recv(&mut self.recv.buf) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(len) => {
                    return sasl_recv(self.recv.buf.read_buf(len).get());
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
            let mut guard = self.connection.writable_mut().await?;

            match guard.get_inner_mut().sasl_begin(&mut self.send.buf) {
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

        self.send.write_message(&m)?;
        self.state = ClientState::HelloSent(m.serial());
        Ok(())
    }

    /// Request the given well-known name.
    pub async fn request_name(&mut self, name: &str, flags: NameFlag) -> Result<NameReply> {
        self.body.write(name);
        self.body.store(flags);

        let m = self
            .send
            .method_call(org_freedesktop_dbus::PATH, "RequestName")
            .with_destination(org_freedesktop_dbus::DESTINATION)
            .with_body_buf(&self.body);

        self.send.write_message(&m)?;
        let serial = m.serial();

        self.body.clear();

        loop {
            let message_ref = self.process().await?;
            let message = self.recv.read_message(&message_ref)?;

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

    async fn io(&mut self) -> Result<MessageRef, Error> {
        loop {
            if let Some(advance) = self.recv.advance.take() {
                self.recv.buf.advance(advance.get());
                self.recv.buf.update_alignment_base();
            }

            let mut interest = Interest::READABLE;

            if !self.send.buf.is_empty() {
                interest |= Interest::WRITABLE;
            }

            let mut guard = self.connection.ready_mut(interest).await?;

            if guard.ready().is_readable() {
                match guard.get_inner_mut().recv_message(&mut self.recv.buf) {
                    Ok(message_ref) => {
                        self.recv.advance = NonZeroUsize::new(message_ref.total);
                        return Ok(message_ref);
                    }
                    Err(e) if e.would_block() => {
                        guard.clear_ready_matching(Ready::READABLE);
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }

            if guard.ready().is_writable() {
                match guard.get_inner().send_buf(&mut self.send.buf) {
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
