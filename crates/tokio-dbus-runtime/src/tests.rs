use std::collections::HashMap;

use tokio_dbus::{BodyBuf, Signature};

use crate::{Decode, Encode, Result, Value};

mod connection;

/// Round trip a value through a body buffer, checking that it comes back
/// unchanged and that the whole body was consumed.
#[track_caller]
fn round_trip<T>(signature: &str, value: T) -> Result<()>
where
    T: Encode + Decode + PartialEq + std::fmt::Debug,
{
    let signature = Signature::new(signature)?;

    let mut buf = BodyBuf::new();
    buf.extend_signature(signature)?;
    value.encode(&mut buf.raw());

    assert_eq!(buf.signature(), signature);

    let mut body = buf.as_body();
    let actual = T::decode(&mut body)?;

    assert_eq!(actual, value);
    assert!(body.is_empty(), "Body was not fully consumed");
    Ok(())
}

#[test]
fn scalars() -> Result<()> {
    round_trip("y", 42u8)?;
    round_trip("b", true)?;
    round_trip("n", -1i16)?;
    round_trip("q", 1u16)?;
    round_trip("i", -1i32)?;
    round_trip("u", 1u32)?;
    round_trip("x", -1i64)?;
    round_trip("t", 1u64)?;
    round_trip("d", 1.5f64)?;
    round_trip("s", String::from("Hello World!"))?;
    Ok(())
}

#[test]
fn arrays() -> Result<()> {
    round_trip("as", vec![String::from("a"), String::from("b")])?;
    round_trip("ay", vec![1u8, 2, 3])?;
    // An array of 8-byte aligned elements is the case where the padding after
    // the length prefix matters.
    round_trip("at", vec![1u64, 2])?;
    round_trip("at", Vec::<u64>::new())?;
    round_trip("aas", vec![vec![String::from("a")], Vec::new()])?;
    Ok(())
}

#[test]
fn structs() -> Result<()> {
    round_trip("(is)", (1i32, String::from("one")))?;
    round_trip("a(iiay)", vec![(1i32, 2i32, vec![3u8, 4])])?;
    round_trip("(y(us))", (1u8, (2u32, String::from("two"))))?;
    Ok(())
}

#[test]
fn dicts() -> Result<()> {
    let mut map = HashMap::new();
    map.insert(String::from("a"), 1u32);
    map.insert(String::from("b"), 2u32);
    round_trip("a{su}", map)?;
    round_trip("a{su}", HashMap::<String, u32>::new())?;
    Ok(())
}

#[test]
fn variants() -> Result<()> {
    round_trip("v", Value::U8(1))?;
    round_trip("v", Value::String("Hello".into()))?;
    round_trip("v", Value::Bool(true))?;

    round_trip(
        "v",
        Value::Struct(vec![Value::I32(1), Value::String("one".into())]),
    )?;

    round_trip(
        "v",
        Value::Array {
            element: Signature::new("s")?.to_owned(),
            values: vec![Value::String("a".into()), Value::String("b".into())],
        },
    )?;

    round_trip("v", Value::array("(iiay)")?)?;
    Ok(())
}

/// The property dictionaries which show up all over the desktop interfaces.
#[test]
fn hints() -> Result<()> {
    let mut hints = HashMap::new();

    hints.insert(String::from("urgency"), Value::U8(1));
    hints.insert(String::from("category"), Value::String("device".into()));

    hints.insert(
        String::from("image-data"),
        Value::Struct(vec![
            Value::I32(1),
            Value::I32(1),
            Value::I32(4),
            Value::Bool(true),
            Value::I32(8),
            Value::I32(4),
            Value::Array {
                element: Signature::new("y")?.to_owned(),
                values: vec![Value::U8(1), Value::U8(2), Value::U8(3), Value::U8(4)],
            },
        ]),
    );

    round_trip("a{sv}", hints)?;
    Ok(())
}

