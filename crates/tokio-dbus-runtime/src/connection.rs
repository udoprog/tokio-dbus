use std::collections::VecDeque;
use std::fmt;

use tokio_dbus::org_freedesktop_dbus::{self, NameFlag, NameReply};
use tokio_dbus::{
    Alignment, Body, BodyBuf, Buffers, MessageBuf, MessageKind, ObjectPath, RawArray, Serial,
    Signature,
};

use crate::error::ErrorKind;
use crate::{Decode, Encode, Error, Result};

/// The body of a message being built.
///
/// The signature of the arguments is declared up front, since generated code
/// knows it at build time, after which each argument is written in order.
///
/// # Examples
///
/// ```
/// use tokio_dbus::Signature;
/// use tokio_dbus_runtime::Arguments;
///
/// let mut arguments = Arguments::new(Signature::new("su")?)?;
/// arguments.store("Hello World!");
/// arguments.store(&42u32);
/// # Ok::<_, tokio_dbus_runtime::Error>(())
/// ```
#[derive(Default)]
pub struct Arguments {
    buf: BodyBuf,
}

impl Arguments {
    /// Construct an argument list matching the given signature.
    pub fn new(signature: &Signature) -> Result<Self> {
        let mut buf = BodyBuf::new();
        buf.extend_signature(signature)?;
        Ok(Self { buf })
    }

    /// Construct an empty argument list.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Write the next argument.
    pub fn store<T>(&mut self, value: T) -> &mut Self
    where
        T: Encode,
    {
        value.encode(&mut self.buf.raw());
        self
    }

    /// Write the next argument as a variant containing a value of the given
    /// type.
    ///
    /// The signature is the one of the value inside the variant, not the `v` of
    /// the variant itself.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::Signature;
    /// use tokio_dbus_runtime::Arguments;
    ///
    /// let mut arguments = Arguments::new(Signature::VARIANT)?;
    /// arguments.store_variant(Signature::UINT32, 42u32);
    /// # Ok::<_, tokio_dbus_runtime::Error>(())
    /// ```
    pub fn store_variant<T>(&mut self, signature: &Signature, value: T) -> &mut Self
    where
        T: Encode,
    {
        let mut raw = self.buf.raw();
        raw.store_signature(signature);
        value.encode(&mut raw);
        self
    }

    /// Write the next argument as an `a{sv}`, which is how a set of properties
    /// of differing types is carried.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::Signature;
    /// use tokio_dbus_runtime::Arguments;
    ///
    /// let mut arguments = Arguments::new(Signature::new("a{sv}")?)?;
    ///
    /// let mut dict = arguments.store_variant_dict();
    /// dict.entry("Version", Signature::UINT32, 3u32);
    /// dict.entry("Status", Signature::STRING, "normal");
    /// dict.finish();
    /// # Ok::<_, tokio_dbus_runtime::Error>(())
    /// ```
    pub fn store_variant_dict(&mut self) -> VariantDict<'_> {
        VariantDict {
            // NB: Dict entries are aligned just like structs.
            array: self.buf.raw().into_array(Alignment::U64),
        }
    }

    fn body(&self) -> Body<'_> {
        self.buf.as_body()
    }

    #[cfg(test)]
    pub(crate) fn body_for_test(&self) -> Body<'_> {
        self.body()
    }
}

/// A writer for an `a{sv}`, where every value is a variant of its own type.
///
/// See [`Arguments::store_variant_dict`].
pub struct VariantDict<'a> {
    array: RawArray<'a>,
}

impl VariantDict<'_> {
    /// Write an entry, whose value is a variant containing a value of the given
    /// type.
    pub fn entry<T>(&mut self, name: &str, signature: &Signature, value: T) -> &mut Self
    where
        T: Encode,
    {
        let mut entry = self.array.as_raw();
        entry.align(Alignment::U64);
        name.encode(&mut entry);
        entry.store_signature(signature);
        value.encode(&mut entry);
        self
    }

    /// Finish writing the dictionary.
    ///
    /// This also happens implicitly when the writer is dropped.
    pub fn finish(self) {}
}

