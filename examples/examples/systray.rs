//! A systray icon with a menu, implemented on top of the two interfaces that
//! Linux desktops actually speak for this: `org.kde.StatusNotifierItem` (SNI)
//! and `com.canonical.dbusmenu`.
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example systray
//! ```
//!
//! # Desktop support
//!
//! KDE Plasma implements a status notifier host natively, so the icon shows up
//! with no further setup.
//!
//! GNOME Shell does *not* ship a status notifier host. The icon only shows up
//! there if the "AppIndicator and KStatusNotifierItem Support" extension is
//! installed and enabled. Everything on the wire is the same either way.
//!
//! Most other trays (Waybar, swaybar, xfce4-statusnotifier-plugin, ...) speak
//! the same two interfaces.
//!
//! # Layout of this example
//!
//! * The item lives at `/StatusNotifierItem` and holds the icon, the tooltip
//!   and the path of the menu.
//! * The menu lives at `/MenuBar` and serves a tree of items, each of which is a
//!   bag of properties keyed by name.
//! * Both objects answer `org.freedesktop.DBus.Properties` and
//!   `org.freedesktop.DBus.Introspectable`, which is how hosts discover them.
//!
//! Once the well known name has been acquired, the item is registered with
//! `org.kde.StatusNotifierWatcher`. The watcher may not be running yet, or may
//! restart later, so `NameOwnerChanged` is watched to re-register.

use std::fmt;

use anyhow::{Result, bail};
use tokio_dbus::org_freedesktop_dbus::{self, NameFlag, NameReply};
use tokio_dbus::{
    BodyBuf, Buffers, Connection, Message, MessageKind, ObjectPath, SendBuf, Serial, Signature,
    StoreArray, StoreStruct, StoreVariant, Variant, ty,
};

/// The interface implemented by the status notifier item itself.
const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";
/// The path the item is served from. Hosts locate it through the watcher, so
/// this is only a convention.
const ITEM_PATH: &ObjectPath = ObjectPath::new_const(b"/StatusNotifierItem");

/// The interface implemented by the menu.
const MENU_INTERFACE: &str = "com.canonical.dbusmenu";
/// The path the menu is served from. Hosts learn about it through the `Menu`
/// property of the item.
const MENU_PATH: &ObjectPath = ObjectPath::new_const(b"/MenuBar");

/// The service which keeps track of all status notifier items on the bus.
const WATCHER_NAME: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_INTERFACE: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &ObjectPath = ObjectPath::new_const(b"/StatusNotifierWatcher");

/// The signature of a single node in a menu layout. Note that it is recursive:
/// the `av` at the end holds variants which contain this very type again.
const LAYOUT: &Signature = Signature::new_const(b"(ia{sv}av)");
/// The signature of a list of icons, each of which is a width, a height and
/// ARGB32 pixel data in network byte order.
const PIXMAPS: &Signature = Signature::new_const(b"a(iiay)");
/// The signature of a tooltip: icon name, icon data, title and description.
const TOOL_TIP: &Signature = Signature::new_const(b"(sa(iiay)ss)");
const STRINGS: &Signature = Signature::new_const(b"as");

/// The properties of a menu item, `a{sv}`.
type Properties = ty::Array<ty::Dict<ty::Str, ty::Variant>>;
/// The fields of a single node in a menu layout, matching [`LAYOUT`].
type Layout = (i32, Properties, ty::Array<ty::Variant>);
/// The fields of a single icon, matching an element of [`PIXMAPS`].
type Pixmap = (i32, i32, ty::Array<u8>);

/// Item identifiers. The root of the menu is required to be `0`.
const ROOT: i32 = 0;
const HELLO: i32 = 1;
const THEMED_ICON: i32 = 3;
const ATTENTION: i32 = 4;
const LIGHT: i32 = 6;
const DARK: i32 = 7;
const AUTO: i32 = 8;
const QUIT: i32 = 10;

/// The radio items which make up the one radio group in this menu.
const THEMES: [i32; 3] = [LIGHT, DARK, AUTO];

