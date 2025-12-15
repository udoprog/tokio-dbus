use core::fmt;

use std::io;

use tokio::io::unix::AsyncFd;
use tokio::io::{Interest, Ready};

use crate::connection::builder::AuthKind;
use crate::error::{Error, ErrorKind, Result};
use crate::lossy_str::LossyStr;
use crate::sasl::Auth;
use crate::{Buffers, SendBuf};

use super::{ConnectionBuilder, Transport};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Sasl {
    /// The stage to realize.
    Stage(bool, SaslStage),
    /// Sending data.
    Send(SaslStage),
    /// Receiving data.
    Recv(SaslStage),
}

impl fmt::Display for Sasl {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sasl::Stage(_, stage) => write!(f, "sasl-{stage}"),
            Sasl::Send(stage) => write!(f, "sasl-send-{stage}"),
            Sasl::Recv(stage) => write!(f, "sasl-recv-{stage}"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SaslStage {
    Auth,
    Begin,
}

impl fmt::Display for SaslStage {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaslStage::Auth => write!(f, "auth"),
            SaslStage::Begin => write!(f, "begin"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum ConnectionState {
    /// SASL negotiation.
    Sasl(Sasl),
    /// Connection is idle.
    Idle,
    /// Body is being received.
    Message(usize),
}

impl ConnectionState {
    /// Test if connection is in a state where it is interested in writing.
    #[inline]
    fn is_writing(&self) -> bool {
        matches!(
            self,
            Self::Sasl(Sasl::Send(..)) | Self::Message(_) | Self::Idle
        )
    }
}

/// An asynchronous D-Bus client.
pub struct Connection {
    state: ConnectionState,
    /// Poller for the underlying file descriptor.
    transport: AsyncFd<Transport>,
}

impl Connection {
    /// Construct a new asynchronous D-Bus client.
    pub(crate) fn new(auth: AuthKind, transport: Transport) -> io::Result<Self> {
        transport.set_nonblocking(true)?;

        Ok(Self {
            state: match auth {
                AuthKind::Uid => ConnectionState::Sasl(Sasl::Stage(true, SaslStage::Auth)),
                AuthKind::None => ConnectionState::Sasl(Sasl::Stage(true, SaslStage::Begin)),
            },
            transport: AsyncFd::new(transport)?,
        })
    }

    /// Shorthand for connecting the client to the system bus using the default
    /// configuration.
    #[inline]
    pub fn session_bus() -> Result<Self> {
        ConnectionBuilder::new().session_bus().build()
    }

    /// Shorthand for connecting the client to the system bus using the default
    /// configuration.
    #[inline]
    pub fn system_bus() -> Result<Self> {
        ConnectionBuilder::new().system_bus().build()
    }

    /// Authenticate connection.
    #[cfg(feature = "libc")]
    fn sasl_auth_uid(&mut self, send: &mut SendBuf) -> Result<()> {
        let mut auth_buf = [0; 32];

        match Auth::external_from_uid(&mut auth_buf) {
            Auth::External(external) => {
                send.extend_from_slice(b"AUTH EXTERNAL ");
                send.extend_from_slice(external);
                send.extend_from_slice(b"\r\n");
            }
        }

        Ok(())
    }

    #[cfg(not(feature = "libc"))]
    fn sasl_auth_uid(&mut self, _: &mut SendBuf) -> Result<()> {
        Err(Error::new(ErrorKind::UnsupportedAuthUid))
    }

    fn sasl_begin(&mut self, send: &mut SendBuf) {
        send.extend_from_slice(b"BEGIN\r\n");
    }

    /// Test if the connection is fully established.
    pub fn is_connected(&self) -> bool {
        matches!(
            self.state,
            ConnectionState::Idle | ConnectionState::Message(_)
        )
    }

    /// Wait until the connection is fully established.
    ///
    /// This must be used before messages can be sent or received over this
    /// connection.
    pub async fn connect(&mut self, buf: &mut Buffers) -> Result<()> {
        if !self.is_connected() {
            // During the connection stage, the send buffer is used to
            // communicate in both directions. We clear it now to ensure there's
            // nothing unexpected on it.
            buf.send.buf_mut().clear();

            while !self.is_connected() {
                self.io(buf).await?;
            }
        }

        Ok(())
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
    /// use tokio_dbus::{Buffers, Connection, Message};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let mut c = Connection::session_bus()?;
    /// let mut buf = Buffers::new();
    /// c.connect(&mut buf).await?;
    /// c.wait(&mut buf).await?;
    /// let message: Message<'_> = buf.recv.last_message()?;
    /// # Ok(()) }
    /// ```
    pub async fn wait(&mut self, buf: &mut Buffers) -> Result<()> {
        buf.recv.clear();

        while !buf.recv.has_message() {
            self.io(buf).await?;
        }

        Ok(())
    }

    async fn io(&mut self, buf: &mut Buffers) -> Result<()> {
        if let ConnectionState::Sasl(Sasl::Stage(initial, stage)) = self.state {
            if initial {
                buf.send.extend_from_slice(b"\0");
            }

            match stage {
                SaslStage::Auth => {
                    self.sasl_auth_uid(&mut buf.send)?;
                    self.state = ConnectionState::Sasl(Sasl::Send(SaslStage::Auth));
                }
                SaslStage::Begin => {
                    self.sasl_begin(&mut buf.send);
                    self.state = ConnectionState::Sasl(Sasl::Send(SaslStage::Begin));
                }
            }
        }

        let mut interest = Interest::READABLE;

        if self.state.is_writing() && !buf.send.buf().is_empty() {
            interest |= Interest::WRITABLE;
        }

        let mut guard = self.transport.ready_mut(interest).await?;

        loop {
            if guard.ready().is_writable() {
                match guard.get_inner_mut().send_buf(buf.send.buf_mut()) {
                    Ok(()) => {
                        if let ConnectionState::Sasl(Sasl::Send(stage)) = self.state {
                            match stage {
                                SaslStage::Auth => {
                                    self.state = ConnectionState::Sasl(Sasl::Recv(stage));
                                }
                                // NB: We do not expect a response after we've
                                // sent BEGIN, but we *also* do not have a
                                // message yet.
                                SaslStage::Begin => {
                                    self.state = ConnectionState::Idle;
                                }
                            }
                        }

                        if buf.send.buf().is_empty() {
                            guard.clear_ready_matching(Ready::WRITABLE);
                        }
                    }
                    Err(e) if e.would_block() => {
                        guard.clear_ready_matching(Ready::WRITABLE);
                    }
                    Err(e) => return Err(e),
                }

                continue;
            }

            if guard.ready().is_readable() {
                match recv(self.state, guard.get_inner_mut(), buf) {
                    Ok(state) => {
                        self.state = state;

                        if matches!(self.state, ConnectionState::Idle) {
                            return Ok(());
                        }
                    }
                    Err(e) if e.would_block() => {
                        guard.clear_ready_matching(Ready::READABLE);
                    }
                    Err(e) => return Err(e),
                }

                continue;
            }

            return Ok(());
        }
    }
}

fn recv(
    state: ConnectionState,
    transport: &mut Transport,
    buf: &mut Buffers,
) -> Result<ConnectionState> {
    match state {
        ConnectionState::Sasl(sasl) => {
            // During the SASL negotiation stage, we use a single buffer for
            // sending and receiving.
            let io = buf.send.buf_mut();

            let n = transport.recv_line(io)?;

            let Some(bytes) = io.get().get(..n) else {
                return Err(Error::new(ErrorKind::InvalidSasl));
            };

            let state = match sasl {
                Sasl::Recv(state) => match state {
                    SaslStage::Auth => {
                        _ = ok_guid(bytes)?;
                        ConnectionState::Sasl(Sasl::Stage(false, SaslStage::Begin))
                    }
                    SaslStage::Begin => ConnectionState::Idle,
                },
                sasl => {
                    return Err(Error::new(ErrorKind::InvalidSaslState(sasl)));
                }
            };

            io.advance(n);
            Ok(state)
        }
        ConnectionState::Idle => {
            let total = transport.idle(&mut buf.recv)?;
            Ok(ConnectionState::Message(total))
        }
        ConnectionState::Message(total) => {
            transport.recv_body(&mut buf.recv, total)?;
            Ok(ConnectionState::Idle)
        }
    }
}

/// Parse an OK GUID.
pub(crate) fn ok_guid(bytes: &[u8]) -> Result<&LossyStr> {
    let line = crate::utils::trim_end(bytes);

    let Some((command, rest)) = crate::utils::split_once(line, b' ') else {
        return Err(Error::new(ErrorKind::InvalidSasl));
    };

    match command {
        b"OK" => Ok(LossyStr::new(rest)),
        _ => Err(Error::new(ErrorKind::InvalidSaslResponse)),
    }
}
