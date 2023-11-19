use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::io::{Read, Write};
use std::mem;
use std::num::NonZeroU32;
use std::os::fd::AsRawFd;
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;

use crate::buf::padding_to;
use crate::error::{Error, ErrorKind, Result};
use crate::frame::Frame;
use crate::protocol;
use crate::protocol::Variant;
use crate::sasl::{Guid, SaslRequest, SaslResponse};
use crate::ReadBuf;
use crate::{Message, MessageKind, OwnedBuf, Signature};

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
pub(crate) enum ConnectionState {
    // Newly opened socket in the SASL state.
    Sasl(SaslState),
    // Connection is open and idle.
    Idle,
    /// Header fields are being received.
    RecvHeaderFields(protocol::Header),
    /// Body is being received.
    RecvBody(protocol::Header, usize, usize),
}

impl fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionState::Sasl(state) => write!(f, "sasl ({state})"),
            ConnectionState::Idle => write!(f, "idle"),
            ConnectionState::RecvHeaderFields(..) => write!(f, "recv-header-fields"),
            ConnectionState::RecvBody(..) => write!(f, "recv-body"),
        }
    }
}

/// A connection to a d-bus session.
pub struct Connection {
    // Stream of the connection.
    stream: UnixStream,
    // The state of the connection.
    state: ConnectionState,
    // Current serial used by the connection.
    serial: u32,
}

