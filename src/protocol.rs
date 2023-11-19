//! Low level details for the D-Bus protocol implementation.

use std::fmt;
use std::ops::{BitAnd, BitOr, BitXor};

use crate::frame::Frame;

/// A protocol header.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub(crate) struct Header {
    pub(crate) endianness: Endianness,
    pub(crate) message_type: MessageType,
    pub(crate) flags: Flags,
    pub(crate) version: u8,
    pub(crate) body_length: u32,
    pub(crate) serial: u32,
}

unsafe impl Frame for Header {
    fn adjust(&mut self, endianness: Endianness) {
        self.body_length.adjust(endianness);
        self.serial.adjust(endianness);
    }
}

macro_rules! raw_enum {
    (
        $(#[doc = $doc:literal])*
        #[repr($repr:ty)]
        $vis:vis enum $name:ident {
            $(
                $(#[$($variant_meta:meta)*])*
                $variant:ident = $value:expr
            ),* $(,)?
        }
    ) => {
        $(#[doc = $doc])*
        #[derive(Clone, Copy, PartialEq, Eq)]
        #[repr(transparent)]
        $vis struct $name(pub(crate) $repr);

        impl $name {
            $(
                $(#[$($variant_meta)*])*
                $vis const $variant: Self = Self($value);
            )*
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match *self {
                    $(Self::$variant => f.write_str(stringify!($variant)),)*
                    _ => f.write_str("INVALID"),
                }
            }
        }
    }
}

macro_rules! raw_set {
    (
        $(#[doc = $doc:literal])*
        #[repr($repr:ty)]
        $vis:vis enum $name:ident {
            $(
                $(#[$($variant_meta:meta)*])*
                $variant:ident = $value:expr
            ),* $(,)?
        }
    ) => {
        $(#[doc = $doc])*
        #[derive(Default, Clone, Copy, PartialEq, Eq)]
        #[repr(transparent)]
        $vis struct $name(pub(crate) $repr);

        impl $name {
            $(
                $(#[$($variant_meta)*])*
                $vis const $variant: Self = Self($value);
            )*
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                struct Raw(&'static str);

                impl fmt::Debug for Raw {
                    #[inline]
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, "{}", self.0)
                    }
                }

                struct Bits($repr);

                impl fmt::Debug for Bits {
                    #[inline]
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        write!(f, "{:b}", self.0)
                    }
                }

                let mut f = f.debug_set();

                let mut this = *self;

                $(
                    if this & Self::$variant {
                        f.entry(&Raw(stringify!($variant)));
                        this = this ^ Self::$variant;
                    }
                )*

                if this.0 != 0 {
                    f.entry(&Bits(this.0));
                }

                f.finish()
            }
        }
    }
}

raw_enum! {
    /// The endianness of a message.
    #[repr(u8)]
    pub enum Endianness {
        /// Little endian.
        LITTLE = b'l',
        /// Big endian.
        BIG = b'B',
    }
}

impl Endianness {
    /// Native endian.
    #[cfg(target_endian = "little")]
    pub(crate) const NATIVE: Self = Self::LITTLE;
    /// Native endian.
    #[cfg(target_endian = "big")]
    pub(crate) const NATIVE: Self = Self::BIG;
}

raw_enum! {
    /// The type of a message.
    #[repr(u8)]
    pub(crate) enum MessageType {
        /// Method call. This message type may prompt a reply.
        METHOD_CALL = 1,
        /// Method reply with returned data.
        METHOD_RETURN = 2,
        /// Error reply. If the first argument exists and is a string, it is an
        /// error message.
        ERROR = 3,
        /// Signal emission.
        SIGNAL = 4,
    }
}

raw_set! {
    /// Flags inside of a D-Bus message.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::protocol::Flags;
    /// let flags = Flags::EMPTY;
    /// assert!(!(flags & Flags::NO_REPLY_EXPECTED));
    ///
    /// let flags = Flags::EMPTY | Flags::NO_REPLY_EXPECTED;
    /// assert!(flags & Flags::NO_REPLY_EXPECTED);
    /// assert!(!(flags & Flags::NO_AUTO_START));
    /// ```
    #[repr(u8)]
    pub enum Flags {
        /// An empty set of flags.
        EMPTY = 0,
        /// This message does not expect method return replies or error replies,
        /// even if it is of a type that can have a reply; the reply should be
        /// omitted.
        NO_REPLY_EXPECTED = 1,
        /// The bus must not launch an owner for the destination name in response to
        /// this message.
        NO_AUTO_START = 2,
        /// This flag may be set on a method call message to inform the receiving
        /// side that the caller is prepared to wait for interactive authorization,
        /// which might take a considerable time to complete. For instance, if this
        /// flag is set, it would be appropriate to query the user for passwords or
        /// confirmation via Polkit or a similar framework.
        ///
        /// This flag is only useful when unprivileged code calls a more privileged
        /// method call, and an authorization framework is deployed that allows
        /// possibly interactive authorization. If no such framework is deployed it
        /// has no effect. This flag should not be set by default by client
        /// implementations. If it is set, the caller should also set a suitably
        /// long timeout on the method call to make sure the user interaction may
        /// complete. This flag is only valid for method call messages, and shall be
        /// ignored otherwise.
        ///
        /// Interaction that takes place as a part of the effect of the method being
        /// called is outside the scope of this flag, even if it could also be
        /// characterized as authentication or authorization. For instance, in a
        /// method call that directs a network management service to attempt to
        /// connect to a virtual private network, this flag should control how the
        /// network management service makes the decision "is this user allowed to
        /// change system network configuration?", but it should not affect how or
        /// whether the network management service interacts with the user to obtain
        /// the credentials that are required for access to the VPN.
        ///
        /// If a this flag is not set on a method call, and a service determines
        /// that the requested operation is not allowed without interactive
        /// authorization, but could be allowed after successful interactive
        /// authorization, it may return the
        /// org.freedesktop.DBus.Error.InteractiveAuthorizationRequired error.
        ///
        /// The absence of this flag does not guarantee that interactive
        /// authorization will not be applied, since existing services that pre-date
        /// this flag might already use interactive authorization. However, existing
        /// D-Bus APIs that will use interactive authorization should document that
        /// the call may take longer than usual, and new D-Bus APIs should avoid
        /// interactive authorization in the absence of this flag.
        ALLOW_INTERACTIVE_AUTHORIZATION = 4,
    }
}

impl BitOr<Flags> for Flags {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Flags) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd<Flags> for Flags {
    type Output = bool;

    #[inline]
    fn bitand(self, rhs: Flags) -> Self::Output {
        self.0 & rhs.0 != 0
    }
}

impl BitXor<Flags> for Flags {
    type Output = Self;

    #[inline]
    fn bitxor(self, rhs: Flags) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

raw_enum! {
    #[repr(u8)]
    pub(crate) enum Variant {
        /// The object to send a call to, or the object a signal is emitted from.
        /// The special path /org/freedesktop/DBus/Local is reserved;
        /// implementations should not send messages with this path, and the
        /// reference implementation of the bus daemon will disconnect any
        /// application that attempts to do so. This header field is controlled by
        /// the message sender.
        PATH = 1,
        /// The interface to invoke a method call on, or that a signal is emitted
        /// from. Optional for method calls, required for signals. The special
        /// interface org.freedesktop.DBus.Local is reserved; implementations should
        /// not send messages with this interface, and the reference implementation
        /// of the bus daemon will disconnect any application that attempts to do
        /// so. This header field is controlled by the message sender.
        INTERFACE = 2,
        /// The member, either the method name or signal name. This header field is
        /// controlled by the message sender.
        MEMBER = 3,
        /// The name of the error that occurred, for errors.
        ERROR_NAME = 4,
        /// The serial number of the message this message is a reply to. (The serial
        /// number is the second UINT32 in the header.) This header field is
        /// controlled by the message sender.
        REPLY_SERIAL = 5,
        /// The name of the connection this message is intended for. This field is
        /// usually only meaningful in combination with the message bus (see the
        /// section called “Message Bus Specification”), but other servers may
        /// define their own meanings for it. This header field is controlled by the
        /// message sender.
        DESTINATION = 6,
        /// Unique name of the sending connection. This field is usually only
        /// meaningful in combination with the message bus, but other servers may
        /// define their own meanings for it. On a message bus, this header field is
        /// controlled by the message bus, so it is as reliable and trustworthy as
        /// the message bus itself. Otherwise, this header field is controlled by
        /// the message sender, unless there is out-of-band information that
        /// indicates otherwise.
        SENDER = 7,
        /// The signature of the message body. If omitted, it is assumed to be the
        /// empty signature "" (i.e. the body must be 0-length). This header field
        /// is controlled by the message sender.
        SIGNATURE = 8,
        /// The number of Unix file descriptors that accompany the message. If
        /// omitted, it is assumed that no Unix file descriptors accompany the
        /// message. The actual file descriptors need to be transferred via platform
        /// specific mechanism out-of-band. They must be sent at the same time as
        /// part of the message itself. They may not be sent before the first byte
        /// of the message itself is transferred or after the last byte of the
        /// message itself. This header field is controlled by the message sender.
        UNIX_FDS = 9,
    }
}

unsafe impl Frame for Variant {
    #[inline]
    fn adjust(&mut self, _: Endianness) {
        // NB: single byte so no adjustment needed.
    }
}

raw_enum! {
    /// The type inside of a signature.
    #[repr(u8)]
    pub(crate) enum Type {
        /// Not a valid type code, used to terminate signatures
        INVALID = b'\0',
        /// 8-bit unsigned integer
        BYTE = b'y',
        /// Boolean value, 0 is FALSE and 1 is TRUE. Everything else is invalid.
        BOOLEAN = b'b',
        /// 16-bit signed integer
        INT16 = b'n',
        /// 16-bit unsigned integer
        UINT16 = b'q',
        /// 32-bit signed integer
        INT32 = b'i',
        /// 32-bit unsigned integer
        UINT32 = b'u',
        /// 64-bit signed integer
        INT64 = b'x',
        /// 64-bit unsigned integer
        UINT64 = b't',
        /// IEEE 754 double
        DOUBLE = b'd',
        /// UTF-8 string (must be valid UTF-8). Must be nul terminated and contain
        /// no other nul bytes.
        STRING = b's',
        /// Name of an object instance
        OBJECT_PATH = b'o',
        /// A type signature
        SIGNATURE = b'g',
        /// Array.
        ARRAY = b'a',
        /// Struct; type code 114 'r' is reserved for use in bindings and
        /// implementations to represent the general concept of a struct, and must
        /// not appear in signatures used on D-Bus..
        STRUCT = b'r',
        OPEN_PAREN = b'(',
        CLOSE_PAREN = b')',
        /// Variant type (the type of the value is part of the value itself).
        VARIANT = b'v',
        /// Entry in a dict or map (array of key-value pairs). Type code 101 'e' is
        /// reserved for use in bindings and implementations to represent the
        /// general concept of a dict or dict-entry, and must not appear in
        /// signatures used on D-Bus..
        DICT_ENTRY = b'e',
        OPEN_BRACE = b'{',
        CLOSE_BRACE = b'}',
        /// Unix file descriptor.
        UNIX_FD = b'h',
        /// Reserved for a 'maybe' type compatible with the one in GVariant, and
        /// must not appear in signatures used on D-Bus until specified here.
        RESERVED0 = b'm',
        /// Reserved for use in bindings/implementations to represent any single
        /// complete type, and must not appear in signatures used on D-Bus.
        RESERVED1 = b'*',
        /// Reserved for use in bindings/implementations to represent any basic
        /// type, and must not appear in signatures used on D-Bus.
        RESERVED2 = b'?',
        /// Reserved for internal use by bindings/implementations, and must not
        /// appear in signatures used on D-Bus. GVariant uses these type-codes to
        /// encode calling conventions.
        RESERVED3 = b'@',
        RESERVED4 = b'&',
        RESERVED5 = b'^',
    }
}
