use core::mem::size_of;
use core::num::NonZeroU32;

use alloc::vec::Vec;

use std::env;
use std::ffi::OsStr;
use std::io;
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;

use crate::buf::{AlignedBuf, MAX_ARRAY_LENGTH, MAX_BODY_LENGTH, UnalignedBuf, padding_to};
use crate::error::{Error, ErrorKind, Result};
use crate::proto;
use crate::recv_buf::MessageRef;
use crate::{Frame, RecvBuf, Serial};

const ENV_STARTER_ADDRESS: &str = "DBUS_STARTER_ADDRESS";
const ENV_SESSION_BUS: &str = "DBUS_SESSION_BUS_ADDRESS";
const ENV_SYSTEM_BUS: &str = "DBUS_SYSTEM_BUS_ADDRESS";
const DEFAULT_SYSTEM_BUS: &str = "unix:path=/var/run/dbus/system_bus_socket";
/// The major protocol version this implementation speaks.
const PROTOCOL_VERSION: u8 = 1;

/// A connection to a d-bus session.
pub struct Transport {
    // Stream of the connection.
    stream: UnixStream,
}

impl Transport {
    /// Construct a new connection to the session bus.
    ///
    /// This uses the `DBUS_SESSION_BUS_ADDRESS` environment variable to
    /// determine its address.
    pub fn session_bus() -> Result<Self> {
        Self::from_env([ENV_STARTER_ADDRESS, ENV_SESSION_BUS], None)
    }

    /// Construct a new connection to the system bus.
    ///
    /// This uses the `DBUS_SYSTEM_BUS_ADDRESS` environment variable to
    /// determine its address or fallback to the well-known address
    /// `unix:path=/var/run/dbus/system_bus_socket`.
    pub fn system_bus() -> Result<Self> {
        Self::from_env(
            [ENV_STARTER_ADDRESS, ENV_SYSTEM_BUS],
            Some(DEFAULT_SYSTEM_BUS),
        )
    }

    /// Construct a new connection from the first of the given environment
    /// variables which is set, falling back to `default` when none is.
    fn from_env(
        envs: impl IntoIterator<Item: AsRef<OsStr>>,
        default: Option<&str>,
    ) -> Result<Self> {
        let address_storage;

        let address = 'address: {
            for env in envs {
                let Some(address) = env::var_os(env) else {
                    continue;
                };

                address_storage = address;
                break 'address address_storage.as_os_str();
            }

            if let Some(address) = default {
                break 'address OsStr::new(address);
            }

            return Err(Error::new(ErrorKind::MissingBus));
        };

