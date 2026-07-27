//! The same systray icon as [`systray.rs`], served through bindings generated
//! from interface files instead of dispatched by hand.
//!
//! [`systray.rs`]: https://github.com/udoprog/tokio-dbus/blob/main/examples/examples/systray.rs
//!
//! Run it with:
//!
//! ```sh
//! cargo run --example systray_codegen
//! ```
//!
//! `build.rs` asks [`tokio_dbus_codegen`] for servers for
//! `org.kde.StatusNotifierItem` and `com.canonical.dbusmenu`, and for a client
//! for `org.kde.StatusNotifierWatcher`. What is left to write here is the state
//! of the icon and the menu, plus the loop which hands incoming calls to the
//! generated dispatchers.
//!
//! Note what the generated `dispatch` covers beyond the methods themselves: it
//! answers `org.freedesktop.DBus.Properties` for its interface, so `Get`,
//! `GetAll` and `Set` come from the property accessors on the trait.
//!
//! # Desktop support
//!
//! KDE Plasma implements a status notifier host natively. GNOME Shell needs the
//! "AppIndicator and KStatusNotifierItem Support" extension. See the
//! documentation on the low level `systray` example for the details.

use std::collections::HashMap;

use anyhow::Result as AnyResult;
use tokio_dbus::ObjectPath;
use tokio_dbus::org_freedesktop_dbus::{self, NameFlag};
use tokio_dbus_runtime::{Connection, Error, Incoming, Result, Value};

include!(concat!(env!("OUT_DIR"), "/systray.rs"));

use self::dbusmenu::DbusmenuServer;
use self::status_notifier_item::StatusNotifierItemServer;
use self::status_notifier_watcher::StatusNotifierWatcher;

/// The object serving `org.kde.StatusNotifierItem`.
const ITEM_PATH: &ObjectPath = ObjectPath::new_const(b"/StatusNotifierItem");
/// The object serving `com.canonical.dbusmenu`.
const MENU_PATH: &ObjectPath = ObjectPath::new_const(b"/MenuBar");

const WATCHER_NAME: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &ObjectPath = ObjectPath::new_const(b"/StatusNotifierWatcher");

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
async fn main() -> AnyResult<()> {
    let mut conn = Connection::session_bus().await?;
    println!("Connected as {}", conn.unique_name());

    let mut app = Systray::new();

    // Hosts which predate object path registration look the item up by this
    // name, so the shape of it matters.
    conn.acquire_name(&app.name, NameFlag::DO_NOT_QUEUE).await?;
    println!("Acquired {}", app.name);

    // So that the item can be re-registered if the watcher restarts.
    conn.add_match(&format!(
        "type='signal',sender='{DBUS}',interface='{DBUS}',member='NameOwnerChanged',arg0='{WATCHER_NAME}'",
        DBUS = org_freedesktop_dbus::DESTINATION,
    ))
    .await?;

    let watcher = StatusNotifierWatcher::new(WATCHER_NAME, WATCHER_PATH);
    register(&watcher, &mut conn, &app.name).await;

    while app.running {
        match conn.next().await? {
            Incoming::Call(call) => {
                let handled = if call.path() == ITEM_PATH {
                    status_notifier_item::dispatch(&mut app, &mut conn, &call).await?
                } else if call.path() == MENU_PATH {
                    dbusmenu::dispatch(&mut app, &mut conn, &call).await?
                } else {
                    false
                };

                if !handled {
                    // Nothing recognised the call, so the caller is told rather
                    // than left waiting for a reply which never comes.
                    conn.reply_error(
                        &call,
                        &Error::remote(
                            org_freedesktop_dbus::UNKNOWN_METHOD_ERROR,
                            format_args!(
                                "No such method: {}.{}",
                                call.interface().unwrap_or_default(),
                                call.member()
                            ),
                        ),
                    )?;
                }
            }
            // The watcher appeared, so the item is registered again.
            Incoming::Signal(message) if message.member() == "NameOwnerChanged" => {
                let mut body = message.body();
                let name = body.read::<str>()?;
                let _old_owner = body.read::<str>()?;
                let new_owner = body.read::<str>()?;

                if name == WATCHER_NAME && !new_owner.is_empty() {
                    register(&watcher, &mut conn, &app.name).await;
                }
            }
            _ => {}
        }

        app.flush(&mut conn)?;
    }

    conn.release_name(&app.name).await?;
    conn.flush().await?;
    Ok(())
}

/// Ask the watcher to start tracking this item, reporting rather than failing
/// when it is not running yet.
async fn register(watcher: &StatusNotifierWatcher, conn: &mut Connection, name: &str) {
    match watcher.register_status_notifier_item(conn, name).await {
        Ok(()) => println!("Registered with {WATCHER_NAME}"),
        Err(error) => println!("Waiting for {WATCHER_NAME} ({error})"),
    }
}

