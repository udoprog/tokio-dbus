//! Turning D-Bus names into Rust names.

/// Words which cannot be used as an identifier, and which are escaped with the
/// raw identifier syntax instead.
///
/// `crate`, `self`, `super` and `Self` cannot be raw identifiers, so they are
/// suffixed with an underscore instead.
const KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "dyn", "else", "enum", "extern", "false",
    "fn", "for", "gen", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "static", "struct", "trait", "true", "type", "union", "unsafe", "use",
    "where", "while", "abstract", "become", "box", "do", "final", "macro", "override", "priv",
    "try", "typeof", "unsized", "virtual", "yield",
];

/// Keywords which the raw identifier syntax cannot express.
const RESERVED: &[&str] = &["crate", "self", "Self", "super"];

/// Convert a D-Bus name such as `GetLayout` or `icon-name` into a snake case
/// Rust identifier, escaping it if it collides with a keyword.
///
/// # Examples
///
/// ```
/// use tokio_dbus_codegen::snake_case;
///
/// assert_eq!(snake_case("GetLayout"), "get_layout");
/// assert_eq!(snake_case("WindowId"), "window_id");
/// assert_eq!(snake_case("ProvideXdgActivationToken"), "provide_xdg_activation_token");
/// assert_eq!(snake_case("icon-name"), "icon_name");
/// assert_eq!(snake_case("type"), "r#type");
/// assert_eq!(snake_case("Self"), "self_");
/// ```
pub fn snake_case(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut chars = name.chars().peekable();
    let mut previous: Option<char> = None;

    while let Some(c) = chars.next() {
        if c == '-' || c == '.' || c == ' ' {
            if !out.ends_with('_') && !out.is_empty() {
                out.push('_');
            }

            previous = None;
            continue;
        }

        if c.is_ascii_uppercase() {
            let starts_word = match previous {
                // `getLayout` and `id2Name` start a word at the capital.
                Some(p) if p.is_ascii_lowercase() || p.is_ascii_digit() => true,
                // The last capital of a run starts a word when a lowercase
                // follows it, as in the `T` of `XdgToken`.
                Some(p) if p.is_ascii_uppercase() => {
                    chars.peek().is_some_and(|n| n.is_ascii_lowercase())
                }
                _ => false,
            };

            if starts_word && !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }

            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }

        previous = Some(c);
    }

    escape(out)
}

/// Escape an identifier which collides with a keyword.
fn escape(name: String) -> String {
    if name.is_empty() {
        return String::from("_");
    }

    if RESERVED.contains(&name.as_str()) {
        return format!("{name}_");
    }

    if KEYWORDS.contains(&name.as_str()) {
        return format!("r#{name}");
    }

    if name.starts_with(|c: char| c.is_ascii_digit()) {
        return format!("_{name}");
    }

    name
}

/// Convert a D-Bus name into a Pascal case Rust identifier, which is what enum
/// variants and generated types are named after.
///
/// # Examples
///
/// ```
/// use tokio_dbus_codegen::pascal_case;
///
/// assert_eq!(pascal_case("NewIcon"), "NewIcon");
/// assert_eq!(pascal_case("com.canonical.dbusmenu"), "Dbusmenu");
/// assert_eq!(pascal_case("icon-name"), "IconName");
/// ```
pub fn pascal_case(name: &str) -> String {
    let last = name.rsplit('.').next().unwrap_or(name);
    let mut out = String::with_capacity(last.len());
    let mut capitalize = true;

    for c in last.chars() {
        if c == '-' || c == '_' || c == ' ' {
            capitalize = true;
            continue;
        }

        if capitalize {
            out.extend(c.to_uppercase());
            capitalize = false;
        } else {
            out.push(c);
        }
    }

    if out.is_empty() {
        out.push('_');
    }

    out
}

/// The name of the module generated for an interface, derived from its last
/// segment.
///
/// # Examples
///
/// ```
/// use tokio_dbus_codegen::module_name;
///
/// assert_eq!(module_name("org.freedesktop.Notifications"), "notifications");
/// assert_eq!(module_name("org.kde.StatusNotifierItem"), "status_notifier_item");
/// assert_eq!(module_name("com.canonical.dbusmenu"), "dbusmenu");
/// ```
pub fn module_name(interface: &str) -> String {
    snake_case(interface.rsplit('.').next().unwrap_or(interface))
}

/// The name of an argument, falling back to a positional name when the
/// interface file does not give one.
pub fn argument_name(name: Option<&str>, index: usize) -> String {
    match name {
        Some(name) if !name.is_empty() => {
            let name = snake_case(name);

            // NB: Generated code uses a `__` prefix for its own locals, so an
            // argument is never allowed to take one.
            if name.starts_with("__") {
                format!("arg_{name}")
            } else {
                name
            }
        }
        _ => format!("arg{index}"),
    }
}
