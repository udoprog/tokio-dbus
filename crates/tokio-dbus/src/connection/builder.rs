use crate::error::Result;
#[cfg(not(feature = "libc"))]
use crate::error::{Error, ErrorKind};

use super::{Connection, Transport};

enum BusKind {
    Session,
    System,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AuthKind {
    /// No authentication.
    None,
    /// Authenticate using the current UID.
    ///
    /// This is only supported if the `libc` feature is enabled.
    Uid,
}

impl AuthKind {
    #[cfg(not(feature = "libc"))]
    const DEFAULT: Self = Self::None;
    #[cfg(feature = "libc")]
    const DEFAULT: Self = Self::Uid;
}

/// Builder of a [`Connection`].
pub struct ConnectionBuilder {
    bus: BusKind,
    auth: AuthKind,
}

impl ConnectionBuilder {
    /// Construct a new [`ConnectionBuilder`] with the default configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::ConnectionBuilder;
    ///
    /// let c = ConnectionBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            bus: BusKind::Session,
            auth: AuthKind::DEFAULT,
        }
    }

    /// Explicitly disable authentication for this connection.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::{Buffers, ConnectionBuilder};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let c = ConnectionBuilder::new().no_auth().session_bus().build()?;
    /// # Ok(()) }
    /// ```
    pub fn no_auth(&mut self) -> &mut Self {
        self.auth = AuthKind::None;
        self
    }

    /// Construct a connection connecting to the session bus (default).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::{Buffers, ConnectionBuilder};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let c = ConnectionBuilder::new().session_bus().build()?;
    /// # Ok(()) }
    /// ```
    pub fn session_bus(&mut self) -> &mut Self {
        self.bus = BusKind::Session;
        self
    }

    /// Construct a connection connecting to the system bus.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::{Buffers, ConnectionBuilder};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let c = ConnectionBuilder::new().system_bus().build()?;
    /// # Ok(()) }
    /// ```
    pub fn system_bus(&mut self) -> &mut Self {
        self.bus = BusKind::System;
        self
    }

    /// Construct and connect a [`Connection`] with the current configuration.
    pub fn build(&self) -> Result<Connection> {
        let transport = match self.bus {
            BusKind::Session => Transport::session_bus()?,
            BusKind::System => Transport::system_bus()?,
        };

        Ok(Connection::new(self.auth, transport)?)
    }
}

impl Default for ConnectionBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