#[tokio::main]
async fn main() -> Result<()> {
    let mut buf = Buffers::new();
    let mut c = Connection::session_bus()?;
    c.connect(&mut buf).await?;

    let mut app = Systray::new();

    let hello = buf.hello()?;
    let name = buf.request_name(&app.name, NameFlag::DO_NOT_QUEUE)?;

    // Without a match rule the bus only delivers the signals which are directed
    // at us. We want to know when the watcher appears or restarts.
    buf.add_match(&format!(
        "type='signal',sender='{DBUS}',interface='{DBUS}',member='NameOwnerChanged',arg0='{WATCHER_NAME}'",
        DBUS = org_freedesktop_dbus::DESTINATION,
    ))?;

    let mut register = Some(app.register(&mut buf)?);

    while app.running {
        c.wait(&mut buf).await?;

        let message = buf.recv.last_message()?;

        match message.kind() {
            MessageKind::MethodReturn { reply_serial } if reply_serial == hello => {
                println!("Connected as {}", message.body().read::<str>()?);
            }
            MessageKind::MethodReturn { reply_serial } if reply_serial == name => {
                match message.body().load::<NameReply>()? {
                    NameReply::PRIMARY_OWNER => println!("Acquired {}", app.name),
                    reply => bail!("Could not acquire {}: {reply:?}", app.name),
                }
            }
            MessageKind::MethodReturn { reply_serial } if Some(reply_serial) == register => {
                register = None;
                println!("Registered with {WATCHER_NAME}");
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } if Some(reply_serial) == register => {
                // The watcher is not running yet. Registration is retried once
                // it shows up on the bus.
                register = None;
                println!("Waiting for {WATCHER_NAME} ({error_name})");
            }
            MessageKind::Error {
                error_name,
                reply_serial,
            } => {
                let message = message.body().read::<str>()?;
                bail!("{error_name}: {reply_serial}: {message}");
            }
            MessageKind::Signal { member, .. }
                if message.interface() == Some(org_freedesktop_dbus::INTERFACE) =>
            {
                if member == "NameOwnerChanged" {
                    let mut body = message.body();
                    let name = body.read::<str>()?;
                    let _old_owner = body.read::<str>()?;
                    let new_owner = body.read::<str>()?;

                    if name == WATCHER_NAME && !new_owner.is_empty() {
                        register = Some(app.register(&mut buf)?);
                    }
                }
            }
            MessageKind::MethodCall { path, member } => {
                buf.body.clear();

                let m = match app.call(&message, path, member, &mut buf.send, &mut buf.body) {
                    Ok(m) => m,
                    Err(error) => {
                        // The handler may have buffered part of a reply before
                        // running into the error.
                        buf.body.clear();
                        buf.body.store(error.to_string())?;
                        message
                            .error(error.name, buf.send.next_serial())
                            .with_body(&buf.body)
                    }
                };

                buf.send.write_message(m)?;
                app.flush(&mut buf.send, &mut buf.body)?;
            }
            _ => {}
        }
    }

    // Give the bus a chance to see the name go away before the socket closes.
    buf.release_name(&app.name)?;
    c.flush(&mut buf).await?;
    Ok(())
}

/// The state of the systray icon.
struct Systray {
    /// The well known name this item is registered under.
    name: String,
    /// One of `Passive`, `Active` or `NeedsAttention`.
    status: &'static str,
    /// Set when the themed icon is preferred over the generated pixmap.
    themed: bool,
    /// The generated icon. A list, because the spec allows an item to offer the
    /// same icon at several sizes.
    icon: Vec<(i32, i32, Vec<u8>)>,
    /// Bumped every time the layout changes, so hosts can tell whether the copy
    /// they hold is stale.
    revision: u32,
    /// The menu, rooted at the item with id [`ROOT`].
    root: Item,
    /// Signals which are emitted once the reply to the current call has been
    /// written.
    pending: Vec<Signal>,
    running: bool,
}

impl Systray {
    fn new() -> Self {
        Self {
            // Hosts which predate object path registration look the item up by
            // this name, so the shape of it matters.
            name: format!("org.kde.StatusNotifierItem-{}-1", std::process::id()),
            status: "Active",
            themed: false,
            icon: vec![icon()],
            revision: 1,
            root: Item::menu(),
            pending: Vec::new(),
            running: true,
        }
    }

    /// Ask the watcher to start tracking this item.
    fn register(&self, buf: &mut Buffers) -> Result<Serial> {
        buf.body.clear();
        buf.body.store(self.name.as_str())?;

        let m = buf
            .send
            .method_call(WATCHER_PATH, "RegisterStatusNotifierItem")
            .with_destination(WATCHER_NAME)
            .with_interface(WATCHER_INTERFACE)
            .with_body(&buf.body);

        let serial = m.serial();
        buf.send.write_message(m)?;
        Ok(serial)
    }

