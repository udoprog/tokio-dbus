use super::ObjectPath;

#[test]
fn legal_paths() {
    assert!(ObjectPath::new(b"").is_err());
    assert!(ObjectPath::new(b"a").is_err());
    assert!(ObjectPath::new(b"/a").is_ok());
    assert!(ObjectPath::new(b"/a").is_ok());
    assert!(ObjectPath::new(b"//").is_err());
    assert!(ObjectPath::new(b"/se/tedro").is_ok());
    assert!(ObjectPath::new(b"/se/tedro/").is_err());
}

/// The root path is the only path which may end in a slash.
#[test]
fn root_path() {
    assert!(ObjectPath::new(b"/").is_ok());
    assert_eq!(ObjectPath::ROOT, ObjectPath::new(b"/").unwrap());
    assert!(ObjectPath::ROOT.iter().next().is_none());
}

/// Underscores are legal in elements, per the specification which permits the
/// ASCII characters `"[A-Z][a-z][0-9]_"`.
#[test]
fn underscores_are_legal() {
    assert!(ObjectPath::new(b"/_").is_ok());
    assert!(ObjectPath::new(b"/_foo").is_ok());
    assert!(ObjectPath::new(b"/foo_").is_ok());
    assert!(ObjectPath::new(b"/foo_bar").is_ok());
    assert!(ObjectPath::new(b"/___").is_ok());
    assert!(ObjectPath::new(b"/org/freedesktop/DBus_Test/_1").is_ok());
    assert!(ObjectPath::new(b"/se/tedro/dbus_example").is_ok());

    let path = ObjectPath::new(b"/foo_bar/_baz1").unwrap();
    let mut it = path.iter();
    assert_eq!(it.next(), Some("foo_bar"));
    assert_eq!(it.next(), Some("_baz1"));
    assert_eq!(it.next(), None);
}

/// Elements may start with a digit, unlike interface and member names.
#[test]
fn elements_may_start_with_a_digit() {
    assert!(ObjectPath::new(b"/0").is_ok());
    assert!(ObjectPath::new(b"/1foo").is_ok());
    assert!(ObjectPath::new(b"/foo/2bar").is_ok());
}

#[test]
fn every_legal_element_character() {
    // A single element containing every legal character: 26 + 26 + 10 + 1,
    // prefixed by the leading '/'.
    let mut element = [b'_'; 64];
    element[0] = b'/';
    let mut len = 1;

    for b in u8::MIN..=u8::MAX {
        let legal = b.is_ascii_alphanumeric() || b == b'_';

        let mut path = *b"/foo/X/bar";
        path[5] = b;

        assert_eq!(
            ObjectPath::new(&path).is_ok(),
            legal,
            "byte {b:?} in element position"
        );

        if legal {
            element[len] = b;
            len += 1;
        }
    }

    assert_eq!(len, element.len());
    assert!(ObjectPath::new(&element).is_ok());
}

#[test]
fn illegal_paths() {
    // Must begin with a slash.
    assert!(ObjectPath::new(b"foo").is_err());
    assert!(ObjectPath::new(b"foo/bar").is_err());
    assert!(ObjectPath::new(b"_foo").is_err());

    // No empty elements.
    assert!(ObjectPath::new(b"//foo").is_err());
    assert!(ObjectPath::new(b"/foo//bar").is_err());
    assert!(ObjectPath::new(b"/foo///bar").is_err());

    // No trailing slash except for the root path.
    assert!(ObjectPath::new(b"/foo/").is_err());
    assert!(ObjectPath::new(b"/foo/bar/").is_err());

    // Characters outside of `"[A-Z][a-z][0-9]_"`.
    assert!(ObjectPath::new(b"/foo-bar").is_err());
    assert!(ObjectPath::new(b"/foo.bar").is_err());
    assert!(ObjectPath::new(b"/foo bar").is_err());
    assert!(ObjectPath::new(b"/foo\0bar").is_err());
    assert!(ObjectPath::new("/fooäbar").is_err());
}