/// The recursive layout served by `com.canonical.dbusmenu`.
#[test]
fn recursive_menu_layout() -> Result<()> {
    fn item(id: i32, children: Vec<Value>) -> Value {
        let mut properties = Vec::new();

        properties.push((
            Value::String("label".into()),
            Value::Variant(Box::new(Value::String(format!("Item {id}")))),
        ));

        Value::Struct(vec![
            Value::I32(id),
            Value::Dict {
                key: Signature::new("s").unwrap().to_owned(),
                value: Signature::new("v").unwrap().to_owned(),
                entries: properties,
            },
            Value::Array {
                element: Signature::new("v").unwrap().to_owned(),
                values: children,
            },
        ])
    }

    let layout = item(
        0,
        vec![
            Value::Variant(Box::new(item(1, Vec::new()))),
            Value::Variant(Box::new(item(
                2,
                vec![Value::Variant(Box::new(item(3, Vec::new())))],
            ))),
        ],
    );

    let value = (1u32, layout);
    round_trip("(u(ia{sv}av))", value.clone())?;

    // The same layout as it is actually sent, which is a `u` followed by the
    // struct rather than a struct containing both.
    let signature = Signature::new("u(ia{sv}av)")?;
    let mut buf = BodyBuf::new();
    buf.extend_signature(signature)?;
    value.0.encode(&mut buf.raw());
    value.1.encode_value(&mut buf.raw());

    let mut body = buf.as_body();
    assert_eq!(u32::decode(&mut body)?, 1);
    let actual = Value::decode_as(&mut body, Signature::new("(ia{sv}av)")?, 0)?;
    assert_eq!(actual, value.1);
    assert!(body.is_empty());
    Ok(())
}

/// Encode the reply to `com.canonical.dbusmenu.GetLayout` the way generated
/// code does, then read it back with the low level typed reader, which is an
/// independent decoder.
#[test]
fn get_layout_reply() -> Result<()> {
    use tokio_dbus::ty;

    use crate::Arguments;

    fn item(id: i32, children: Vec<Value>) -> (i32, HashMap<String, Value>, Vec<Value>) {
        let mut properties = HashMap::new();
        properties.insert(String::from("label"), Value::String(format!("Item {id}")));
        (id, properties, children)
    }

    fn nested(id: i32, children: Vec<Value>) -> Value {
        let (id, properties, children) = item(id, children);

        Value::Struct(vec![
            Value::I32(id),
            Value::Dict {
                key: Signature::STRING.to_owned(),
                value: Signature::VARIANT.to_owned(),
                entries: properties
                    .into_iter()
                    .map(|(k, v)| (Value::String(k), Value::Variant(Box::new(v))))
                    .collect(),
            },
            Value::Array {
                element: Signature::VARIANT.to_owned(),
                values: children,
            },
        ])
    }

    type Layout = (
        i32,
        ty::Array<ty::Dict<ty::Str, ty::Variant>>,
        ty::Array<ty::Variant>,
    );

    // The grandchild is what matters here: a variant holding a struct whose
    // `av` is itself non-empty.
    let root = item(
        0,
        vec![
            nested(1, Vec::new()),
            nested(2, vec![nested(3, Vec::new())]),
        ],
    );

    let mut arguments = Arguments::new(Signature::new("u(ia{sv}av)")?)?;
    arguments.store(7u32);
    arguments.store(&root);

    let mut body = arguments.body_for_test();
    assert_eq!(body.load::<u32>()?, 7);

    let (id, mut properties, mut children) = body.load_struct::<Layout>()?;
    assert_eq!(id, 0);
    assert_eq!(
        properties.load_entry()?,
        Some(("label", tokio_dbus::Variant::String("Item 0")))
    );
    assert_eq!(properties.load_entry()?, None);

    while !children.is_empty() {
        let signature = children.skip_variant()?.expect("Missing child").to_owned();

        assert_eq!(signature, "(ia{sv}av)");
    }

    assert!(body.is_empty(), "Body was not fully consumed");
    Ok(())
}