    /// Dispatch an incoming method call.
    fn call<'a>(
        &mut self,
        msg: &Message<'a>,
        path: &'a ObjectPath,
        member: &'a str,
        send: &mut SendBuf,
        body: &'a mut BodyBuf,
    ) -> Result<Message<'a>, CallError> {
        let interface = msg.interface().unwrap_or_default();

        if path != ITEM_PATH && path != MENU_PATH {
            return Err(CallError::new(
                org_freedesktop_dbus::UNKNOWN_OBJECT_ERROR,
                format_args!("No such object: {path}"),
            ));
        }

        if interface == org_freedesktop_dbus::INTROSPECTABLE_INTERFACE && member == "Introspect" {
            let xml = if path == ITEM_PATH {
                ITEM_XML
            } else {
                MENU_XML
            };
            body.store(xml)?;
            return Ok(msg.method_return(send.next_serial()).with_body(body));
        }

        if interface == org_freedesktop_dbus::PEER_INTERFACE && member == "Ping" {
            return Ok(msg.method_return(send.next_serial()));
        }

        if interface == org_freedesktop_dbus::PROPERTIES_INTERFACE {
            return self.properties(msg, path, member, send, body);
        }

        if path == ITEM_PATH && interface == ITEM_INTERFACE {
            return self.item(msg, member, send);
        }

        if path == MENU_PATH && interface == MENU_INTERFACE {
            return self.menu(msg, member, send, body);
        }

        Err(CallError::new(
            org_freedesktop_dbus::UNKNOWN_INTERFACE_ERROR,
            format_args!("No such interface on {path}: {interface}"),
        ))
    }

    /// Handle `org.freedesktop.DBus.Properties`.
    fn properties<'a>(
        &mut self,
        msg: &Message<'a>,
        path: &'a ObjectPath,
        member: &'a str,
        send: &mut SendBuf,
        body: &'a mut BodyBuf,
    ) -> Result<Message<'a>, CallError> {
        let mut read = msg.body();

        match member {
            "Get" => {
                let _interface = read.read::<str>()?;
                let name = read.read::<str>()?;

                let Some(value) = self.property(path, name) else {
                    return Err(CallError::new(
                        org_freedesktop_dbus::UNKNOWN_PROPERTY_ERROR,
                        format_args!("No such property: {name}"),
                    ));
                };

                value.store(body.store_variant(value.signature())?);
            }
            "GetAll" => {
                let names = if path == ITEM_PATH {
                    ITEM_PROPERTIES
                } else {
                    MENU_PROPERTIES
                };

                let mut dict = body.store_array::<ty::Dict<ty::Str, ty::Variant>>()?;

                for name in names {
                    let Some(value) = self.property(path, name) else {
                        continue;
                    };

                    dict.store_entry()
                        .store(*name)
                        .store_variant(value.signature(), |w| value.store(w))
                        .finish();
                }

                dict.finish();
            }
            "Set" => {
                return Err(CallError::new(
                    org_freedesktop_dbus::FAILED_ERROR,
                    format_args!("All properties on {path} are read-only"),
                ));
            }
            member => return Err(CallError::unknown_method(member)),
        }

        Ok(msg.method_return(send.next_serial()).with_body(body))
    }

    /// The value of a property on one of the two objects we serve.
    fn property(&self, path: &ObjectPath, name: &str) -> Option<Value<'_>> {
        if path == MENU_PATH {
            return Some(match name {
                // 3 is the version which introduced `EventGroup` and
                // `AboutToShowGroup`, both of which are implemented below.
                "Version" => Value::U32(3),
                "TextDirection" => Value::Str("ltr"),
                "Status" => Value::Str("normal"),
                "IconThemePath" => Value::Strings(&[]),
                _ => return None,
            });
        }

        Some(match name {
            "Category" => Value::Str("ApplicationStatus"),
            "Id" => Value::Str("tokio-dbus-systray"),
            "Title" => Value::Str("tokio-dbus systray example"),
            "Status" => Value::Str(self.status),
            // Only meaningful for items which are backed by an X11 window.
            "WindowId" => Value::I32(0),
            "IconName" => Value::Str(if self.themed {
                "dialog-information"
            } else {
                ""
            }),
            // Hosts prefer `IconName` when it is set, which is what the "Use
            // themed icon" entry in the menu toggles between.
            "IconPixmap" => Value::Pixmaps(if self.themed { &[] } else { &self.icon }),
            "OverlayIconName" | "AttentionIconName" | "AttentionMovieName" => Value::Str(""),
            "OverlayIconPixmap" | "AttentionIconPixmap" => Value::Pixmaps(&[]),
            "ToolTip" => Value::ToolTip("tokio-dbus", "A systray example"),
            // When this is set the host opens the menu on left click instead of
            // calling `Activate`.
            "ItemIsMenu" => Value::Bool(false),
            "Menu" => Value::Path(MENU_PATH),
            _ => return None,
        })
    }

    /// Handle `org.kde.StatusNotifierItem`.
    ///
    /// Every method here replies with an empty body, so unlike the other
    /// handlers this one has no use for a [`BodyBuf`].
    fn item<'a>(
        &mut self,
        msg: &Message<'a>,
        member: &'a str,
        send: &mut SendBuf,
    ) -> Result<Message<'a>, CallError> {
        let mut read = msg.body();

        match member {
            "Activate" | "SecondaryActivate" | "ContextMenu" => {
                let x = read.load::<i32>()?;
                let y = read.load::<i32>()?;
                println!("{member} at ({x}, {y})");
            }
            "Scroll" => {
                let delta = read.load::<i32>()?;
                let orientation = read.read::<str>()?;
                println!("Scroll {delta} {orientation}");
            }
            "ProvideXdgActivationToken" => {
                let _token = read.read::<str>()?;
            }
            member => return Err(CallError::unknown_method(member)),
        }

        Ok(msg.method_return(send.next_serial()))
    }

    /// Handle `com.canonical.dbusmenu`.
    fn menu<'a>(
        &mut self,
        msg: &Message<'a>,
        member: &'a str,
        send: &mut SendBuf,
        body: &'a mut BodyBuf,
    ) -> Result<Message<'a>, CallError> {
        let mut read = msg.body();

        match member {
            // (in i parentId, in i recursionDepth, in as propertyNames,
            //  out u revision, out (ia{sv}av) layout)
            "GetLayout" => {
                let parent = read.load::<i32>()?;
                let depth = read.load::<i32>()?;
                let filter = strings(&mut read)?;

                let Some(item) = self.root.find(parent) else {
                    return Err(CallError::no_such_item(parent));
                };

                body.store(self.revision)?;
                store_layout(body.store_struct::<Layout>()?, item, depth, &filter);
            }
            // (in ai ids, in as propertyNames, out a(ia{sv}) properties)
            "GetGroupProperties" => {
                let mut ids = Vec::new();
                let mut read_ids = read.load_array::<i32>()?;

                while let Some(id) = read_ids.load()? {
                    ids.push(id);
                }

                let filter = strings(&mut read)?;
                let mut array = body.store_array::<(i32, Properties)>()?;

                for id in ids {
                    let Some(item) = self.root.find(id) else {
                        continue;
                    };

                    array
                        .store_struct()
                        .store(id)
                        .store_array(|w| store_properties(w, item, &filter))
                        .finish();
                }

                array.finish();
            }
            // (in i id, in s name, out v value)
            "GetProperty" => {
                let id = read.load::<i32>()?;
                let name = read.read::<str>()?;

                let Some(item) = self.root.find(id) else {
                    return Err(CallError::no_such_item(id));
                };

                let Some((_, value)) = item.properties().into_iter().find(|(n, _)| *n == name)
                else {
                    return Err(CallError::new(
                        org_freedesktop_dbus::UNKNOWN_PROPERTY_ERROR,
                        format_args!("No such property on {id}: {name}"),
                    ));
                };

                // A `Variant` already knows how to write its own signature, so
                // unlike the container values above it is stored directly.
                body.store(value)?;
            }
            // (in i id, in s eventId, in v data, in u timestamp)
            "Event" => {
                let id = read.load::<i32>()?;
                let event = read.read::<str>()?;
                // The payload is only meaningful for a handful of event types,
                // none of which this menu uses.
                read.skip_variant()?;
                let _timestamp = read.load::<u32>()?;
                self.event(id, event);
            }
            // (in a(isvu) events, out ai idErrors)
            "EventGroup" => {
                let mut events = read.load_array::<(i32, ty::Str, ty::Variant, u32)>()?;

                while let Some((id, event)) = events.load_with(|b| {
                    b.load_struct_with(|b| {
                        let id = b.load::<i32>()?;
                        let event = b.read::<str>()?;
                        b.skip_variant()?;
                        let _timestamp = b.load::<u32>()?;
                        Ok((id, event))
                    })
                })? {
                    self.event(id, event);
                }

                body.store_array::<i32>()?.finish();
            }
            // (in i id, out b needUpdate)
            "AboutToShow" => {
                let _id = read.load::<i32>()?;
                // The menu is built up front, so it never needs to be refreshed
                // before being shown.
                body.store(false)?;
            }
            // (in ai ids, out ai updatesNeeded, out ai idErrors)
            "AboutToShowGroup" => {
                body.store_array::<i32>()?.finish();
                body.store_array::<i32>()?.finish();
            }
            member => return Err(CallError::unknown_method(member)),
        }

        Ok(msg.method_return(send.next_serial()).with_body(body))
    }

    /// Act on a menu item being clicked.
    fn event(&mut self, id: i32, event: &str) {
        if event != "clicked" {
            return;
        }

        match id {
            HELLO => println!("Hello from the tokio-dbus systray example!"),
            QUIT => {
                println!("Quitting");
                self.running = false;
            }
            THEMED_ICON => {
                self.themed = !self.themed;
                self.set_toggle(THEMED_ICON, self.themed);
                self.pending.push(Signal::NewIcon);
                self.pending.push(Signal::Updated(vec![THEMED_ICON]));
            }
            ATTENTION => {
                let attention = self.status == "Active";
                self.status = if attention {
                    "NeedsAttention"
                } else {
                    "Active"
                };
                self.set_toggle(ATTENTION, attention);
                self.pending.push(Signal::NewStatus);
                self.pending.push(Signal::Updated(vec![ATTENTION]));
            }
            id if THEMES.contains(&id) => {
                for theme in THEMES {
                    self.set_toggle(theme, theme == id);
                }

                println!("Selected theme {id}");
                self.pending.push(Signal::Updated(THEMES.to_vec()));
            }
            _ => {}
        }
    }

    fn set_toggle(&mut self, id: i32, value: bool) {
        if let Some(item) = self.root.find_mut(id) {
            match &mut item.kind {
                Kind::Check { checked } | Kind::Radio { checked } => *checked = value,
                _ => {}
            }
        }
    }

    /// Emit the signals queued while handling a call.
    fn flush(&mut self, send: &mut SendBuf, body: &mut BodyBuf) -> Result<()> {
        for signal in std::mem::take(&mut self.pending) {
            body.clear();

            let m = match &signal {
                Signal::NewIcon => send
                    .signal(ITEM_PATH, "NewIcon")
                    .with_interface(ITEM_INTERFACE),
                Signal::NewStatus => {
                    body.store(self.status)?;

                    send.signal(ITEM_PATH, "NewStatus")
                        .with_interface(ITEM_INTERFACE)
                        .with_body(&*body)
                }
                Signal::Updated(ids) => {
                    // (a(ia{sv}) updatedProps, a(ias) removedProps)
                    let mut updated = body.store_array::<(i32, Properties)>()?;

                    for &id in ids {
                        let Some(item) = self.root.find(id) else {
                            continue;
                        };

                        updated
                            .store_struct()
                            .store(id)
                            .store_array(|w| store_properties(w, item, &[]))
                            .finish();
                    }

                    updated.finish();
                    body.store_array::<(i32, ty::Array<ty::Str>)>()?.finish();

                    send.signal(MENU_PATH, "ItemsPropertiesUpdated")
                        .with_interface(MENU_INTERFACE)
                        .with_body(&*body)
                }
            };

            send.write_message(m)?;
        }

        Ok(())
    }
}