        Ok(Self::from_std(connect(address.as_bytes())?))
    }

    /// Set the connection as non-blocking.
    pub(crate) fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.stream.set_nonblocking(nonblocking)?;
        Ok(())
    }

    /// Construct a connection directly from a unix stream.
    pub(crate) fn from_std(stream: UnixStream) -> Self {
        Self { stream }
    }

    /// Receive a sasl response.
    pub(crate) fn recv_line(&mut self, buf: &mut UnalignedBuf) -> io::Result<usize> {
        loop {
            if let Some(n) = buf.get().iter().position(|b| *b == b'\n') {
                return Ok(n + 1);
            }

            buf.reserve_bytes(4096);
            let n = self.stream.read(buf.get_mut())?;

            if n == 0 {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }

            buf.advance_mut(n);
        }
    }

    /// Send the contents of the given buffer.
    pub(crate) fn send_buf(&mut self, buf: &mut UnalignedBuf) -> Result<()> {
        while !buf.is_empty() {
            let n = self.stream.write(buf.get())?;
            buf.advance(n);
        }

        self.stream.flush()?;
        Ok(())
    }

    pub(crate) fn idle(&mut self, recv: &mut RecvBuf) -> Result<usize> {
        self.recv_buf(
            recv.buf_mut(),
            size_of::<proto::Header>().wrapping_add(size_of::<u32>()),
        )?;

        let mut read_buf = recv.buf().as_aligned();

        let mut header = read_buf.load::<proto::Header>()?;

        if !matches!(
            header.endianness,
            proto::Endianness::LITTLE | proto::Endianness::BIG
        ) {
            return Err(Error::new(ErrorKind::InvalidEndianness(header.endianness)));
        }

        if header.version != PROTOCOL_VERSION {
            return Err(Error::new(ErrorKind::UnsupportedProtocolVersion(
                header.version,
            )));
        }

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

        let serial = Serial::new(NonZeroU32::new(header.serial).ok_or(ErrorKind::ZeroSerial)?);

        // Padding used in the header.
        let total = headers + padding_to::<u64>(headers) + body_length;

        let message_ref = MessageRef {
            serial,
            message_type: header.message_type,
            flags: header.flags,
            headers,
        };

        recv.set_endianness(header.endianness);
        recv.set_last_message(message_ref);
        Ok(total)
    }

    /// Receive the remaining body.
    pub(crate) fn recv_body(&mut self, recv: &mut RecvBuf, total: usize) -> Result<()> {
        // The fixed header received by `idle()` is still in the buffer, so the
        // target includes it.
        let n = size_of::<proto::Header>()
            .wrapping_add(size_of::<u32>())
            .wrapping_add(total);

        self.recv_buf(recv.buf_mut(), n)?;
        Ok(())
    }

    /// Receive bytes into the receive buffer until it holds `n`.
    ///
    /// Never reads beyond `n` bytes, since anything past that belongs to the
    /// next message and would be lost when the buffer is cleared.
    pub(crate) fn recv_buf(&mut self, buf: &mut AlignedBuf, n: usize) -> io::Result<()> {
        buf.reserve_bytes(n.saturating_sub(buf.len()));

        while buf.len() < n {
            let remaining = n - buf.len();
            let read = self.stream.read(&mut buf.get_mut()[..remaining])?;

            if read == 0 {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }

            buf.advance(read);
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

impl AsRawFd for Transport {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        self.stream.as_raw_fd()
    }
}

/// A connectable endpoint parsed from one entry of a server address.
#[derive(Debug, PartialEq, Eq)]
enum Endpoint {
    /// `unix:path=`.
    Path(Vec<u8>),
    /// `unix:abstract=`.
    Abstract(Vec<u8>),
}

/// Connect to the first usable entry of a `;`-separated address list.
///
/// Returns the last error encountered, or `InvalidAddress` if no entry was
/// usable at all.
fn connect(address: &[u8]) -> Result<UnixStream> {
    let mut last_error = None;

    for entry in parse_address_list(address) {
        let endpoint = match entry {
            Ok(Some(endpoint)) => endpoint,
            Ok(None) => continue,
            Err(error) => {
                last_error = Some(error);
                continue;
            }
        };

        match connect_endpoint(&endpoint) {
            Ok(stream) => return Ok(stream),
            Err(error) => last_error = Some(Error::from(error)),
        }
    }

    Err(last_error.unwrap_or_else(|| Error::new(ErrorKind::InvalidAddress)))
}

fn connect_endpoint(endpoint: &Endpoint) -> io::Result<UnixStream> {
    match endpoint {
        Endpoint::Path(path) => UnixStream::connect(OsStr::from_bytes(path)),
        #[cfg(any(target_os = "linux", target_os = "android"))]
        Endpoint::Abstract(name) => {
            #[cfg(target_os = "android")]
            use std::os::android::net::SocketAddrExt;
            #[cfg(target_os = "linux")]
            use std::os::linux::net::SocketAddrExt;
            use std::os::unix::net::SocketAddr;

            UnixStream::connect_addr(&SocketAddr::from_abstract_name(name)?)
        }
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        Endpoint::Abstract(..) => Err(io::Error::from(io::ErrorKind::Unsupported)),
    }
}

/// Parse each entry of a `;`-separated server address list.
///
/// An entry yields `Ok(None)` when it is well-formed but not usable by this
/// client, such as an unsupported transport.
fn parse_address_list(bytes: &[u8]) -> impl Iterator<Item = Result<Option<Endpoint>>> + '_ {
    bytes
        .split(|&b| b == b';')
        .filter(|entry| !entry.is_empty())
        .map(parse_address)
}

/// Parse a single server address of the form `transport:key=value,...`.
fn parse_address(bytes: &[u8]) -> Result<Option<Endpoint>> {
    let Some(index) = bytes.iter().position(|&b| b == b':') else {
        return Err(Error::new(ErrorKind::InvalidAddress));
    };

    let (transport, rest) = bytes.split_at(index);
    let rest = rest.get(1..).unwrap_or_default();

    if transport.is_empty() {
        return Err(Error::new(ErrorKind::InvalidAddress));
    }

    let mut path = None;
    let mut abstract_name = None;

    for pair in rest.split(|&b| b == b',').filter(|pair| !pair.is_empty()) {
        let Some(index) = pair.iter().position(|&b| b == b'=') else {
            return Err(Error::new(ErrorKind::InvalidAddress));
        };

        let (key, value) = pair.split_at(index);
        let value = unescape(value.get(1..).unwrap_or_default())?;

        if key.is_empty() {
            return Err(Error::new(ErrorKind::InvalidAddress));
        }

        match key {
            b"path" => path = Some(value),
            b"abstract" => abstract_name = Some(value),
            _ => {}
        }
    }

    if transport != b"unix" {
        return Ok(None);
    }

    Ok(match (path, abstract_name) {
        (Some(path), _) => Some(Endpoint::Path(path)),
        (None, Some(name)) => Some(Endpoint::Abstract(name)),
        (None, None) => None,
    })
}

/// Percent-unescape an address value.
fn unescape(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut it = bytes.iter();

    while let Some(&b) = it.next() {
        if b != b'%' {
            out.push(b);
            continue;
        }

        let (Some(hi), Some(lo)) = (
            it.next().and_then(|&b| hex(b)),
            it.next().and_then(|&b| hex(b)),
        ) else {
            return Err(Error::new(ErrorKind::InvalidAddress));
        };

        out.push((hi << 4) | lo);
    }

    Ok(out)
}

fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;

    use std::io::Write;
    use std::os::unix::net::UnixStream;

    use crate::RecvBuf;
    use crate::error::Result;
    use crate::proto::Endianness;

    use super::{Endpoint, Error, ErrorKind, Transport, parse_address, parse_address_list};

    fn list(address: &str) -> Vec<Option<Endpoint>> {
        parse_address_list(address.as_bytes())
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn invalid(address: &str) -> bool {
        let expected = Error::new(ErrorKind::InvalidAddress).to_string();
        parse_address(address.as_bytes()).is_err_and(|e| e.to_string() == expected)
    }

    #[test]
    fn path_with_guid() {
        assert_eq!(
            list("unix:path=/tmp/dbus-AbCdEf,guid=0123456789abcdef0123456789abcdef"),
            [Some(Endpoint::Path(b"/tmp/dbus-AbCdEf".to_vec()))]
        );
    }

    #[test]
    fn abstract_with_guid() {
        assert_eq!(
            list("unix:abstract=/tmp/dbus-x,guid=0123456789abcdef0123456789abcdef"),
            [Some(Endpoint::Abstract(b"/tmp/dbus-x".to_vec()))]
        );
    }

    #[test]
    fn escaped_path() {
        assert_eq!(
            list("unix:path=/tmp/a%2cb%2Cc%3d%25"),
            [Some(Endpoint::Path(b"/tmp/a,b,c=%".to_vec()))]
        );
    }

    #[test]
    fn address_list() {
        assert_eq!(
            list("tcp:host=localhost,port=1234;unix:path=/a;unix:abstract=b"),
            [
                None,
                Some(Endpoint::Path(b"/a".to_vec())),
                Some(Endpoint::Abstract(b"b".to_vec())),
            ]
        );
    }

    #[test]
    fn unusable_unix() {
        assert_eq!(list("unix:dir=/tmp,guid=00"), [None]);
    }

    #[test]
    fn malformed() {
        assert!(parse_address_list(b"").next().is_none());
        assert!(invalid(""));
        assert!(invalid("unix"));
        assert!(invalid("/tmp/socket"));
        assert!(invalid(":path=/a"));
        assert!(invalid("unix:path"));
        assert!(invalid("unix:=/a"));
        assert!(invalid("unix:path=/a%"));
        assert!(invalid("unix:path=/a%2"));
        assert!(invalid("unix:path=/a%zz"));
    }

    /// Receive `message` through a transport and parse it.
    fn recv(message: &[u8]) -> Result<()> {
        let (mut peer, stream) = UnixStream::pair()?;
        peer.write_all(message)?;

        let mut transport = Transport::from_std(stream);
        let mut recv = RecvBuf::new();
        let total = transport.idle(&mut recv)?;
        transport.recv_body(&mut recv, total)?;
        recv.last_message()?;
        Ok(())
    }

    #[track_caller]
    fn assert_error(result: Result<()>, kind: ErrorKind) {
        let expected = Error::new(kind).to_string();

        match result {
            Ok(()) => panic!("Expected error `{expected}`"),
            Err(error) => assert_eq!(error.to_string(), expected),
        }
    }

    /// A little endian signal with serial 1, an empty body and the given
    /// header fields.
    fn signal(endianness: u8, version: u8, fields: &[u8]) -> Vec<u8> {
        let mut message = vec![endianness, 4, 0, version];
        message.extend_from_slice(&0u32.to_le_bytes());
        message.extend_from_slice(&1u32.to_le_bytes());
        message.extend_from_slice(&(fields.len() as u32).to_le_bytes());
        message.extend_from_slice(fields);

        while message.len() % 8 != 0 {
            message.push(0);
        }

        message
    }

    /// The PATH `/` and MEMBER `A` header fields followed by a field with an
    /// unknown code whose value is a byte in `variants` nested variants.
    fn nested_field(variants: usize) -> Vec<u8> {
        let mut field = Vec::new();
        field.extend_from_slice(b"\x01\x01o\x00\x01\x00\x00\x00/\x00\x00\x00\x00\x00\x00\x00");
        field.extend_from_slice(b"\x03\x01s\x00\x01\x00\x00\x00A\x00\x00\x00\x00\x00\x00\x00");
        field.push(200);

        for _ in 1..variants {
            field.extend_from_slice(b"\x01v\x00");
        }

        field.extend_from_slice(b"\x01y\x00\x2a");
        field
    }

    #[test]
    fn valid_header() -> Result<()> {
        recv(&signal(b'l', 1, &nested_field(1)))?;
        recv(&signal(b'l', 1, &nested_field(62)))?;
        Ok(())
    }

    #[test]
    fn invalid_endianness() {
        assert_error(
            recv(&signal(b'x', 1, &[])),
            ErrorKind::InvalidEndianness(Endianness::new(b'x')),
        );
    }

    #[test]
    fn unsupported_version() {
        assert_error(
            recv(&signal(b'l', 2, &[])),
            ErrorKind::UnsupportedProtocolVersion(2),
        );
    }

    #[test]
    fn header_field_nested_too_deep() {
        // The header field is already inside of an array and a struct, so 62
        // variants is the most it can hold.
        assert_error(
            recv(&signal(b'l', 1, &nested_field(63))),
            ErrorKind::NestingTooDeep,
        );
    }
}