impl Connection {
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
            state: ConnectionState::Sasl(SaslState::Init),
            serial: 0,
        }
    }

    /// Send a SASL message and receive a response.
    pub(crate) fn sasl_send(
        &mut self,
        buf: &mut OwnedBuf,
        request: &SaslRequest<'_>,
    ) -> Result<()> {
        loop {
            match &mut self.state {
                ConnectionState::Sasl(sasl) => match sasl {
                    SaslState::Init => {
                        buf.extend_from_slice(b"\0");
                        *sasl = SaslState::Idle;
                    }
                    SaslState::Idle => {
                        buf.write(request);
                        buf.extend_from_slice(b"\r\n");
                        *sasl = SaslState::Send;
                    }
                    SaslState::Send => {
                        send_buf(&mut self.stream, buf)?;
                        *sasl = SaslState::Idle;
                        return Ok(());
                    }
                },
                state => return Err(Error::new(ErrorKind::InvalidState(*state))),
            }
        }
    }

    /// Receive a sasl response.
    pub(crate) fn sasl_recv(&mut self, buf: &mut OwnedBuf) -> Result<usize> {
        match self.state {
            ConnectionState::Sasl(SaslState::Idle) => {
                let value = recv_line(&mut self.stream, buf)?;
                Ok(value)
            }
            state => Err(Error::new(ErrorKind::InvalidState(state))),
        }
    }

    /// Send the SASL `BEGIN` message.
    ///
    /// This does not expect a response from the server, instead it is expected
    /// to transition into the binary D-Bus protocol.
    pub fn sasl_begin(&mut self, buf: &mut OwnedBuf) -> Result<()> {
        loop {
            match &mut self.state {
                ConnectionState::Sasl(sasl) => match sasl {
                    SaslState::Init => {
                        buf.extend_from_slice(b"\0");
                        *sasl = SaslState::Idle;
                    }
                    SaslState::Idle => {
                        buf.extend_from_slice(b"BEGIN\r\n");
                        *sasl = SaslState::Send;
                    }
                    SaslState::Send => {
                        send_buf(&mut self.stream, buf)?;
                        self.state = ConnectionState::Idle;
                        return Ok(());
                    }
                },
                state => return Err(Error::new(ErrorKind::InvalidState(*state))),
            }
        }
    }

    /// Write a `message` to the internal buffer and return the serial number
    /// associated with it.
    ///
    /// This can be used to add a message to the internal buffer immediately
    /// without sending it.
    ///
    /// To subsequently send the message you can use [`send_buf()`].
    ///
    /// [`send_buf()`]: Self::send_buf
    pub fn write_message(
        &mut self,
        buf: &mut OwnedBuf,
        message: &Message,
    ) -> Result<NonZeroU32, Error> {
        if matches!(self.state, ConnectionState::Sasl(..)) {
            return Err(Error::new(ErrorKind::InvalidState(self.state)));
        }

        let serial = if let Some(serial) = message.serial {
            serial.get()
        } else {
            self.serial += 1;

            if self.serial == 0 {
                self.serial = 1;
            }

            self.serial
        };

        buf.write(&protocol::Header {
            endianness: buf.endianness(),
            message_type: message.message_type(),
            flags: message.flags,
            version: 1,
            body_length: 0,
            serial,
        });

        // SAFETY: We've ensured just above that it's non-zero.
        let serial = unsafe { NonZeroU32::new_unchecked(self.serial) };

        let mut array = buf.write_array();

        match message.kind {
            MessageKind::MethodCall { path, member } => {
                let mut st = array.write_struct();
                st.write(&Variant::PATH);
                st.write(Signature::OBJECT_PATH);
                st.write(path);

                let mut st = array.write_struct();
                st.write(&Variant::MEMBER);
                st.write(Signature::STRING);
                st.write(member);
            }
            MessageKind::MethodReturn { reply_serial } => {
                let mut st = array.write_struct();
                st.write(&Variant::REPLY_SERIAL);
                st.write(Signature::UINT32);
                st.write(&reply_serial.get());
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } => {
                let mut st = array.write_struct();
                st.write(&Variant::ERROR_NAME);
                st.write(Signature::STRING);
                st.write(error_name);

                let mut st = array.write_struct();
                st.write(&Variant::REPLY_SERIAL);
                st.write(Signature::UINT32);
                st.write(&reply_serial.get());
            }
            MessageKind::Signal { member } => {
                let mut st = array.write_struct();
                st.write(&Variant::MEMBER);
                st.write(Signature::STRING);
                st.write(member);
            }
        }

        if let Some(interface) = message.interface {
            let mut st = array.write_struct();
            st.write(&Variant::INTERFACE);
            st.write(Signature::STRING);
            st.write(interface);
        }

        if let Some(destination) = message.destination {
            let mut st = array.write_struct();
            st.write(&Variant::DESTINATION);
            st.write(Signature::STRING);
            st.write(destination);
        }

        if let Some(sender) = message.sender {
            let mut st = array.write_struct();
            st.write(&Variant::SENDER);
            st.write(Signature::STRING);
            st.write(sender);
        }

        if !message.signature.is_empty() {
            let mut st = array.write_struct();
            st.write(&Variant::SIGNATURE);
            st.write(Signature::SIGNATURE);
            st.write(message.signature);
        }

        array.finish();
        buf.align_mut::<u64>();
        Ok(serial)
    }

    /// Write and sned a single message over the connection.
    pub(crate) fn send_buf(&mut self, buf: &mut OwnedBuf) -> Result<(), Error> {
        send_buf(&mut self.stream, buf)?;
        Ok(())
    }

    /// Receive a message.
    pub(crate) fn recv_message(
        &mut self,
        buf: &mut OwnedBuf,
    ) -> Result<(protocol::Header, usize, usize), Error> {
        loop {
            match self.state {
                ConnectionState::Idle => {
                    let mut header = *self.read_frame::<protocol::Header>(buf)?;
                    header.adjust(header.endianness);
                    buf.set_endianness(header.endianness);
                    self.state = ConnectionState::RecvHeaderFields(header);
                }
                ConnectionState::RecvHeaderFields(header) => {
                    let headers = usize::try_from(*self.read_frame::<u32>(buf)?)
                        .map_err(|_| ErrorKind::HeaderLengthTooLong)?;

                    let body_length = usize::try_from(header.body_length)
                        .map_err(|_| ErrorKind::BodyLengthTooLong)?;

                    let total = headers
                        .checked_add(padding_to::<u64>(headers))
                        .ok_or_else(|| ErrorKind::BodyLengthTooLong)?;

                    let total = total
                        .checked_add(body_length)
                        .ok_or_else(|| ErrorKind::BodyLengthTooLong)?;

                    self.state = ConnectionState::RecvBody(header, headers, total);
                }
                ConnectionState::RecvBody(header, headers, total) => {
                    self.recv_buf(buf, total)?;
                    self.state = ConnectionState::Idle;
                    return Ok((header, headers, total));
                }
                state => return Err(Error::new(ErrorKind::InvalidState(state))),
            }
        }
    }

    /// Read a header from the connection.
    ///
    /// This simply reads into the buffer pointed to by `header`, without
    /// validating the header.
    pub(crate) fn read_frame<'buf, T>(&mut self, buf: &'buf mut OwnedBuf) -> io::Result<&'buf T>
    where
        T: Frame,
    {
        buf.reserve_and_align::<T>();

        while buf.len() < mem::size_of::<T>() {
            recv_some(&mut self.stream, buf)?;
        }

        Ok(buf.load())
    }

    /// Fill a buffer up to `n` bytes.
    pub(crate) fn recv_buf(&mut self, buf: &mut OwnedBuf, n: usize) -> io::Result<()> {
        buf.reserve_bytes(n);

        while buf.len() < n {
            recv_some(&mut self.stream, buf)?;
        }

        Ok(())
    }
}