/// A signal which is emitted once the current call has been replied to.
enum Signal {
    NewIcon,
    NewStatus,
    Updated(Vec<i32>),
}

/// A property value of one of the objects we serve.
///
/// The basic types could be represented by [`Variant`] directly, but the icons
/// and the tooltip are containers, which is what [`StoreVariant`] is for.
enum Value<'a> {
    Str(&'a str),
    Path(&'a ObjectPath),
    I32(i32),
    U32(u32),
    Bool(bool),
    Strings(&'a [&'a str]),
    Pixmaps(&'a [(i32, i32, Vec<u8>)]),
    ToolTip(&'a str, &'a str),
}

impl Value<'_> {
    /// The signature of the value, which is written into the variant ahead of
    /// the value itself.
    fn signature(&self) -> &'static Signature {
        match self {
            Value::Str(..) => Signature::STRING,
            Value::Path(..) => Signature::OBJECT_PATH,
            Value::I32(..) => Signature::INT32,
            Value::U32(..) => Signature::UINT32,
            Value::Bool(..) => Signature::BOOLEAN,
            Value::Strings(..) => STRINGS,
            Value::Pixmaps(..) => PIXMAPS,
            Value::ToolTip(..) => TOOL_TIP,
        }
    }

    fn store(&self, w: StoreVariant<'_>) {
        match *self {
            Value::Str(value) => w.store(value),
            Value::Path(value) => w.store(value),
            Value::I32(value) => w.store(value),
            Value::U32(value) => w.store(value),
            Value::Bool(value) => w.store(value),
            Value::Strings(values) => {
                let mut array = w.store_array::<ty::Str>();

                for value in values {
                    array.store(value);
                }
            }
            Value::Pixmaps(pixmaps) => {
                let mut array = w.store_array::<Pixmap>();
                store_pixmaps(&mut array, pixmaps);
            }
            Value::ToolTip(title, description) => {
                w.store_struct::<(ty::Str, ty::Array<Pixmap>, ty::Str, ty::Str)>()
                    .store("")
                    .store_array(|w| store_pixmaps(w, &[]))
                    .store(title)
                    .store(description)
                    .finish();
            }
        }
    }
}

