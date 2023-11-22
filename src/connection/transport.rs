use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::io::{Read, Write};
use std::mem::size_of;
use std::os::fd::AsRawFd;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;

use crate::buf::{padding_to, AlignedBuf, UnalignedBuf, MAX_ARRAY_LENGTH, MAX_BODY_LENGTH};
use crate::error::{Error, ErrorKind, Result};
use crate::proto;
use crate::sasl::Auth;
use crate::sasl::{Guid, SaslRequest, SaslResponse};
use crate::{Frame, RecvBuf};

use super::MessageRef;

const ENV_SESSION_BUS: &str = "DBUS_SESSION_BUS_ADDRESS";
const ENV_SYSTEM_BUS: &str = "DBUS_SYSTEM_BUS_ADDRESS";
const DEFAULT_SYSTEM_BUS: &str = "unix:path=/var/run/dbus/system_bus_socket";

#[derive(Debug, Clone, Copy)]
pub(crate) enum SaslState {
    // SASL state before it's been initialized.
    Init,
    // SASL message being sent.
    Idle,
    // SASL message is being sent.
    Send,
}

impl fmt::Display for SaslState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaslState::Init => write!(f, "sasl-init"),
            SaslState::Idle => write!(f, "sasl-idle"),
            SaslState::Send => write!(f, "sasl-send"),
        }
    }
}

/// The state of the connection.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TransportState {
    // Newly opened socket in the SASL state.
    Sasl(SaslState),
    // Connection is open and idle.
    Idle,
    /// Body is being received.
    RecvBody(proto::Header, usize, usize),
}

impl fmt::Display for TransportState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransportState::Sasl(state) => write!(f, "sasl ({state})"),
            TransportState::Idle => write!(f, "idle"),
            TransportState::RecvBody(..) => write!(f, "recv-body"),
        }
    }
}

/// A connection to a d-bus session.
pub struct Transport {
    // Stream of the connection.
    stream: UnixStream,
    // The state of the connection.
    state: TransportState,
}

impl Transport {
    /// Construct a new connection to the session bus.
    ///
    /// This uses the `DBUS_SESSION_BUS_ADDRESS` environment variable to
    /// determine its address.
    pub fn session_bus() -> Result<Self> {
        Self::from_env(ENV_SESSION_BUS, None)
    }

    /// Construct a new connection to the session bus.
    ///
    /// This uses the `DBUS_SYSTEM_BUS_ADDRESS` environment variable to
    /// determine its address or fallback to the well-known address
    /// `unix:path=/var/run/dbus/system_bus_socket`.
    pub fn system_bus() -> Result<Self> {
        Self::from_env(ENV_SYSTEM_BUS, Some(DEFAULT_SYSTEM_BUS))
    }

    /// Construct a new connection to the session bus.
    ///
    /// This uses the `DBUS_SESSION_BUS_ADDRESS` environment variable to
    /// determine its address.
    fn from_env(env: &str, default: Option<&str>) -> Result<Self> {
        let value;

        let address: &OsStr = match env::var_os(env) {
            Some(address) => {
                value = address;
                value.as_os_str()
            }
            None => match default {
                Some(default) => default.as_ref(),
                None => return Err(Error::new(ErrorKind::MissingBus)),
            },
        };

        let stream = match parse_address(address)? {
            Address::Unix(address) => UnixStream::connect(OsStr::from_bytes(address))?,
        };

        Ok(Self::from_std(stream))
    }

