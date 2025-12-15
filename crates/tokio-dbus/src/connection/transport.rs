use core::mem::size_of;
use core::num::NonZeroU32;

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

    /// Construct a new connection to the session bus.
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

    /// Construct a new connection to the session bus.
    ///
    /// This uses the `DBUS_SESSION_BUS_ADDRESS` environment variable to
    /// determine its address.
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

    /// Receive a the remaining body.
    pub(crate) fn recv_body(&mut self, recv: &mut RecvBuf, total: usize) -> Result<()> {
        self.recv_buf(recv.buf_mut(), total)?;
        Ok(())
    }

    /// Receive exactly `n` bytes into the receive buffer.
    pub(crate) fn recv_buf(&mut self, buf: &mut AlignedBuf, n: usize) -> io::Result<()> {
        buf.reserve_bytes(n);

        while buf.len() < n {
            let n = self.stream.read(&mut buf.get_mut()[..n])?;

            if n == 0 {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
            }

            buf.advance(n);
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
