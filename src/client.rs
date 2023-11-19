use std::io;
use std::num::NonZeroU32;

use tokio::io::unix::AsyncFd;
use tokio::io::{Interest, Ready};

use crate::connection::{read_message, sasl_recv};
use crate::error::Result;
use crate::sasl::{SaslRequest, SaslResponse};
use crate::{ClientBuilder, Connection, Error, Message, OwnedBuf};

/// An asynchronous D-Bus client.
pub struct Client {
    /// Poller for the underlying file descriptor.
    connection: AsyncFd<Connection>,
    /// Buffer used for sending data.
    send: OwnedBuf,
    /// Buffer used for receiving data.
    recv: OwnedBuf,
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

    /// Construct a new asynchronous D-Bus client.
    pub(crate) fn new(connection: Connection) -> io::Result<Self> {
        connection.set_nonblocking(true)?;

        Ok(Self {
            connection: AsyncFd::new(connection)?,
            send: OwnedBuf::new(),
            recv: OwnedBuf::new(),
        })
    }

    /// Send a SASL message and receive a response.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::{Client, Connection};
    /// use tokio_dbus::sasl::{Auth, SaslRequest};
    ///
    /// # #[tokio::main] async fn main() -> Result<(), tokio_dbus::Error> {
    /// let mut c = Client::session_bus().await?;
    /// let sasl = c.sasl_request(&SaslRequest::Auth(Auth::External(b"31303030"))).await?;
    /// # Ok(()) }
    /// ```
    pub async fn sasl_request(&mut self, sasl: &SaslRequest<'_>) -> Result<SaslResponse<'_>> {
        loop {
            let mut guard = self.connection.writable_mut().await?;

            match guard.get_inner_mut().sasl_send(&mut self.send, sasl) {
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

            match guard.get_inner_mut().sasl_recv(&mut self.recv) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(len) => {
                    return sasl_recv(self.recv.read_buf(len).get());
                }
            }
        }
    }

    /// Send the SASL `BEGIN` message.
    ///
    /// This does not expect a response from the server, instead it is expected
    /// to transition into the binary D-Bus protocol.
    pub async fn sasl_begin(&mut self) -> Result<()> {
        loop {
            let mut guard = self.connection.writable_mut().await?;

            match guard.get_inner_mut().sasl_begin(&mut self.send) {
                Err(e) if e.would_block() => {
                    guard.clear_ready();
                    continue;
                }
                Err(e) => return Err(e),
                Ok(()) => return Ok(()),
            }
        }
    }

    /// Write a `message` to the specified buffer and return the serial number
    /// associated with it.
    ///
    /// This can be used to add a message to the internal buffer immediately
    /// without sending it.
    ///
    /// To subsequently send the message you can use [`send_buf()`].
    ///
    /// [`send_buf()`]: Self::send_buf
    ///
    /// # Errors
    ///
    /// This only errors if the connection is not in a state yet to buffer
    /// messages, such as before authentication.
    pub fn write_message(&mut self, message: &Message<'_>) -> Result<NonZeroU32, Error> {
        self.send.update_alignment_base();

        self.connection
            .get_mut()
            .write_message(&mut self.send, message)
    }

    /// Process the current connection.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::sasl::{Auth, SaslRequest, SaslResponse};
    /// use tokio_dbus::{Client, Connection, Message, MessageKind, Result};
    ///
    /// # #[tokio::main] async fn main() -> Result<()> {
    /// let mut c = Client::session_bus().await?;
    ///
    /// let m = Message::method_call("/org/freedesktop/DBus", "Hello")
    ///     .with_destination("org.freedesktop.DBus");
    ///
    /// let serial = c.write_message(&m)?;
    ///
    /// let message = c.process().await?;
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
    pub async fn process(&mut self) -> Result<Message<'_>, Error> {
        loop {
            let mut interest = Interest::READABLE;

            if !self.send.is_empty() {
                interest |= Interest::WRITABLE;
            }

            let mut guard = self.connection.ready_mut(interest).await?;

            if guard.ready().is_readable() {
                match guard.get_inner_mut().recv_message(&mut self.recv) {
                    Ok((header, headers, total)) => {
                        return read_message(self.recv.read_buf(total), header, headers)
                    }
                    Err(e) if e.would_block() => {
                        guard.clear_ready_matching(Ready::READABLE);
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            }

            if guard.ready().is_writable() {
                match guard.get_inner_mut().send_buf(&mut self.send) {
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
