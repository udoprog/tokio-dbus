use crate::{Body, BodyBuf};

mod sealed {
    use crate::{Body, BodyBuf};
    pub trait Sealed<'de> {}
    impl<'de> Sealed<'de> for &Body<'de> {}
    impl<'de> Sealed<'de> for Body<'de> {}
    impl<'de> Sealed<'de> for &'de BodyBuf {}
    impl<'de> Sealed<'de> for &'de mut BodyBuf {}
}

/// Trait for types which can be cheaply coerced into a [`Body`].
///
/// This is used in combination with [`Message::with_body`] to allow for
/// convenient construction of a borrowed body.
///
/// [`Message::with_body`]: crate::Message::with_body
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, MessageKind, ObjectPath, SendBuf, Signature};
///
/// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
///
/// let mut send = SendBuf::new();
/// let mut body = BodyBuf::new();
///
/// body.store("Hello World!");
///
/// let m = send.method_call(PATH, "Hello")
///     .with_body(&body);
///
/// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
/// assert_eq!(m.signature(), Signature::STRING);
/// ```
pub trait AsBody<'de>: self::sealed::Sealed<'de> {
    /// Coerce this type into a [`Body`].
    #[allow(clippy::wrong_self_convention)]
    fn as_body(self) -> Body<'de>;
}

/// Convert a reference to a [`Body`] into a [`Body`].
///
/// Since [`Body`] is cheap to clone, it doesn't hurt to provide this coercions.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, MessageKind, ObjectPath, SendBuf, Signature};
///
/// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
///
/// let mut send = SendBuf::new();
/// let mut body = BodyBuf::new();
///
/// body.store("Hello World!");
///
/// let m = send.method_call(PATH, "Hello")
///     .with_body(&body.as_body());
///
/// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
/// assert_eq!(m.signature(), Signature::STRING);
/// ```
impl<'de> AsBody<'de> for &Body<'de> {
    #[inline]
    fn as_body(self) -> Body<'de> {
        self.clone()
    }
}

/// Convert a [`Body`] into a [`Body`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, MessageKind, ObjectPath, SendBuf, Signature};
///
/// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
///
/// let mut send = SendBuf::new();
/// let mut body = BodyBuf::new();
///
/// body.store("Hello World!");
///
/// let m = send.method_call(PATH, "Hello")
///     .with_body(body.as_body());
///
/// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
/// assert_eq!(m.signature(), Signature::STRING);
/// ```
impl<'de> AsBody<'de> for Body<'de> {
    #[inline]
    fn as_body(self) -> Body<'de> {
        self
    }
}

/// Convert a borrowed [`BodyBuf`] into a [`Body`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, MessageKind, ObjectPath, SendBuf, Signature};
///
/// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
///
/// let mut send = SendBuf::new();
/// let mut body = BodyBuf::new();
///
/// body.store("Hello World!");
///
/// let m = send.method_call(PATH, "Hello")
///     .with_body(&body);
///
/// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
/// assert_eq!(m.signature(), Signature::STRING);
/// ```
impl<'de> AsBody<'de> for &'de BodyBuf {
    #[inline]
    fn as_body(self) -> Body<'de> {
        BodyBuf::as_body(self)
    }
}

/// Convert a mutably borrowed [`BodyBuf`] into a [`Body`].
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, MessageKind, ObjectPath, SendBuf, Signature};
///
/// const PATH: &ObjectPath = ObjectPath::new_const(b"/org/freedesktop/DBus");
///
/// let mut send = SendBuf::new();
/// let mut body = BodyBuf::new();
///
/// body.store("Hello World!");
///
/// let m = send.method_call(PATH, "Hello")
///     .with_body(&mut body);
///
/// assert!(matches!(m.kind(), MessageKind::MethodCall { .. }));
/// assert_eq!(m.signature(), Signature::STRING);
/// ```
impl<'de> AsBody<'de> for &'de mut BodyBuf {
    #[inline]
    fn as_body(self) -> Body<'de> {
        BodyBuf::as_body(self)
    }
}
