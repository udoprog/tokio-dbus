pub use self::send_buf::SendBuf;
mod send_buf;

pub use self::recv_buf::RecvBuf;
mod recv_buf;

use std::io;
use std::num::{NonZeroU32, NonZeroUsize};

use tokio::io::unix::AsyncFd;
use tokio::io::{Interest, Ready};

use crate::connection::{sasl_recv, MessageRef};
use crate::error::Result;
use crate::sasl::{SaslRequest, SaslResponse};
use crate::{ClientBuilder, Connection, Error, MessageKind};

/// Well known interface name.
pub(crate) const ORG_FREEDESKTOP_DBUS: &'static str = "org.freedesktop.DBus";

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
    /// The name of the client.
    name: Option<Box<str>>,
}

impl Client {
    /// Shorthand for connecting the client to the system bus using the default
    /// configuration.
    #[inline]
    pub async fn session_bus(send: &mut SendBuf, recv: &mut RecvBuf) -> Result<Self> {
        ClientBuilder::new().session_bus().connect(send, recv).await
    }

    /// Shorthand for connecting the client to the system bus using the default
    /// configuration.
    #[inline]
    pub async fn system_bus(send: &mut SendBuf, recv: &mut RecvBuf) -> Result<Self> {
        ClientBuilder::new().system_bus().connect(send, recv).await
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
    /// let mut send = SendBuf::new();
    /// let mut recv = RecvBuf::new();
    ///
    /// let mut c = Client::session_bus(&mut send, &mut recv).await?;
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello")
    ///     .with_destination("org.freedesktop.DBus");
    ///
    /// let serial = send.write_message(&m)?;
    ///
    /// let message = c.process(&mut send, &mut recv).await?;
    /// let message = recv.message(&message)?;
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
    pub async fn process(
        &mut self,
        send: &mut SendBuf,
        recv: &mut RecvBuf,
    ) -> Result<MessageRef, Error> {
        loop {
            if let Some(advance) = recv.advance.take() {
                recv.buf.advance(advance.get());
                recv.buf.update_alignment_base();
            }

            let message_ref = self.io(send, recv).await?;
            recv.advance = NonZeroUsize::new(message_ref.total);

            // Read once for internal processing. Avoid this once borrow checker
            // allows returning a reference here directly.
            let message = recv.message(&message_ref)?;

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

            if let Some(ORG_FREEDESKTOP_DBUS) = message.interface {
                // TODO: Ignore for now, but eventually we might want to handle
                // internally.
                continue;
            }

            return Ok(message_ref);
        }
    }

    /// Set client state.
    pub(crate) fn set_state(&mut self, state: ClientState) {
        self.state = state;
    }

    /// Construct a new asynchronous D-Bus client.
    pub(crate) fn new(connection: Connection) -> io::Result<Self> {
        connection.set_nonblocking(true)?;

        Ok(Self {
            connection: AsyncFd::new(connection)?,
            state: ClientState::Init,
            name: None,
        })
    }

    /// Send a SASL message and receive a response.
    pub(crate) async fn sasl_request<'buf>(
        &mut self,
        send: &mut SendBuf,
        recv: &'buf mut RecvBuf,
        sasl: &SaslRequest<'_>,
    ) -> Result<SaslResponse<'buf>> {
        loop {
            let mut guard = self.connection.writable_mut().await?;

            match guard.get_inner_mut().sasl_send(&mut send.buf, sasl) {
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

            match guard.get_inner_mut().sasl_recv(&mut recv.buf) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(len) => {
                    return sasl_recv(recv.buf.read_buf(len).get());
                }
            }
        }
    }

    /// Send the SASL `BEGIN` message.
    ///
    /// This does not expect a response from the server, instead it is expected
    /// to transition into the binary D-Bus protocol.
    pub(crate) async fn sasl_begin(&mut self, send: &mut SendBuf) -> Result<()> {
        loop {
            let mut guard = self.connection.writable_mut().await?;

            match guard.get_inner_mut().sasl_begin(&mut send.buf) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(()) => return Ok(()),
            }
        }
    }

    async fn io(&mut self, send: &mut SendBuf, recv: &mut RecvBuf) -> Result<MessageRef, Error> {
        loop {
            let mut interest = Interest::READABLE;

            if !send.buf.is_empty() {
                interest |= Interest::WRITABLE;
            }

            let mut guard = self.connection.ready_mut(interest).await?;

            if guard.ready().is_readable() {
                match guard.get_inner_mut().recv_message(&mut recv.buf) {
                    Ok(params) => {
                        return Ok(params);
                    }
                    Err(e) if e.would_block() => {
                        guard.clear_ready_matching(Ready::READABLE);
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }

            if guard.ready().is_writable() {
                match guard.get_inner().send_buf(&mut send.buf) {
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
