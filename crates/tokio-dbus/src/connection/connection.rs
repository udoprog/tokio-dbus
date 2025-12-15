use std::io;

use tokio::io::unix::AsyncFd;
use tokio::io::{Interest, Ready};

use crate::Buffers;
use crate::error::Result;
use crate::sasl::{SaslRequest, SaslResponse};

use super::{ConnectionBuilder, Transport, sasl_recv};

/// An asynchronous D-Bus client.
pub struct Connection {
    /// Poller for the underlying file descriptor.
    transport: AsyncFd<Transport>,
}

impl Connection {
    /// Construct a new asynchronous D-Bus client.
    pub(crate) fn new(transport: Transport) -> io::Result<Self> {
        transport.set_nonblocking(true)?;

        Ok(Self {
            transport: AsyncFd::new(transport)?,
        })
    }

    /// Shorthand for connecting the client to the system bus using the default
    /// configuration.
    #[inline]
    pub async fn session_bus(buf: &mut Buffers) -> Result<Self> {
        ConnectionBuilder::new().session_bus().connect(buf).await
    }

    /// Shorthand for connecting the client to the system bus using the default
    /// configuration.
    #[inline]
    pub async fn system_bus(buf: &mut Buffers) -> Result<Self> {
        ConnectionBuilder::new().system_bus().connect(buf).await
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
    /// let mut buf = Buffers::new();
    /// let mut c = Connection::session_bus(&mut buf).await?;
    /// c.wait(&mut buf).await?;
    /// let message: Message<'_> = buf.recv.last_message()?;
    /// # Ok(()) }
    /// ```
    pub async fn wait(&mut self, buf: &mut Buffers) -> Result<()> {
        self.io(buf).await
    }

    /// Send a SASL message and receive a response.
    pub(crate) async fn sasl_request<'buf>(
        &mut self,
        buf: &'buf mut Buffers,
        sasl: &SaslRequest<'_>,
    ) -> Result<SaslResponse<'buf>> {
        loop {
            let mut guard = self.transport.writable_mut().await?;

            match guard.get_inner_mut().sasl_send(buf.send.buf_mut(), sasl) {
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

            match guard.get_inner_mut().sasl_recv(buf.send.buf_mut()) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(len) => {
                    return sasl_recv(buf.send.buf_mut().read_until(len));
                }
            }
        }
    }

    /// Send the SASL `BEGIN` message.
    ///
    /// This does not expect a response from the server, instead it is expected
    /// to transition into the binary D-Bus protocol.
    pub(crate) async fn sasl_begin(&mut self, buf: &mut Buffers) -> Result<()> {
        loop {
            let mut guard = self.transport.writable_mut().await?;

            match guard.get_inner_mut().sasl_begin(buf.send.buf_mut()) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(()) => return Ok(()),
            }
        }
    }

    async fn io(&mut self, buf: &mut Buffers) -> Result<()> {
        loop {
            let mut interest = Interest::READABLE;

            if !buf.send.buf().is_empty() {
                interest |= Interest::WRITABLE;
            }

            let mut guard = self.transport.ready_mut(interest).await?;

            if guard.ready().is_readable() {
                match guard.get_inner_mut().recv_message(&mut buf.recv) {
                    Ok(()) => {
                        return Ok(());
                    }
                    Err(e) if e.would_block() => {
                        guard.clear_ready_matching(Ready::READABLE);
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }

            if guard.ready().is_writable() {
                match guard.get_inner().send_buf(buf.send.buf_mut()) {
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