fn store_pixmaps(array: &mut StoreArray<'_, Pixmap>, pixmaps: &[(i32, i32, Vec<u8>)]) {
    for (width, height, data) in pixmaps {
        array
            .store_struct()
            .store(*width)
            .store(*height)
            .store_array(|w| w.write_slice(data))
            .finish();
    }
}

/// A node in the menu.
struct Item {
    id: i32,
    label: String,
    kind: Kind,
    children: Vec<Item>,
}

enum Kind {
    Standard,
    Separator,
    Check { checked: bool },
    Radio { checked: bool },
}

impl Item {
    fn new(id: i32, label: &str, kind: Kind) -> Self {
        Self {
            id,
            label: label.to_owned(),
            kind,
            children: Vec::new(),
        }
    }

    fn separator(id: i32) -> Self {
        Self::new(id, "", Kind::Separator)
    }

    fn with(mut self, children: impl IntoIterator<Item = Item>) -> Self {
        self.children.extend(children);
        self
    }

    /// The menu served by this example.
    fn menu() -> Self {
        Self::new(ROOT, "", Kind::Standard).with([
            Item::new(HELLO, "Say hello", Kind::Standard),
            Item::separator(2),
            Item::new(
                THEMED_ICON,
                "Use themed icon",
                Kind::Check { checked: false },
            ),
            Item::new(ATTENTION, "Needs attention", Kind::Check { checked: false }),
            Item::new(5, "Theme", Kind::Standard).with([
                Item::new(LIGHT, "Light", Kind::Radio { checked: true }),
                Item::new(DARK, "Dark", Kind::Radio { checked: false }),
                Item::new(AUTO, "Follow system", Kind::Radio { checked: false }),
            ]),
            Item::separator(9),
            Item::new(QUIT, "Quit", Kind::Standard),
        ])
    }