/// Read a variant which is expected to contain a value of type `T`.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};
/// use tokio_dbus_runtime::decode_variant;
///
/// let mut buf = BodyBuf::new();
/// buf.store_variant(Signature::UINT32)?.store(42u32);
///
/// let mut body = buf.as_body();
/// assert_eq!(decode_variant::<u32>(&mut body, Signature::UINT32)?, 42);
/// # Ok::<_, tokio_dbus_runtime::Error>(())
/// ```
pub fn decode_variant<T>(body: &mut Body<'_>, expected: &Signature) -> Result<T>
where
    T: Decode,
{
    let signature = body.read::<Signature>()?;

    if signature != expected {
        return Err(Error::new(ErrorKind::UnexpectedSignature(Box::new((
            expected.to_owned(),
            signature.to_owned(),
        )))));
    }

    T::decode(body)
}

impl fmt::Debug for Arguments {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Arguments")
            .field("signature", &self.buf.signature())
            .finish()
    }
}

/// A connection to a bus which speaks in owned Rust values.
///
/// This is the driver used by generated clients and servers. It wraps a
/// [`tokio_dbus::Connection`] and takes care of matching replies to calls,
/// buffering the messages which arrive while a call is outstanding so that they
/// can be dispatched later.
///
/// Incoming messages are copied out of the receive buffer so that the connection
/// stays usable while one is being handled. Use the low level API directly if
/// that copy matters.
pub struct Connection {
    connection: tokio_dbus::Connection,
    buffers: Buffers,
    /// Messages which arrived while waiting for the reply to a call.
    queue: VecDeque<MessageBuf>,
    unique_name: String,
}

impl Connection {
    /// Connect to the session bus and say `Hello`.
    pub async fn session_bus() -> Result<Self> {
        Self::start(tokio_dbus::Connection::session_bus()?).await
    }

    /// Connect to the system bus and say `Hello`.
    pub async fn system_bus() -> Result<Self> {
        Self::start(tokio_dbus::Connection::system_bus()?).await
    }

    async fn start(connection: tokio_dbus::Connection) -> Result<Self> {
        let mut this = Self {
            connection,
            buffers: Buffers::new(),
            queue: VecDeque::new(),
            unique_name: String::new(),
        };

        this.connection.connect(&mut this.buffers).await?;

        let serial = this.buffers.hello()?;
        let reply = this.wait_for(serial).await?;

        let Ok(name) = reply.body().read::<str>() else {
            return Err(Error::new(ErrorKind::MissingUniqueName));
        };

        this.unique_name = name.to_owned();
        Ok(this)
    }

    /// The unique name the bus assigned to this connection, such as `:1.42`.
    pub fn unique_name(&self) -> &str {
        &self.unique_name
    }

    /// Call a method and wait for its reply.
    ///
    /// An error reply is turned into an [`Error`] carrying the name the remote
    /// end used.
    pub async fn call(
        &mut self,
        destination: &str,
        path: &ObjectPath,
        interface: &str,
        member: &str,
        arguments: &Arguments,
    ) -> Result<Reply> {
        let m = self
            .buffers
            .send
            .method_call(path, member)
            .with_destination(destination)
            .with_interface(interface)
            .with_body(arguments.body());

        let serial = m.serial();
        self.buffers.send.write_message(m)?;
        let message = self.wait_for(serial).await?;
        Ok(Reply { message })
    }

    /// Emit a signal.
    ///
    /// Signals are buffered and written out the next time the connection makes
    /// progress. Call [`flush()`] to force them out.
    ///
    /// [`flush()`]: Self::flush
    pub fn emit(
        &mut self,
        path: &ObjectPath,
        interface: &str,
        member: &str,
        arguments: &Arguments,
    ) -> Result<()> {
        let m = self
            .buffers
            .send
            .signal(path, member)
            .with_interface(interface)
            .with_body(arguments.body());

        self.buffers.send.write_message(m)?;
        Ok(())
    }

