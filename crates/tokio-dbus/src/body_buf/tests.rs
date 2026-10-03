use crate::{BodyBuf, Endianness, Result, Signature, Variant, ty};

/// An array whose elements are 8-byte aligned is padded after its length
/// prefix, and that padding is not counted towards the encoded length.
#[test]
fn dict_array_padding() -> Result<()> {
    let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);

    let mut dict = buf.store_array::<ty::Dict<ty::Str, ty::Variant>>()?;
    dict.store_entry()
        .store("a")
        .store(Variant::U32(1))
        .finish();
    dict.finish();

    assert_eq!(buf.signature(), "a{sv}");

    #[rustfmt::skip]
    assert_eq!(buf.get(), &[
        // length, excluding the padding below
        16, 0, 0, 0,
        // padding up to the alignment of a dict entry
        0, 0, 0, 0,
        // "a"
        1, 0, 0, 0, b'a', 0,
        // the signature of the variant
        1, b'u', 0,
        // padding, then 1u32
        0, 0, 0, 1, 0, 0, 0,
    ]);

    Ok(())
}

/// The same, for an empty array. The padding is present even though there are
/// no elements to align.
#[test]
fn empty_dict_array_padding() -> Result<()> {
    let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);

    buf.store_array::<ty::Dict<ty::Str, ty::Variant>>()?
        .finish();
    buf.store(42u8)?;

    assert_eq!(buf.signature(), "a{sv}y");
    assert_eq!(buf.get(), &[0, 0, 0, 0, 0, 0, 0, 0, 42]);
    Ok(())
}

/// Data written after an array must not be padded to the alignment of the
/// element type.
#[test]
fn no_trailing_array_padding() -> Result<()> {
    let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);

    let mut array = buf.store_array::<u64>()?;
    array.store(1u64);
    array.finish();
    buf.store(2u8)?;

    assert_eq!(buf.signature(), "aty");

    #[rustfmt::skip]
    assert_eq!(buf.get(), &[
        8, 0, 0, 0,
        0, 0, 0, 0,
        1, 0, 0, 0, 0, 0, 0, 0,
        2,
    ]);

    Ok(())
}

/// Round trip an array of structs, which are aligned to 8 bytes.
#[test]
fn array_of_structs() -> Result<()> {
    let mut buf = BodyBuf::new();

    let mut array = buf.store_array::<(i32, i32, ty::Array<u8>)>()?;

    for n in [1i32, 2i32] {
        array
            .store_struct()
            .store(n)
            .store(n)
            .store_array(|w| w.write_slice(&[n as u8; 3]))
            .finish();
    }

    array.finish();

    assert_eq!(buf.signature(), "a(iiay)");

    let mut buf = buf.as_body();
    let mut array = buf.load_array::<(i32, i32, ty::Array<u8>)>()?;

    for n in [1i32, 2i32] {
        let Some((a, b, mut bytes)) = array.load_struct()? else {
            panic!("Missing element {n}");
        };

        assert_eq!(a, n);
        assert_eq!(b, n);
        assert_eq!(bytes.load()?, Some(n as u8));
        assert_eq!(bytes.load()?, Some(n as u8));
        assert_eq!(bytes.load()?, Some(n as u8));
        assert_eq!(bytes.load()?, None);
    }

    assert!(array.load_struct()?.is_none());
    Ok(())
}

/// Variants are aligned to a single byte, and may contain containers of any
/// depth.
#[test]
fn nested_variants() -> Result<()> {
    const INNER: &Signature = Signature::new_const(b"a{sv}");

    let mut buf = BodyBuf::new();

    buf.store(1u8)?;

    let mut outer = buf
        .store_variant(INNER)?
        .store_array::<ty::Dict<ty::Str, ty::Variant>>();

    outer
        .store_entry()
        .store("inner")
        .store_variant(INNER, |w| {
            let mut inner = w.store_array::<ty::Dict<ty::Str, ty::Variant>>();
            inner
                .store_entry()
                .store("value")
                .store(Variant::String("Hello World!"))
                .finish();
        })
        .finish();

    outer.finish();

    buf.store(2u8)?;

    assert_eq!(buf.signature(), "yvy");

    let mut buf = buf.as_body();
    assert_eq!(buf.load::<u8>()?, 1);
    assert_eq!(buf.skip_variant()?, INNER);
    assert_eq!(buf.load::<u8>()?, 2);
    assert!(buf.is_empty());
    Ok(())
}

/// Read back a nested variant rather than skipping over it.
#[test]
fn nested_variants_round_trip() -> Result<()> {
    const INNER: &Signature = Signature::new_const(b"a{sv}");

    let mut buf = BodyBuf::new();

    let mut outer = buf.store_array::<ty::Dict<ty::Str, ty::Variant>>()?;

    outer
        .store_entry()
        .store("inner")
        .store_variant(INNER, |w| {
            let mut inner = w.store_array::<ty::Dict<ty::Str, ty::Variant>>();
            inner
                .store_entry()
                .store("value")
                .store(Variant::String("Hello World!"))
                .finish();
            inner
                .store_entry()
                .store("n")
                .store(Variant::I32(-1))
                .finish();
        })
        .finish();

    outer.finish();

    assert_eq!(buf.signature(), "a{sv}");

    let mut buf = buf.as_body();
    let mut outer = buf.load_array::<ty::Dict<ty::Str, ty::Variant>>()?;

    let Some((key, mut value)) =
        outer.load_entry_as::<ty::Array<ty::Dict<ty::Str, ty::Variant>>>()?
    else {
        panic!("Missing entry");
    };

    assert_eq!(key, "inner");
    assert_eq!(
        value.load_entry()?,
        Some(("value", Variant::String("Hello World!")))
    );
    assert_eq!(value.load_entry()?, Some(("n", Variant::I32(-1))));
    assert_eq!(value.load_entry()?, None);
    Ok(())
}

/// Converting a partially read body copies it whole, so values keep their
/// alignment and the signature still describes the bytes.
#[test]
fn from_partially_read_body() -> Result<()> {
    let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    buf.store(1u8)?;
    buf.store(2u64)?;

    let mut body = buf.as_body();
    assert_eq!(body.load::<u8>()?, 1);

    let owned = BodyBuf::from(body);
    assert_eq!(owned.signature(), "yt");
    assert_eq!(
        owned.get(),
        &[1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0]
    );

    let mut body = owned.as_body();
    assert_eq!(body.load::<u8>()?, 1);
    assert_eq!(body.load::<u64>()?, 2);
    assert!(body.is_empty());
    Ok(())
}