/// The state of the systray icon.
struct Systray {
    name: String,
    status: String,
    themed: bool,
    icon: Vec<(i32, i32, Vec<u8>)>,
    revision: u32,
    root: Item,
    pending: Vec<Signal>,
    running: bool,
}

/// A signal which is emitted once the current call has been replied to.
enum Signal {
    NewIcon,
    NewStatus,
    Updated(Vec<i32>),
}

impl Systray {
    fn new() -> Self {
        Self {
            name: format!("org.kde.StatusNotifierItem-{}-1", std::process::id()),
            status: String::from("Active"),
            themed: false,
            icon: vec![icon()],
            revision: 1,
            root: Item::menu(),
            pending: Vec::new(),
            running: true,
        }
    }

    /// Emit the signals queued while handling a call.
    fn flush(&mut self, conn: &mut Connection) -> Result<()> {
        for signal in std::mem::take(&mut self.pending) {
            match signal {
                Signal::NewIcon => {
                    status_notifier_item::Signal::NewIcon.emit(conn, ITEM_PATH)?;
                }
                Signal::NewStatus => {
                    status_notifier_item::Signal::NewStatus {
                        status: self.status.clone(),
                    }
                    .emit(conn, ITEM_PATH)?;
                }
                Signal::Updated(ids) => {
                    let updated_props = ids
                        .iter()
                        .filter_map(|&id| self.root.find(id))
                        .map(|item| (item.id, item.properties(&[])))
                        .collect();

                    dbusmenu::Signal::ItemsPropertiesUpdated {
                        updated_props,
                        removed_props: Vec::new(),
                    }
                    .emit(conn, MENU_PATH)?;
                }
            }
        }

        Ok(())
    }

    /// Act on a menu item being clicked.
    fn clicked(&mut self, id: i32) {
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
                self.status = String::from(if attention {
                    "NeedsAttention"
                } else {
                    "Active"
                });
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
}

impl StatusNotifierItemServer for Systray {
    async fn context_menu(&mut self, x: i32, y: i32) -> Result<()> {
        println!("ContextMenu at ({x}, {y})");
        Ok(())
    }

    async fn activate(&mut self, x: i32, y: i32) -> Result<()> {
        println!("Activate at ({x}, {y})");
        Ok(())
    }

    async fn secondary_activate(&mut self, x: i32, y: i32) -> Result<()> {
        println!("SecondaryActivate at ({x}, {y})");
        Ok(())
    }

    async fn scroll(&mut self, delta: i32, orientation: String) -> Result<()> {
        println!("Scroll {delta} {orientation}");
        Ok(())
    }

    async fn category(&mut self) -> Result<String> {
        Ok(String::from("ApplicationStatus"))
    }

    async fn id(&mut self) -> Result<String> {
        Ok(String::from("tokio-dbus-systray"))
    }

    async fn title(&mut self) -> Result<String> {
        Ok(String::from("tokio-dbus systray example"))
    }

    async fn status(&mut self) -> Result<String> {
        Ok(self.status.clone())
    }

    /// Only meaningful for items which are backed by an X11 window.
    async fn window_id(&mut self) -> Result<i32> {
        Ok(0)
    }

    /// Hosts prefer a themed icon when this is set, which is what the "Use
    /// themed icon" entry in the menu toggles between.
    async fn icon_name(&mut self) -> Result<String> {
        Ok(String::from(if self.themed {
            "dialog-information"
        } else {
            ""
        }))
    }

    async fn icon_pixmap(&mut self) -> Result<Vec<(i32, i32, Vec<u8>)>> {
        Ok(if self.themed {
            Vec::new()
        } else {
            self.icon.clone()
        })
    }

    async fn overlay_icon_name(&mut self) -> Result<String> {
        Ok(String::new())
    }

    async fn overlay_icon_pixmap(&mut self) -> Result<Vec<(i32, i32, Vec<u8>)>> {
        Ok(Vec::new())
    }

    async fn attention_icon_name(&mut self) -> Result<String> {
        Ok(String::new())
    }

    async fn attention_icon_pixmap(&mut self) -> Result<Vec<(i32, i32, Vec<u8>)>> {
        Ok(Vec::new())
    }

    async fn attention_movie_name(&mut self) -> Result<String> {
        Ok(String::new())
    }

    /// Icon name, icon data, title and description.
    async fn tool_tip(&mut self) -> Result<(String, Vec<(i32, i32, Vec<u8>)>, String, String)> {
        Ok((
            String::new(),
            Vec::new(),
            String::from("tokio-dbus"),
            String::from("A systray example"),
        ))
    }

    /// When this is set the host opens the menu on left click instead of
    /// calling `Activate`.
    async fn item_is_menu(&mut self) -> Result<bool> {
        Ok(false)
    }