    /// Reply to a method call.
    pub fn reply(&mut self, call: &Call, arguments: &Arguments) -> Result<()> {
        let m = call
            .message
            .borrow()
            .method_return(self.buffers.send.next_serial())
            .with_body(arguments.body());

        self.buffers.send.write_message(m)?;
        Ok(())
    }

    /// Reply to a method call with an error.
    pub fn reply_error(&mut self, call: &Call, error: &Error) -> Result<()> {
        let name = error
            .name()
            .unwrap_or(org_freedesktop_dbus::FAILED_ERROR)
            .to_owned();

        let mut arguments = Arguments::new(Signature::STRING)?;
        arguments.store(error.to_string().as_str());

        let m = call
            .message
            .borrow()
            .error(&name, self.buffers.send.next_serial())
            .with_body(arguments.body());

        self.buffers.send.write_message(m)?;
        Ok(())
    }

    /// Request ownership of a well known name.
    pub async fn request_name(&mut self, name: &str, flags: NameFlag) -> Result<NameReply> {
        let serial = self.buffers.request_name(name, flags)?;
        let reply = self.wait_for(serial).await?;
        Ok(reply.body().load::<NameReply>()?)
    }

    /// Request ownership of a well known name, erroring unless it was acquired.
    pub async fn acquire_name(&mut self, name: &str, flags: NameFlag) -> Result<()> {
        match self.request_name(name, flags).await? {
            NameReply::PRIMARY_OWNER | NameReply::ALREADY_OWNER => Ok(()),
            _ => Err(Error::new(ErrorKind::NameTaken(name.into()))),
        }
    }

    /// Release a well known name previously acquired.
    pub async fn release_name(&mut self, name: &str) -> Result<()> {
        let serial = self.buffers.release_name(name)?;
        self.wait_for(serial).await?;
        Ok(())
    }

    /// Add a match rule, so that the bus routes matching signals here.
    pub async fn add_match(&mut self, rule: &str) -> Result<()> {
        let serial = self.buffers.add_match(rule)?;
        self.wait_for(serial).await?;
        Ok(())
    }

    /// Remove a match rule.
    pub async fn remove_match(&mut self, rule: &str) -> Result<()> {
        let serial = self.buffers.remove_match(rule)?;
        self.wait_for(serial).await?;
        Ok(())
    }

    /// Write out everything which has been buffered for sending.
    ///
    /// This is only needed before dropping the connection, since [`next()`] and
    /// [`call()`] both drive writes as a side effect.
    ///
    /// [`next()`]: Self::next
    /// [`call()`]: Self::call
    pub async fn flush(&mut self) -> Result<()> {
        self.connection.flush(&mut self.buffers).await?;

        if self.buffers.recv.has_message() {
            let message = self.buffers.recv.last_message()?.to_owned();
            self.queue.push_back(message);
            self.buffers.recv.clear();
        }

        Ok(())
    }

    /// Wait for the next method call or signal directed at this connection.
    pub async fn next(&mut self) -> Result<Incoming> {
        loop {
            if let Some(message) = self.queue.pop_front() {
                if let Some(incoming) = Incoming::new(message) {
                    return Ok(incoming);
                }

                continue;
            }

            self.connection.wait(&mut self.buffers).await?;
            let message = self.buffers.recv.last_message()?.to_owned();

            if let Some(incoming) = Incoming::new(message) {
                return Ok(incoming);
            }
        }
    }

