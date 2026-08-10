use tokio_dbus::org_freedesktop_dbus;

use crate::{Result, SignalMessage};

/// The `org.freedesktop.DBus.NameOwnerChanged` signal, which the bus emits
/// when the ownership of a well known name changes.
///
/// Register interest with [`Connection::watch_name`], then decode incoming
/// signals with [`decode()`].
///
/// [`Connection::watch_name`]: crate::Connection::watch_name
/// [`decode()`]: Self::decode
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct NameOwnerChanged {
    /// The name whose ownership changed.
    pub name: String,
    /// The unique name of the previous owner, or `None` when the name was not
    /// owned before.
    pub old_owner: Option<String>,
    /// The unique name of the new owner, or `None` when the name went away.
    pub new_owner: Option<String>,
}

impl NameOwnerChanged {
    /// The member name of this signal.
    pub const MEMBER: &'static str = "NameOwnerChanged";

    /// Decode a `NameOwnerChanged` signal.
    ///
    /// Returns `None` when the message is some other signal, so this can be
    /// applied to everything which arrives without inspecting the message
    /// first. The empty owner strings the bus uses for "no owner" are decoded
    /// into `None`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio_dbus_runtime::{Connection, Incoming, NameOwnerChanged};
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus_runtime::Result<()> {
    /// let mut c = Connection::session_bus().await?;
    /// c.watch_name("org.kde.StatusNotifierWatcher").await?;
    ///
    /// loop {
    ///     if let Incoming::Signal(message) = c.next().await? {
    ///         if let Some(changed) = NameOwnerChanged::decode(&message)? {
    ///             if changed.new_owner.is_some() {
    ///                 // The watcher is back, register with it again.
    ///             }
    ///         }
    ///     }
    /// }
    /// # }
    /// ```
    pub fn decode(message: &SignalMessage) -> Result<Option<Self>> {
        if message.interface() != Some(org_freedesktop_dbus::INTERFACE)
            || message.member() != Self::MEMBER
            || message.sender() != Some(org_freedesktop_dbus::DESTINATION)
        {
            return Ok(None);
        }

        let mut body = message.body();
        let name = body.read::<str>()?.to_owned();
        let old_owner = body.read::<str>()?;
        let new_owner = body.read::<str>()?;

        Ok(Some(Self {
            name,
            old_owner: (!old_owner.is_empty()).then(|| old_owner.to_owned()),
            new_owner: (!new_owner.is_empty()).then(|| new_owner.to_owned()),
        }))
    }

    /// The match rule which selects this signal for `name`, as passed to
    /// [`Connection::add_match`].
    ///
    /// [`Connection::add_match`]: crate::Connection::add_match
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus_runtime::NameOwnerChanged;
    ///
    /// assert_eq!(
    ///     NameOwnerChanged::rule("org.kde.StatusNotifierWatcher"),
    ///     "type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',member='NameOwnerChanged',arg0='org.kde.StatusNotifierWatcher'",
    /// );
    /// ```
    pub fn rule(name: &str) -> String {
        format!(
            "type='signal',sender='{bus}',interface='{bus}',member='{member}',arg0='{name}'",
            bus = org_freedesktop_dbus::DESTINATION,
            member = Self::MEMBER,
        )
    }
}