    fn find(&self, id: i32) -> Option<&Item> {
        if self.id == id {
            return Some(self);
        }

        self.children.iter().find_map(|c| c.find(id))
    }

    fn find_mut(&mut self, id: i32) -> Option<&mut Item> {
        if self.id == id {
            return Some(self);
        }

        self.children.iter_mut().find_map(|c| c.find_mut(id))
    }

    /// The properties a host reads to render this item.
    ///
    /// Only properties which differ from their default have to be sent, which is
    /// why for instance `enabled` is absent here.
    fn properties(&self) -> Vec<(&'static str, Variant<'_>)> {
        let mut out = Vec::new();

        if let Kind::Separator = self.kind {
            out.push(("type", Variant::String("separator")));
            return out;
        }

        out.push(("label", Variant::String(&self.label)));

        match self.kind {
            Kind::Check { checked } => {
                out.push(("toggle-type", Variant::String("checkmark")));
                out.push(("toggle-state", Variant::I32(i32::from(checked))));
            }
            Kind::Radio { checked } => {
                out.push(("toggle-type", Variant::String("radio")));
                out.push(("toggle-state", Variant::I32(i32::from(checked))));
            }
            _ => {}
        }

        if !self.children.is_empty() {
            out.push(("children-display", Variant::String("submenu")));
        }

        out
    }
}

