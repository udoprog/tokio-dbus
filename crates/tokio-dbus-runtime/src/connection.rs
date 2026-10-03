use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::time::{Instant, Sleep};

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

    /// Construct an argument list matching a signature which is known at
    /// compile time.
    ///
    /// This is the infallible form of [`new()`], for the common case where the
    /// signature comes out of [`Signature::new_const`] and has therefore
    /// already been validated at compile time.
    ///
    /// [`new()`]: Self::new
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::Signature;
    /// use tokio_dbus_runtime::Arguments;
    ///
    /// const SIGNATURE: &Signature = Signature::new_const(b"su");
    ///
    /// let mut arguments = Arguments::new_const(SIGNATURE);
    /// arguments.store("Hello World!");
    /// arguments.store(&42u32);
    /// ```
    pub fn new_const(signature: &'static Signature) -> Self {
        let mut buf = BodyBuf::new();

        // NB: Extending an empty buffer with an already validated signature
        // cannot fail, since the only failure is the combined signature growing
        // too long.
        buf.extend_signature(signature)
            .expect("A validated signature cannot fail to extend an empty body");

        Self { buf }
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
///
/// # Queued messages
///
/// Method calls and signals which arrive while waiting for a reply, or during
/// [`flush()`], are queued and returned by [`next()`] in arrival order. The
/// queue is not bounded, since dropping a call would leave its caller waiting
/// and dropping a signal would lose it silently. A connection which receives
/// calls or has match rules installed should therefore keep calling
/// [`next()`], or its memory grows with every message it does not handle.
/// Replies which nothing is waiting for anymore, such as one arriving after
/// its call timed out, are discarded instead of queued.
///
/// [`flush()`]: Self::flush
/// [`next()`]: Self::next
pub struct Connection {
    connection: tokio_dbus::Connection,
    buffers: Buffers,
    /// Messages which arrived while waiting for something else.
    queue: VecDeque<Incoming>,
    unique_name: String,
    /// How long to wait for the reply to a call before giving up.
    timeout: Option<Duration>,
    /// The timer driving call timeouts, created on the first timed call and
    /// reused for every one after that. See [`wait_for()`][Self::wait_for].
    sleep: Option<Pin<Box<Sleep>>>,
}

impl Connection {
    /// The default for how long a call waits for its reply, matching the 25
    /// seconds every other D-Bus implementation defaults to.
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(25);

    /// Connect to the session bus and say `Hello`.
    pub async fn session_bus() -> Result<Self> {
        Self::from_connection(tokio_dbus::Connection::session_bus()?).await
    }

    /// Connect to the system bus and say `Hello`.
    pub async fn system_bus() -> Result<Self> {
        Self::from_connection(tokio_dbus::Connection::system_bus()?).await
    }

    /// Take over a low level connection which has not said `Hello` yet, then
    /// connect it and say `Hello`.
    ///
    /// This is how a connection is set up over something other than the
    /// session or system bus, see [`ConnectionBuilder`].
    ///
    /// [`ConnectionBuilder`]: tokio_dbus::ConnectionBuilder
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::os::unix::net::UnixStream;
    ///
    /// use tokio_dbus::ConnectionBuilder;
    /// use tokio_dbus_runtime::Connection;
    ///
    /// # #[tokio::main] async fn main() -> tokio_dbus_runtime::Result<()> {
    /// let stream = UnixStream::connect("/run/my-bus/socket")?;
    /// let connection = ConnectionBuilder::new().build_with_stream(stream)?;
    /// let connection = Connection::from_connection(connection).await?;
    /// println!("{}", connection.unique_name());
    /// # Ok(()) }
    /// ```
    pub async fn from_connection(connection: tokio_dbus::Connection) -> Result<Self> {
        let mut this = Self {
            connection,
            buffers: Buffers::new(),
            queue: VecDeque::new(),
            unique_name: String::new(),
            timeout: Some(Self::DEFAULT_TIMEOUT),
            sleep: None,
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

    /// Set how long a call waits for its reply before failing, or `None` to
    /// wait forever.
    ///
    /// The default is [`DEFAULT_TIMEOUT`], since the bus does not time method
    /// calls out on its own, a peer which is alive but not reading its socket
    /// would otherwise hang the caller forever. The timeout applies to
    /// everything which waits for a reply, including [`call()`] and the name
    /// and match management methods.
    ///
    /// A call which times out fails with an error for which
    /// [`Error::is_timeout()`] is true and whose [`Error::name()`] is
    /// `org.freedesktop.DBus.Error.NoReply`. The connection itself remains
    /// usable, a reply which arrives after the deadline is discarded.
    ///
    /// The timeout is driven by the Tokio timer, which must be enabled on the
    /// runtime. `#[tokio::main]` enables it by default.
    ///
    /// [`DEFAULT_TIMEOUT`]: Self::DEFAULT_TIMEOUT
    /// [`call()`]: Self::call
    pub fn set_default_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }

    /// How long a call waits for its reply before failing, if limited.
    ///
    /// See [`set_default_timeout()`][Self::set_default_timeout].
    pub fn default_timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Call a method and wait for its reply.
    ///
    /// An error reply is turned into an [`Error`] carrying the name the remote
    /// end used.
    ///
    /// The call fails with a timeout error when no reply arrives within the
    /// configured deadline, see
    /// [`set_default_timeout()`][Self::set_default_timeout].
    ///
    /// # Cancellation
    ///
    /// This method is cancel safe. If the future is dropped before it
    /// completes, the call itself may still reach the peer, but the connection
    /// remains usable and a reply which arrives later is discarded rather than
    /// surfaced or confused with the reply to another call.
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

        let mut arguments = Arguments::new_const(Signature::STRING);
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

    /// Ask the bus to route [`NameOwnerChanged`] signals for `name` here.
    ///
    /// Watching a name is how a client survives its peer restarting: the
    /// signal announces both the name going away and it being claimed again.
    /// Decode the incoming signal with [`NameOwnerChanged::decode`], and pair
    /// this with [`name_owner()`] to learn the initial state, since the signal
    /// only reports changes.
    ///
    /// [`NameOwnerChanged`]: crate::NameOwnerChanged
    /// [`NameOwnerChanged::decode`]: crate::NameOwnerChanged::decode
    /// [`name_owner()`]: Self::name_owner
    pub async fn watch_name(&mut self, name: &str) -> Result<()> {
        self.add_match(&crate::NameOwnerChanged::rule(name)).await
    }

    /// Remove the interest registered by [`watch_name()`][Self::watch_name].
    pub async fn unwatch_name(&mut self, name: &str) -> Result<()> {
        self.remove_match(&crate::NameOwnerChanged::rule(name))
            .await
    }

    /// The unique name currently owning `name`, or `None` when the name has no
    /// owner.
    pub async fn name_owner(&mut self, name: &str) -> Result<Option<String>> {
        let mut arguments = Arguments::new_const(Signature::STRING);
        arguments.store(name);

        let result = self
            .call(
                org_freedesktop_dbus::DESTINATION,
                org_freedesktop_dbus::PATH,
                org_freedesktop_dbus::INTERFACE,
                "GetNameOwner",
                &arguments,
            )
            .await;

        match result {
            Ok(reply) => Ok(Some(reply.read::<String>()?)),
            Err(error) if error.name() == Some(org_freedesktop_dbus::NAME_HAS_NO_OWNER_ERROR) => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// Reply to a method call which no dispatcher recognised.
    ///
    /// The generated `dispatch` functions return `false` for a call which is
    /// not theirs, so that several interfaces can be served from one
    /// connection. Once every dispatcher has declined, this produces the
    /// standard `org.freedesktop.DBus.Error.UnknownMethod` reply leaving the
    /// call unanswered would leave the caller waiting for its timeout instead.
    pub fn reply_unknown_method(&mut self, call: &Call) -> Result<()> {
        self.reply_error(
            call,
            &Error::remote(
                org_freedesktop_dbus::UNKNOWN_METHOD_ERROR,
                format_args!(
                    "No such method: {}.{}",
                    call.interface().unwrap_or_default(),
                    call.member()
                ),
            ),
        )
    }

    /// Write out everything which has been buffered for sending.
    ///
    /// This is only needed before dropping the connection, since [`next()`] and
    /// [`call()`] both drive writes as a side effect. A message which arrives
    /// while flushing is queued for [`next()`].
    ///
    /// [`next()`]: Self::next
    /// [`call()`]: Self::call
    pub async fn flush(&mut self) -> Result<()> {
        self.connection.flush(&mut self.buffers).await?;

        // NB: Every message handed out is cleared from the receive buffer, so
        // one which is left was received during the flush.
        if self.buffers.recv.has_message() {
            let message = take_message(&mut self.buffers)?;
            self.queue.extend(Incoming::new(message));
        }

        Ok(())
    }

    /// Wait for the next method call or signal directed at this connection.
    pub async fn next(&mut self) -> Result<Incoming> {
        if let Some(incoming) = self.queue.pop_front() {
            return Ok(incoming);
        }

        loop {
            self.connection.wait(&mut self.buffers).await?;

            if let Some(incoming) = Incoming::new(take_message(&mut self.buffers)?) {
                return Ok(incoming);
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn queued(&self) -> usize {
        self.queue.len()
    }

    /// Wait for the reply with the given serial, applying the configured
    /// timeout.
    async fn wait_for(&mut self, serial: Serial) -> Result<MessageBuf> {
        let Self {
            connection,
            buffers,
            queue,
            timeout,
            sleep,
            ..
        } = self;

        let future = pin!(drive_until_reply(connection, buffers, queue, serial));

        let Some(timeout) = *timeout else {
            return future.await;
        };

        let deadline = Instant::now() + timeout;

        // The timer is created on the first timed call and reset for each one
        // after that, and is deliberately never cancelled: resetting a timer
        // which is still registered with the runtime to a later deadline is a
        // lock-free store, where registering a fresh one locks the timer
        // wheel. A deadline which fires with no call outstanding wakes the
        // last caller once, spuriously and harmlessly.
        let sleep = match sleep {
            Some(sleep) => {
                sleep.as_mut().reset(deadline);
                sleep
            }
            sleep => sleep.insert(Box::pin(tokio::time::sleep_until(deadline))),
        };

        Timed {
            future,
            sleep: sleep.as_mut(),
            timeout,
        }
        .await
    }
}

/// Drive the connection until the reply with the given serial arrives,
/// queueing everything else which shows up in the meantime.
///
/// This is a function over the fields it needs rather than a method, so that
/// the timer of the connection stays borrowable next to it.
async fn drive_until_reply(
    connection: &mut tokio_dbus::Connection,
    buffers: &mut Buffers,
    queue: &mut VecDeque<Incoming>,
    serial: Serial,
) -> Result<MessageBuf> {
    loop {
        connection.wait(buffers).await?;
        let message = take_message(buffers)?;

        match message.kind() {
            MessageKind::MethodReturn { reply_serial } if reply_serial == serial => {
                return Ok(message);
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } if reply_serial == serial => {
                let text = message.body().read::<str>().unwrap_or_default();
                return Err(Error::remote(error_name, text));
            }
            _ => {
                queue.extend(Incoming::new(message));
            }
        }
    }
}

/// Copy the received message out and clear it from the receive buffer, so that
/// a later flush cannot mistake it for one which arrived during the flush.
fn take_message(buffers: &mut Buffers) -> Result<MessageBuf> {
    let message = buffers.recv.last_message()?.to_owned();
    buffers.recv.clear();
    Ok(message)
}

/// A future bounded by the reply deadline of the connection.
///
/// This is `tokio::time::timeout` with the timer borrowed rather than owned, so
/// that however the wait ends, completion, cancellation or an unwinding panic,
/// the timer stays in the connection for the next call to reuse.
struct Timed<'a, F> {
    future: Pin<&'a mut F>,
    sleep: Pin<&'a mut Sleep>,
    timeout: Duration,
}

impl<T, F> Future for Timed<'_, F>
where
    F: Future<Output = Result<T>>,
{
    type Output = Result<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // NB: The future is polled first so that a reply which is ready wins
        // over a deadline which elapsed while waiting.
        if let Poll::Ready(result) = self.future.as_mut().poll(cx) {
            return Poll::Ready(result);
        }

        if self.sleep.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(Error::new(ErrorKind::Timeout(self.timeout))));
        }

        Poll::Pending
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
