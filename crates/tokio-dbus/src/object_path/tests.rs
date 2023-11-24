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