    /// Drive the connection until the reply with the given serial arrives,
    /// queueing everything else which shows up in the meantime.
    async fn wait_for(&mut self, serial: Serial) -> Result<MessageBuf> {
        loop {
            self.connection.wait(&mut self.buffers).await?;
            let message = self.buffers.recv.last_message()?;

            match message.kind() {
                MessageKind::MethodReturn { reply_serial } if reply_serial == serial => {
                    return Ok(message.to_owned());
                }
                MessageKind::Error {
                    error_name,
                    reply_serial,
                } if reply_serial == serial => {
                    let text = message.body().read::<str>().unwrap_or_default();
                    return Err(Error::remote(error_name, text));
                }
                _ => {
                    let message = message.to_owned();
                    self.queue.push_back(message);
                }
            }
        }
    }
}

/// The reply to a method call.
pub struct Reply {
    message: MessageBuf,
}

impl Reply {
    /// The body of the reply, from which the return values are read.
    pub fn body(&self) -> Body<'_> {
        self.message.body()
    }

    /// Read a single return value.
    pub fn read<T>(&self) -> Result<T>
    where
        T: Decode,
    {
        T::decode(&mut self.body())
    }
}

impl fmt::Debug for Reply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

/// A message which arrived on the connection and is not a reply.
#[derive(Debug)]
#[non_exhaustive]
pub enum Incoming {
    /// A method call which is expected to be replied to.
    Call(Call),
    /// A signal, which is never replied to.
    Signal(SignalMessage),
}

impl Incoming {
    fn new(message: MessageBuf) -> Option<Self> {
        match message.kind() {
            MessageKind::MethodCall { .. } => Some(Incoming::Call(Call { message })),
            MessageKind::Signal { .. } => Some(Incoming::Signal(SignalMessage { message })),
            // NB: A reply which nothing is waiting for anymore.
            _ => None,
        }
    }
}

/// An incoming method call.
pub struct Call {
    message: MessageBuf,
}

impl Call {
    /// The object the call is addressed to.
    pub fn path(&self) -> &ObjectPath {
        match self.message.kind() {
            MessageKind::MethodCall { path, .. } => path,
            _ => unreachable!("Only constructed from a method call"),
        }
    }

    /// The method being called.
    pub fn member(&self) -> &str {
        match self.message.kind() {
            MessageKind::MethodCall { member, .. } => member,
            _ => unreachable!("Only constructed from a method call"),
        }
    }

    /// The interface the method belongs to, if the caller named one.
    pub fn interface(&self) -> Option<&str> {
        self.message.interface()
    }

    /// The unique name of the caller.
    pub fn sender(&self) -> Option<&str> {
        self.message.sender()
    }

    /// The body of the call, from which the arguments are read.
    pub fn body(&self) -> Body<'_> {
        self.message.body()
    }
}

impl fmt::Debug for Call {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Call")
            .field("path", &self.path())
            .field("interface", &self.interface())
            .field("member", &self.member())
            .finish()
    }
}

/// An incoming signal.
pub struct SignalMessage {
    message: MessageBuf,
}

impl SignalMessage {
    /// The object which emitted the signal.
    pub fn path(&self) -> &ObjectPath {
        match self.message.kind() {
            MessageKind::Signal { path, .. } => path,
            _ => unreachable!("Only constructed from a signal"),
        }
    }

    /// The name of the signal.
    pub fn member(&self) -> &str {
        match self.message.kind() {
            MessageKind::Signal { member, .. } => member,
            _ => unreachable!("Only constructed from a signal"),
        }
    }

    /// The interface the signal belongs to, if the sender named one.
    pub fn interface(&self) -> Option<&str> {
        self.message.interface()
    }

    /// The unique name of the sender.
    pub fn sender(&self) -> Option<&str> {
        self.message.sender()
    }

    /// The body of the signal, from which its arguments are read.
    pub fn body(&self) -> Body<'_> {
        self.message.body()
    }
}

impl fmt::Debug for SignalMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SignalMessage")
            .field("path", &self.path())
            .field("interface", &self.interface())
            .field("member", &self.member())
            .finish()
    }
}