    async fn menu(&mut self) -> Result<tokio_dbus::ObjectPathBuf> {
        Ok(MENU_PATH.to_owned())
    }
}

impl DbusmenuServer for Systray {
    async fn get_layout(
        &mut self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
    ) -> Result<(u32, (i32, HashMap<String, Value>, Vec<Value>))> {
        let Some(item) = self.root.find(parent_id) else {
            return Err(Error::remote(
                org_freedesktop_dbus::INVALID_ARGS_ERROR,
                format_args!("No such menu item: {parent_id}"),
            ));
        };

        Ok((self.revision, item.layout(recursion_depth, &property_names)))
    }

    async fn get_group_properties(
        &mut self,
        ids: Vec<i32>,
        property_names: Vec<String>,
    ) -> Result<Vec<(i32, HashMap<String, Value>)>> {
        Ok(ids
            .into_iter()
            .filter_map(|id| self.root.find(id))
            .map(|item| (item.id, item.properties(&property_names)))
            .collect())
    }

    async fn get_property(&mut self, id: i32, name: String) -> Result<Value> {
        let Some(value) = self.root.find(id).and_then(|i| i.property(&name)) else {
            return Err(Error::remote(
                org_freedesktop_dbus::UNKNOWN_PROPERTY_ERROR,
                format_args!("No such property on {id}: {name}"),
            ));
        };

        Ok(value)
    }

    async fn event(
        &mut self,
        id: i32,
        event_id: String,
        // Only meaningful for a handful of event types, none of which this menu
        // uses.
        _data: Value,
        _timestamp: u32,
    ) -> Result<()> {
        if event_id == "clicked" {
            self.clicked(id);
        }

        Ok(())
    }

    async fn event_group(&mut self, events: Vec<(i32, String, Value, u32)>) -> Result<Vec<i32>> {
        for (id, event_id, _, _) in events {
            if event_id == "clicked" {
                self.clicked(id);
            }
        }

        Ok(Vec::new())
    }

    /// The menu is built up front, so it never needs refreshing before it is
    /// shown.
    async fn about_to_show(&mut self, _id: i32) -> Result<bool> {
        Ok(false)
    }

    async fn about_to_show_group(&mut self, _ids: Vec<i32>) -> Result<(Vec<i32>, Vec<i32>)> {
        Ok((Vec::new(), Vec::new()))
    }

    /// 3 is the version which introduced `EventGroup` and `AboutToShowGroup`,
    /// both of which are implemented above.
    async fn version(&mut self) -> Result<u32> {
        Ok(3)
    }

    async fn text_direction(&mut self) -> Result<String> {
        Ok(String::from("ltr"))
    }

    async fn status(&mut self) -> Result<String> {
        Ok(String::from("normal"))
    }

    async fn icon_theme_path(&mut self) -> Result<Vec<String>> {
        Ok(Vec::new())
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
    /// why for instance `enabled` is absent here. An empty filter means the
    /// caller wants all of them.
    fn properties(&self, filter: &[String]) -> HashMap<String, Value> {
        let mut out = HashMap::new();

        let mut insert = |name: &str, value: Value| {
            if filter.is_empty() || filter.iter().any(|f| f == name) {
                out.insert(name.to_owned(), value);
            }
        };

        if let Kind::Separator = self.kind {
            insert("type", Value::String("separator".into()));
            return out;
        }

        insert("label", Value::String(self.label.clone()));

        match self.kind {
            Kind::Check { checked } => {
                insert("toggle-type", Value::String("checkmark".into()));
                insert("toggle-state", Value::I32(i32::from(checked)));
            }
            Kind::Radio { checked } => {
                insert("toggle-type", Value::String("radio".into()));
                insert("toggle-state", Value::I32(i32::from(checked)));
            }
            _ => {}
        }

        if !self.children.is_empty() {
            insert("children-display", Value::String("submenu".into()));
        }

        out
    }

    fn property(&self, name: &str) -> Option<Value> {
        self.properties(&[]).remove(name)
    }

    /// One node of the menu layout, recursing into its children.
    ///
    /// A `recursionDepth` of `-1` means "the whole subtree", which is why the
    /// depth is compared against zero rather than counted down to it.
    ///
    /// Each child is a `Value` which is encoded as a variant holding the node,
    /// which is what makes the recursive `(ia{sv}av)` expressible with owned
    /// Rust values.
    fn layout(&self, depth: i32, filter: &[String]) -> (i32, HashMap<String, Value>, Vec<Value>) {
        let children = if depth == 0 {
            Vec::new()
        } else {
            self.children
                .iter()
                .map(|child| {
                    let (id, properties, children) = child.layout(depth - 1, filter);

                    Value::Struct(vec![
                        Value::I32(id),
                        Value::Dict {
                            key: tokio_dbus::Signature::STRING.to_owned(),
                            value: tokio_dbus::Signature::VARIANT.to_owned(),
                            entries: properties
                                .into_iter()
                                .map(|(k, v)| (Value::String(k), Value::Variant(Box::new(v))))
                                .collect(),
                        },
                        Value::Array {
                            element: tokio_dbus::Signature::VARIANT.to_owned(),
                            values: children,
                        },
                    ])
                })
                .collect()
        };

        (self.id, self.properties(filter), children)
    }
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