/// Write one node of a menu layout, recursing into its children.
///
/// A `recursionDepth` of `-1` means "the whole subtree", which is why the depth
/// is compared against zero rather than counted down to it.
fn store_layout(w: StoreStruct<'_, Layout>, item: &Item, depth: i32, filter: &[&str]) {
    w.store(item.id)
        .store_array(|w| store_properties(w, item, filter))
        .store_array(|children| {
            if depth != 0 {
                for child in &item.children {
                    store_layout(
                        children.store_variant(LAYOUT).store_struct::<Layout>(),
                        child,
                        depth - 1,
                        filter,
                    );
                }
            }
        })
        .finish();
}

/// Write the `a{sv}` of properties for an item.
///
/// An empty filter means that the caller wants all of them.
fn store_properties(
    w: &mut StoreArray<'_, ty::Dict<ty::Str, ty::Variant>>,
    item: &Item,
    filter: &[&str],
) {
    for (name, value) in item.properties() {
        if !filter.is_empty() && !filter.contains(&name) {
            continue;
        }

        w.store_entry().store(name).store(value).finish();
    }
}

/// Read an `as` argument.
fn strings<'a>(body: &mut tokio_dbus::Body<'a>) -> Result<Vec<&'a str>, tokio_dbus::Error> {
    let mut array = body.load_array::<ty::Str>()?;
    let mut out = Vec::new();

    while let Some(string) = array.read()? {
        out.push(string);
    }

    Ok(out)
}

/// Generate a 22x22 icon, which is ARGB32 in network byte order as the spec
/// requires.
fn icon() -> (i32, i32, Vec<u8>) {
    const SIZE: i32 = 22;

    let mut data = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    let center = (SIZE - 1) as f32 / 2.0;

    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let distance = (dx * dx + dy * dy).sqrt();
            let alpha = ((center - distance + 0.5).clamp(0.0, 1.0) * 255.0) as u8;
            data.extend_from_slice(&[alpha, 0x2f, 0x81, 0xf7]);
        }
    }

    (SIZE, SIZE, data)
}

/// An error which is turned into an error reply.
struct CallError {
    name: &'static str,
    message: String,
}

impl CallError {
    fn new(name: &'static str, message: impl fmt::Display) -> Self {
        Self {
            name,
            message: message.to_string(),
        }
    }

    fn unknown_method(member: &str) -> Self {
        Self::new(
            org_freedesktop_dbus::UNKNOWN_METHOD_ERROR,
            format_args!("No such method: {member}"),
        )
    }

    fn no_such_item(id: i32) -> Self {
        Self::new(
            org_freedesktop_dbus::INVALID_ARGS_ERROR,
            format_args!("No such menu item: {id}"),
        )
    }
}

