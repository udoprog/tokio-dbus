use crate::error::Result;
use crate::org_freedesktop_dbus::{self, NameFlag};
use crate::{BodyBuf, RecvBuf, SendBuf, Serial};

/// A collection of heap-allocated buffers for use in connections
///
/// # Examples
///
/// ```
/// use tokio_dbus::Buffers;
///
/// let buffers = Buffers::new();
/// ```
///
/// The [`RecvBuf`] instance is used to access messages received after a call to
/// [`wait()`], through [`RecvBuf::last_message()`].
///
/// The [`BodyBuf`] is the internal buffer that the client uses to construct
/// message bodies. It is empty when it's returned.
///
/// [`wait()`]: crate::Transport::wait
///
/// This is useful, because it permits using all parts of the connection without
/// running into borrowing issues.
///
/// [`write_message()`]: Self::write_message
///
/// We can address this by using [`buffers()`]:
///
/// [`buffers()`]: Self::buffers
///
/// ```no_run
/// use tokio_dbus::{Buffers, Connection, Message};
///
/// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
/// let mut c = Connection::session_bus()?;
///
/// let mut buf = Buffers::new();
/// c.connect(&mut buf).await?;
/// c.wait(&mut buf).await?;
///
/// let message: Message<'_> = buf.recv.last_message()?;
/// let m = message.method_return(buf.send.next_serial()).with_body(&buf.body);
///
/// buf.send.write_message(m);
/// # Ok(()) }
/// ```
#[non_exhaustive]
pub struct Buffers {
    /// The receive buffer.
    pub recv: RecvBuf,
    /// The send buffer.
    pub send: SendBuf,
    /// The body buffer.
    pub body: BodyBuf,
}

impl Buffers {
    /// Construct a new set of buffers.
    pub fn new() -> Self {
        Self {
            recv: RecvBuf::new(),
            send: SendBuf::new(),
            body: BodyBuf::new(),
        }
    }

    /// Serialize a "Hello" message.
    ///
    /// This is the first message that a client MUST send after connecting to the bus.
    pub fn hello(&mut self) -> Result<Serial> {
        let m = self
            .send
            .method_call(org_freedesktop_dbus::PATH, "Hello")
            .with_destination(org_freedesktop_dbus::DESTINATION);

        let serial = m.serial();
        self.send.write_message(m)?;
        Ok(serial)
    }

    /// Request the given name from the bus.
    ///
    /// This is used when servers connect in order to make them addressable.
    pub fn request_name(&mut self, name: &str, flags: NameFlag) -> Result<Serial> {
        self.body.clear();
        self.body.store(name)?;
        self.body.store(flags)?;

        let m = self
            .send
            .method_call(org_freedesktop_dbus::PATH, "RequestName")
            .with_destination(org_freedesktop_dbus::DESTINATION)
            .with_body(&self.body);

        let serial = m.serial();
        self.send.write_message(m)?;
        Ok(serial)
    }

    /// Release a name which was previously acquired with [`request_name()`].
    ///
    /// [`request_name()`]: Self::request_name
    pub fn release_name(&mut self, name: &str) -> Result<Serial> {
        self.body.clear();
        self.body.store(name)?;

        let m = self
            .send
            .method_call(org_freedesktop_dbus::PATH, "ReleaseName")
            .with_destination(org_freedesktop_dbus::DESTINATION)
            .with_body(&self.body);

        let serial = m.serial();
        self.send.write_message(m)?;
        Ok(serial)
    }

    /// Add a match rule, causing the bus to route matching messages to this
    /// connection.
    ///
    /// Signals other than the ones the bus emits directly at this connection
    /// are only delivered once a rule matching them has been added.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus::{Buffers, Connection};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus::Result<()> {
    /// let mut c = Connection::session_bus()?;
    /// let mut buf = Buffers::new();
    /// c.connect(&mut buf).await?;
    ///
    /// buf.hello()?;
    /// buf.add_match(
    ///     "type='signal',sender='org.freedesktop.DBus',\
    ///      interface='org.freedesktop.DBus',member='NameOwnerChanged'",
    /// )?;
    /// # Ok(()) }
    /// ```
    pub fn add_match(&mut self, rule: &str) -> Result<Serial> {
        self.match_rule("AddMatch", rule)
    }

    /// Remove a match rule which was previously added with [`add_match()`].
    ///
    /// [`add_match()`]: Self::add_match
    pub fn remove_match(&mut self, rule: &str) -> Result<Serial> {
        self.match_rule("RemoveMatch", rule)
    }

    fn match_rule(&mut self, member: &str, rule: &str) -> Result<Serial> {
        self.body.clear();
        self.body.store(rule)?;

        let m = self
            .send
            .method_call(org_freedesktop_dbus::PATH, member)
            .with_destination(org_freedesktop_dbus::DESTINATION)
            .with_body(&self.body);

        let serial = m.serial();
        self.send.write_message(m)?;
        Ok(serial)
    }
}

impl Default for Buffers {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
