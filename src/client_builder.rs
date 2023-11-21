use crate::error::Result;
use crate::sasl::{Auth, SaslRequest, SaslResponse};
use crate::{Client, Connection};

enum BusKind {
    Session,
    System,
}

enum AuthKind {
    #[cfg_attr(feature = "libc", allow(unused))]
    None,
    #[cfg(feature = "libc")]
    Uid,
}

impl AuthKind {
    #[cfg(not(feature = "libc"))]
    const DEFAULT: Self = Self::None;
    #[cfg(feature = "libc")]
    const DEFAULT: Self = Self::Uid;
}

/// Builder of a [`Client`].
pub struct ClientBuilder {
    bus: BusKind,
    auth: AuthKind,
}

impl ClientBuilder {
    /// Construct a new client builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::ClientBuilder;
    ///
    /// let c = ClientBuilder::new();
    /// ```
    pub fn new() -> Self {
        Self {
            bus: BusKind::Session,
            auth: AuthKind::DEFAULT,
        }
    }

    /// Construct a client connecting to the session bus (default).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::ClientBuilder;
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let c = ClientBuilder::new().session_bus().connect().await?;
    /// # Ok(()) }
    /// ```
    pub fn session_bus(&mut self) -> &mut Self {
        self.bus = BusKind::Session;
        self
    }

    /// Construct a client connecting to the system bus.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::ClientBuilder;
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let c = ClientBuilder::new().system_bus().connect().await?;
    /// # Ok(()) }
    /// ```
    pub fn system_bus(&mut self) -> &mut Self {
        self.bus = BusKind::System;
        self
    }

    /// Construct and connect a [`Client`] with the current configuration.
    pub async fn connect(&self) -> Result<Client> {
        let c = match self.bus {
            BusKind::Session => Connection::session_bus()?,
            BusKind::System => Connection::system_bus()?,
        };

        let mut auth_buf;

        let auth = match self.auth {
            AuthKind::None => None,
            #[cfg(feature = "libc")]
            AuthKind::Uid => {
                auth_buf = [0; 32];
                Some(Auth::external_from_uid(&mut auth_buf))
            }
        };

        let mut c = Client::new(c)?;

        if let Some(auth) = auth {
            let sasl = c.sasl_request(&SaslRequest::Auth(auth)).await?;

            match sasl {
                SaslResponse::Ok(..) => {}
            }
        }

        // Transition to message mode.
        c.sasl_begin().await?;
        c.hello()?;
        Ok(c)
    }
}

impl Default for ClientBuilder {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