impl Read for Connection {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for Connection {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

/// Read a message out of the buffer.
pub(crate) fn read_message(
    mut buf: ReadBuf<'_>,
    header: protocol::Header,
    headers: usize,
) -> Result<Message<'_>> {
    let serial = NonZeroU32::new(header.serial).ok_or(ErrorKind::ZeroSerial)?;

    let mut path = None;
    let mut interface = None;
    let mut member = None;
    let mut error_name = None;
    let mut reply_serial = None;
    let mut destination = None;
    let mut signature = Signature::empty();
    let mut sender = None;

    let mut header_slice = buf.read_buf(headers);

    while !header_slice.is_empty() {
        header_slice.align::<u64>();
        let variant = *header_slice.read::<protocol::Variant>()?;
        let sig = header_slice.read::<Signature>()?;

        match (variant, sig.as_bytes()) {
            (protocol::Variant::PATH, b"s") => {
                path = Some(header_slice.read::<str>()?);
            }
            (protocol::Variant::INTERFACE, b"s") => {
                interface = Some(header_slice.read::<str>()?);
            }
            (protocol::Variant::MEMBER, b"s") => {
                member = Some(header_slice.read::<str>()?);
            }
            (protocol::Variant::ERROR_NAME, b"s") => {
                error_name = Some(header_slice.read::<str>()?);
            }
            (protocol::Variant::REPLY_SERIAL, b"u") => {
                let number = *header_slice.read::<u32>()?;
                let number = NonZeroU32::new(number).ok_or(ErrorKind::ZeroReplySerial)?;
                reply_serial = Some(number);
            }
            (protocol::Variant::DESTINATION, b"s") => {
                destination = Some(header_slice.read::<str>()?);
            }
            (protocol::Variant::SIGNATURE, b"g") => {
                signature = header_slice.read::<Signature>()?;
            }
            (protocol::Variant::SENDER, b"s") => {
                sender = Some(header_slice.read::<str>()?);
            }
            (variant, _) => return Err(Error::new(ErrorKind::InvalidHeaderVariant(variant))),
        }
    }

    let kind = match header.message_type {
        protocol::MessageType::METHOD_CALL => {
            let Some(path) = path else {
                return Err(Error::new(ErrorKind::MissingPath));
            };

            let Some(member) = member else {
                return Err(Error::new(ErrorKind::MissingMember));
            };

            MessageKind::MethodCall { path, member }
        }
        protocol::MessageType::METHOD_RETURN => {
            let Some(reply_serial) = reply_serial else {
                return Err(Error::new(ErrorKind::MissingReplySerial));
            };

            MessageKind::MethodReturn { reply_serial }
        }
        protocol::MessageType::ERROR => {
            let Some(error_name) = error_name else {
                return Err(Error::new(ErrorKind::MissingErrorName));
            };

            let Some(reply_serial) = reply_serial else {
                return Err(Error::new(ErrorKind::MissingReplySerial));
            };

            MessageKind::Error {
                error_name,
                reply_serial,
            }
        }
        protocol::MessageType::SIGNAL => {
            let Some(member) = member else {
                return Err(Error::new(ErrorKind::MissingMember));
            };

            MessageKind::Signal { member }
        }
        _ => return Err(Error::new(ErrorKind::InvalidProtocol)),
    };

    buf.align::<u64>();

    let body = buf.read_buf(header.body_length as usize);

    Ok(Message {
        kind,
        serial: Some(serial),
        flags: header.flags,
        interface,
        destination,
        sender,
        signature,
        body,
    })
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
fn send_buf(stream: &mut UnixStream, buf: &mut OwnedBuf) -> io::Result<()> {
    while !buf.is_empty() {
        let n = stream.write(buf.get())?;
        buf.advance(n);
    }

    stream.flush()?;
    Ok(())
}

fn recv_line(stream: &mut UnixStream, buf: &mut OwnedBuf) -> io::Result<usize> {
    loop {
        if let Some(n) = buf.get().iter().position(|b| *b == b'\n') {
            return Ok(n + 1);
        }

        recv_some(stream, buf)?;
    }
}

/// Receive data into the specified buffer.
fn recv_some(stream: &mut UnixStream, buf: &mut OwnedBuf) -> io::Result<()> {
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

impl AsRawFd for Connection {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.stream.as_raw_fd()
    }
}
