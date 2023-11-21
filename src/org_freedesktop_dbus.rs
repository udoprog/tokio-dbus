//! Types associated with the `org.freedesktop.DBus` interface.

/// Well known destination name.
pub const DESTINATION: &str = "org.freedesktop.DBus";

/// Well known interface name.
pub const INTERFACE: &str = "org.freedesktop.DBus";

/// Well known D-Bus path.
pub const PATH: &str = "/org/freedesktop/DBus";

raw_set! {
    /// The flags to a `RequestName` call.
    #[repr(u32)]
    pub enum NameFlag {
        /// If an application A specifies this flag and succeeds in becoming the
        /// owner of the name, and another application B later calls
        /// `RequestName` with the `REPLACE_EXISTING` flag, then application A
        /// will lose ownership and receive a `org.freedesktop.DBus.NameLost`
        /// signal, and application B will become the new owner. If
        /// `ALLOW_REPLACEMENT` is not specified by application A, or
        /// `REPLACE_EXISTING` is not specified by application B, then
        /// application B will not replace application A as the owner.
        ALLOW_REPLACEMENT = 1,
        /// Try to replace the current owner if there is one. If this flag is
        /// not set the application will only become the owner of the name if
        /// there is no current owner. If this flag is set, the application will
        /// replace the current owner if the current owner specified
        /// `ALLOW_REPLACEMENT`.
        REPLACE_EXISTING = 2,
        /// Without this flag, if an application requests a name that is already
        /// owned, the application will be placed in a queue to own the name
        /// when the current owner gives it up. If this flag is given, the
        /// application will not be placed in the queue, the request for the
        /// name will simply fail. This flag also affects behavior when an
        /// application is replaced as name owner; by default the application
        /// moves back into the waiting queue, unless this flag was provided
        /// when the application became the name owner.
        DO_NOT_QUEUE = 4,
    }
}

raw_enum! {
    /// The reply to a `RequestName` call.
    #[repr(u32)]
    pub enum NameReply {
        /// The caller is now the primary owner of the name, replacing any
        /// previous owner. Either the name had no owner before, or the caller
        /// specified [`NameFlag::REPLACE_EXISTING`] and the current owner
        /// specified [`NameFlag::ALLOW_REPLACEMENT`].
        PRIMARY_OWNER = 1,
        /// The name already had an owner, [`NameFlag::DO_NOT_QUEUE`] was not
        /// specified, and either the current owner did not specify
        /// [`NameFlag::ALLOW_REPLACEMENT`] or the requesting application did
        /// not specify [`NameFlag::REPLACE_EXISTING`].
        IN_QUEUE = 2,
        /// The name already has an owner, [`NameFlag::DO_NOT_QUEUE`] was
        /// specified, and either [`NameFlag::ALLOW_REPLACEMENT`] was not
        /// specified by the current owner, or [`NameFlag::REPLACE_EXISTING`]
        /// was not specified by the requesting application.
        EXISTS = 3,
        /// The application trying to request ownership of a name is already the
        /// owner of it.
        ALREADY_OWNER = 4,
    }
}