/// A bare value in a variant position is written as a variant, just like one
/// which has been wrapped explicitly.
#[test]
fn bare_values_in_variant_positions() -> Result<()> {
    let wrapped = Value::Array {
        element: Signature::VARIANT.to_owned(),
        values: vec![Value::Variant(Box::new(Value::U32(1)))],
    };

    let bare = Value::Array {
        element: Signature::VARIANT.to_owned(),
        values: vec![Value::U32(1)],
    };

    let mut left = BodyBuf::new();
    left.extend_signature(Signature::new("av")?)?;
    wrapped.encode_value(&mut left.raw());

    let mut right = BodyBuf::new();
    right.extend_signature(Signature::new("av")?)?;
    bare.encode_value(&mut right.raw());

    assert_eq!(left.get(), right.get());

    // What comes back out is always the wrapped form.
    let mut body = left.as_body();
    assert_eq!(
        Value::decode_as(&mut body, Signature::new("av")?, 0)?,
        wrapped
    );
    Ok(())
}

/// The message of an error, or of the low-level error it wraps.
fn message(error: &crate::Error) -> String {
    use std::error::Error as _;

    match error.source() {
        Some(source) => source.to_string(),
        None => error.to_string(),
    }
}

#[track_caller]
fn assert_error<T>(result: Result<T>, expected: &str)
where
    T: std::fmt::Debug,
{
    match result {
        Ok(value) => panic!("Expected error `{expected}`, got {value:?}"),
        Err(error) => assert_eq!(message(&error), expected),
    }
}

/// An array length prefix of 1000 without any elements following it must not
/// be trusted.
#[test]
fn short_array() -> Result<()> {
    let mut buf = BodyBuf::with_endianness(tokio_dbus::Endianness::LITTLE);
    buf.raw().store(1000u32);
    assert_eq!(buf.get(), b"\xe8\x03\x00\x00");

    assert_error(Vec::<u32>::decode(&mut buf.as_body()), "Buffer underflow");
    assert_error(
        HashMap::<String, u32>::decode(&mut buf.as_body()),
        "Buffer underflow",
    );
    assert_error(
        Value::decode_as(&mut buf.as_body(), Signature::new("au")?, 0),
        "Buffer underflow",
    );
    Ok(())
}

/// A body holding `depth` variants nested inside of each other, with a `u32`
/// in the innermost one.
fn nested_variants(depth: usize) -> BodyBuf {
    let mut buf = BodyBuf::with_endianness(tokio_dbus::Endianness::LITTLE);
    let mut raw = buf.raw();

    for _ in 1..depth {
        raw.store_signature(Signature::VARIANT);
    }

    raw.store_signature(Signature::UINT32);
    raw.store(42u32);
    buf
}

#[test]
fn nested_variants_limit() -> Result<()> {
    let buf = nested_variants(64);
    let mut body = buf.as_body();
    let mut value = Value::decode(&mut body)?;
    assert!(body.is_empty());

    for _ in 1..64 {
        let Value::Variant(inner) = value else {
            panic!("Expected a variant, got {value:?}");
        };

        value = *inner;
    }

    assert_eq!(value, Value::U32(42));

    let too_deep = "Containers are nested too deeply (max is 64)";
    assert_error(Value::decode(&mut nested_variants(65).as_body()), too_deep);
    assert_error(
        Value::decode(&mut nested_variants(1_000_000).as_body()),
        too_deep,
    );
    Ok(())
}

#[test]
fn struct_in_nested_variants_limit() -> Result<()> {
    /// A `(u)` inside of `variants` nested variants.
    fn build(variants: usize) -> BodyBuf {
        let mut buf = BodyBuf::with_endianness(tokio_dbus::Endianness::LITTLE);
        let mut raw = buf.raw();

        for _ in 1..variants {
            raw.store_signature(Signature::VARIANT);
        }

        raw.store_signature(Signature::new_const(b"(u)"));
        raw.align(tokio_dbus::Alignment::U64);
        raw.store(42u32);
        buf
    }

    let buf = build(63);
    let mut body = buf.as_body();
    Value::decode(&mut body)?;
    assert!(body.is_empty());

    let too_deep = "Containers are nested too deeply (max is 64)";
    assert_error(Value::decode(&mut build(64).as_body()), too_deep);
    Ok(())
}