impl From<tokio_dbus::Error> for CallError {
    fn from(error: tokio_dbus::Error) -> Self {
        Self::new(org_freedesktop_dbus::FAILED_ERROR, error)
    }
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

const ITEM_XML: &str = r#"<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN" "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
<node>
  <interface name="org.freedesktop.DBus.Introspectable">
    <method name="Introspect"><arg name="xml" type="s" direction="out"/></method>
  </interface>
  <interface name="org.freedesktop.DBus.Properties">
    <method name="Get">
      <arg name="interface" type="s" direction="in"/>
      <arg name="name" type="s" direction="in"/>
      <arg name="value" type="v" direction="out"/>
    </method>
    <method name="GetAll">
      <arg name="interface" type="s" direction="in"/>
      <arg name="properties" type="a{sv}" direction="out"/>
    </method>
  </interface>
  <interface name="org.kde.StatusNotifierItem">
    <property name="Category" type="s" access="read"/>
    <property name="Id" type="s" access="read"/>
    <property name="Title" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="WindowId" type="i" access="read"/>
    <property name="IconName" type="s" access="read"/>
    <property name="IconPixmap" type="a(iiay)" access="read"/>
    <property name="OverlayIconName" type="s" access="read"/>
    <property name="OverlayIconPixmap" type="a(iiay)" access="read"/>
    <property name="AttentionIconName" type="s" access="read"/>
    <property name="AttentionIconPixmap" type="a(iiay)" access="read"/>
    <property name="AttentionMovieName" type="s" access="read"/>
    <property name="ToolTip" type="(sa(iiay)ss)" access="read"/>
    <property name="ItemIsMenu" type="b" access="read"/>
    <property name="Menu" type="o" access="read"/>
    <method name="ContextMenu">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="Activate">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="SecondaryActivate">
      <arg name="x" type="i" direction="in"/>
      <arg name="y" type="i" direction="in"/>
    </method>
    <method name="Scroll">
      <arg name="delta" type="i" direction="in"/>
      <arg name="orientation" type="s" direction="in"/>
    </method>
    <signal name="NewTitle"/>
    <signal name="NewIcon"/>
    <signal name="NewAttentionIcon"/>
    <signal name="NewOverlayIcon"/>
    <signal name="NewToolTip"/>
    <signal name="NewStatus"><arg name="status" type="s"/></signal>
  </interface>
</node>"#;

const MENU_XML: &str = r#"<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN" "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
<node>
  <interface name="org.freedesktop.DBus.Introspectable">
    <method name="Introspect"><arg name="xml" type="s" direction="out"/></method>
  </interface>
  <interface name="org.freedesktop.DBus.Properties">
    <method name="Get">
      <arg name="interface" type="s" direction="in"/>
      <arg name="name" type="s" direction="in"/>
      <arg name="value" type="v" direction="out"/>
    </method>
    <method name="GetAll">
      <arg name="interface" type="s" direction="in"/>
      <arg name="properties" type="a{sv}" direction="out"/>
    </method>
  </interface>
  <interface name="com.canonical.dbusmenu">
    <property name="Version" type="u" access="read"/>
    <property name="TextDirection" type="s" access="read"/>
    <property name="Status" type="s" access="read"/>
    <property name="IconThemePath" type="as" access="read"/>
    <method name="GetLayout">
      <arg name="parentId" type="i" direction="in"/>
      <arg name="recursionDepth" type="i" direction="in"/>
      <arg name="propertyNames" type="as" direction="in"/>
      <arg name="revision" type="u" direction="out"/>
      <arg name="layout" type="(ia{sv}av)" direction="out"/>
    </method>
    <method name="GetGroupProperties">
      <arg name="ids" type="ai" direction="in"/>
      <arg name="propertyNames" type="as" direction="in"/>
      <arg name="properties" type="a(ia{sv})" direction="out"/>
    </method>
    <method name="GetProperty">
      <arg name="id" type="i" direction="in"/>
      <arg name="name" type="s" direction="in"/>
      <arg name="value" type="v" direction="out"/>
    </method>
    <method name="Event">
      <arg name="id" type="i" direction="in"/>
      <arg name="eventId" type="s" direction="in"/>
      <arg name="data" type="v" direction="in"/>
      <arg name="timestamp" type="u" direction="in"/>
    </method>
    <method name="EventGroup">
      <arg name="events" type="a(isvu)" direction="in"/>
      <arg name="idErrors" type="ai" direction="out"/>
    </method>
    <method name="AboutToShow">
      <arg name="id" type="i" direction="in"/>
      <arg name="needUpdate" type="b" direction="out"/>
    </method>
    <method name="AboutToShowGroup">
      <arg name="ids" type="ai" direction="in"/>
      <arg name="updatesNeeded" type="ai" direction="out"/>
      <arg name="idErrors" type="ai" direction="out"/>
    </method>
    <signal name="ItemsPropertiesUpdated">
      <arg name="updatedProps" type="a(ia{sv})"/>
      <arg name="removedProps" type="a(ias)"/>
    </signal>
    <signal name="LayoutUpdated">
      <arg name="revision" type="u"/>
      <arg name="parent" type="i"/>
    </signal>
    <signal name="ItemActivationRequested">
      <arg name="id" type="i"/>
      <arg name="timestamp" type="u"/>
    </signal>
  </interface>
</node>"#;

/// The properties served on the item, in the order `GetAll` returns them.
const ITEM_PROPERTIES: &[&str] = &[
    "Category",
    "Id",
    "Title",
    "Status",
    "WindowId",
    "IconName",
    "IconPixmap",
    "OverlayIconName",
    "OverlayIconPixmap",
    "AttentionIconName",
    "AttentionIconPixmap",
    "AttentionMovieName",
    "ToolTip",
    "ItemIsMenu",
    "Menu",
];

/// The properties served on the menu.
const MENU_PROPERTIES: &[&str] = &["Version", "TextDirection", "Status", "IconThemePath"];