    /// Set the connection as non-blocking.
    pub(crate) fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.stream.set_nonblocking(nonblocking)?;
        Ok(())
    }

    /// Constru.ct a connection directly from a unix stream.
    pub(crate) fn from_std(stream: UnixStream) -> Self {
        Self {
            stream,
            state: TransportState::Sasl(SaslState::Init),
        }
    }

    /// Send a SASL message and receive a response.
    pub(crate) fn sasl_send(
        &mut self,
        buf: &mut UnalignedBuf,
        request: &SaslRequest<'_>,
    ) -> Result<()> {
        loop {
            match &mut self.state {
                TransportState::Sasl(sasl) => match sasl {
                    SaslState::Init => {
                        buf.extend_from_slice(b"\0");
                        *sasl = SaslState::Idle;
                    }
                    SaslState::Idle => {
                        match request {
                            SaslRequest::Auth(auth) => match auth {
                                Auth::External(external) => {
                                    buf.extend_from_slice(b"AUTH EXTERNAL ");
                                    buf.extend_from_slice(external);
                                }
                            },
                        }

                        buf.extend_from_slice(b"\r\n");
                        *sasl = SaslState::Send;
                    }
                    SaslState::Send => {
                        send_buf(&mut &self.stream, buf)?;
                        *sasl = SaslState::Idle;
                        return Ok(());
                    }
                },
                state => return Err(Error::new(ErrorKind::InvalidState(*state))),
            }
        }
    }

    /// Receive a sasl response.
    pub(crate) fn sasl_recv(&mut self, buf: &mut AlignedBuf) -> Result<usize> {
        match self.state {
            TransportState::Sasl(SaslState::Idle) => {
                let value = recv_line(&mut &self.stream, buf)?;
                Ok(value)
            }
            state => Err(Error::new(ErrorKind::InvalidState(state))),
        }
    }

    /// Send the SASL `BEGIN` message.
    ///
    /// This does not expect a response from the server, instead it is expected
    /// to transition into the binary D-Bus protocol.
    pub(crate) fn sasl_begin(&mut self, buf: &mut UnalignedBuf) -> Result<()> {
        loop {
            match &mut self.state {
                TransportState::Sasl(sasl) => match sasl {
                    SaslState::Init => {
                        buf.extend_from_slice(b"\0");
                        *sasl = SaslState::Idle;
                    }
                    SaslState::Idle => {
                        buf.extend_from_slice(b"BEGIN\r\n");
                        *sasl = SaslState::Send;
                    }
                    SaslState::Send => {
                        send_buf(&mut &self.stream, buf)?;
                        self.state = TransportState::Idle;
                        return Ok(());
                    }
                },
                state => return Err(Error::new(ErrorKind::InvalidState(*state))),
            }
        }
    }

    /// Write and sned a single message over the connection.
    pub(crate) fn send_buf(&self, buf: &mut UnalignedBuf) -> Result<()> {
        send_buf(&mut &self.stream, buf)?;
        Ok(())
    }

    /// Receive a message.
    pub(crate) fn recv_message(&mut self, recv: &mut RecvBuf) -> Result<MessageRef> {
        loop {
            match self.state {
                TransportState::Idle => {
                    recv.clear();

                    self.recv_buf(
                        recv.buf_mut(),
                        size_of::<proto::Header>() + size_of::<u32>(),
                    )?;

                    let mut read_buf = recv
                        .buf_mut()
                        .read_until(size_of::<proto::Header>() + size_of::<u32>());

                    let mut header = read_buf.load::<proto::Header>()?;
                    let mut headers = read_buf.load::<u32>()?;

                    header.adjust(header.endianness);
                    headers.adjust(header.endianness);

                    if header.body_length > MAX_BODY_LENGTH {
                        return Err(Error::new(ErrorKind::BodyTooLong(header.body_length)));
                    }

                    if headers > MAX_ARRAY_LENGTH {
                        return Err(Error::new(ErrorKind::ArrayTooLong(headers)));
                    }

                    let Some(body_length) = usize::try_from(header.body_length).ok() else {
                        return Err(Error::new(ErrorKind::BodyTooLong(header.body_length)));
                    };

                    let Some(headers) = usize::try_from(headers).ok() else {
                        return Err(Error::new(ErrorKind::ArrayTooLong(headers)));
                    };

                    // Padding used in the header.
                    let total = headers + padding_to::<u64>(headers) + body_length;
                    self.state = TransportState::RecvBody(header, headers, total);
                }
                TransportState::RecvBody(header, headers, total) => {
                    self.recv_buf(recv.buf_mut(), total)?;
                    self.state = TransportState::Idle;

                    recv.set_last_message(header.serial);

                    return Ok(MessageRef { header, headers });
                }
                state => return Err(Error::new(ErrorKind::InvalidState(state))),
            }
        }
    }

    /// Receive exactly `n` bytes into the receive buffer.
    pub(crate) fn recv_buf(&mut self, buf: &mut AlignedBuf, n: usize) -> io::Result<()> {
        buf.reserve_bytes(n);

        let mut remaining = n;

        while remaining > 0 {
            let n = self.stream.read(&mut buf.get_mut()[..remaining])?;

            if n == 0 {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }

            buf.advance_mut(n);
            remaining -= n;
        }

        Ok(())
    }
}

impl Read for Transport {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for Transport {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

/// Receive a SASL message from the connection.
pub(crate) fn sasl_recv(bytes: &[u8]) -> Result<SaslResponse<'_>> {
    let line = crate::utils::trim_end(bytes);

    let Some((command, rest)) = crate::utils::split_once(line, b' ') else {
        return Err(Error::new(ErrorKind::InvalidSasl));
    };

    match command {
        b"OK" => Ok(SaslResponse::Ok(Guid::new(rest))),
        _ => Err(Error::new(ErrorKind::InvalidSaslResponse)),
    }
}

/// Send the given buffer over the connection.
fn send_buf(stream: &mut &UnixStream, buf: &mut UnalignedBuf) -> io::Result<()> {
    while !buf.is_empty() {
        let n = stream.write(buf.get())?;
        buf.advance(n);
    }

    stream.flush()?;
    Ok(())
}

fn recv_line(stream: &mut &UnixStream, buf: &mut AlignedBuf) -> io::Result<usize> {
    loop {
        if let Some(n) = buf.get().iter().position(|b| *b == b'\n') {
            return Ok(n + 1);
        }

        recv_some(stream, buf)?;
    }
}

/// Receive data into the specified buffer.
fn recv_some(stream: &mut &UnixStream, buf: &mut AlignedBuf) -> io::Result<()> {
    buf.reserve_bytes(4096);
    let n = stream.read(buf.get_mut())?;

    if n == 0 {
        return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
    }

    buf.advance_mut(n);
    Ok(())
}

enum Address<'a> {
    Unix(&'a [u8]),
}

#[cfg(unix)]
fn parse_address(string: &OsStr) -> Result<Address<'_>> {
    parse_address_bytes(string.as_bytes())
}

fn parse_address_bytes(bytes: &[u8]) -> Result<Address<'_>> {
    let Some(index) = bytes.iter().position(|&b| b == b'=') else {
        return Err(Error::new(ErrorKind::InvalidAddress));
    };

    let (head, tail) = bytes.split_at(index);

    match head {
        b"unix:path" => Ok(Address::Unix(&tail[1..])),
        _ => Err(Error::new(ErrorKind::InvalidAddress)),
    }
}

impl AsRawFd for Transport {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.stream.as_raw_fd()
    }
}
