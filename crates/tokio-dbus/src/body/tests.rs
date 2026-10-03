use alloc::string::ToString;

use crate::error::{Error, ErrorKind, Result};
use crate::{Alignment, BodyBuf, Endianness, ty};

#[track_caller]
fn assert_error<T>(result: Result<T>, kind: ErrorKind) {
    let expected = Error::new(kind).to_string();

    match result {
        Ok(..) => panic!("Expected error `{expected}`"),
        Err(error) => assert_eq!(error.to_string(), expected),
    }
}

/// A body holding only an array length prefix of 1000, without any elements.
fn short_array() -> BodyBuf {
    let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    buf.extend_from_slice(b"\xe8\x03\x00\x00");
    buf
}

/// A body holding `depth` variants nested inside of each other, with a `u32`
/// in the innermost one.
fn nested_variants(depth: usize) -> BodyBuf {
    let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);

    for _ in 1..depth {
        buf.extend_from_slice(b"\x01v\x00");
    }

    buf.extend_from_slice(b"\x01u\x00");
    buf.align_mut::<u32>();
    buf.extend_from_slice(b"\x2a\x00\x00\x00");
    buf
}

#[test]
fn read_until_past_end() {
    let buf = short_array();
    let mut body = buf.as_body();
    assert_error(body.read_until(5), ErrorKind::BufferUnderflow);
    assert_eq!(body.len(), 4);
}

#[test]
fn raw_array_past_end() {
    let buf = short_array();
    let mut body = buf.as_body();
    assert_error(
        body.load_raw_array(Alignment::U32),
        ErrorKind::BufferUnderflow,
    );
}

#[test]
fn array_past_end() {
    let buf = short_array();
    let mut body = buf.as_body();
    assert_error(body.load_array::<u32>(), ErrorKind::BufferUnderflow);
}

#[test]
fn nested_array_past_end() -> Result<()> {
    let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);
    // An outer array of 4 bytes, holding an inner array claiming 1000.
    buf.extend_from_slice(b"\x04\x00\x00\x00\xe8\x03\x00\x00");

    let mut body = buf.as_body();
    let mut outer = body.load_array::<ty::Array<u32>>()?;
    assert_error(outer.load_array(), ErrorKind::BufferUnderflow);
    Ok(())
}

#[test]
fn skip_nested_variants() -> Result<()> {
    let buf = nested_variants(64);
    let mut body = buf.as_body();
    assert_eq!(body.skip_variant()?, "v");
    assert!(body.is_empty());

    let buf = nested_variants(65);
    let mut body = buf.as_body();
    assert_error(body.skip_variant(), ErrorKind::NestingTooDeep);
    Ok(())
}

#[test]
fn skip_deeply_nested_variants() {
    let buf = nested_variants(1_000_000);
    let mut body = buf.as_body();
    assert_error(body.skip_variant(), ErrorKind::NestingTooDeep);
}

/// A body holding `variants` nested variants, with a `(au)` holding an empty
/// array in the innermost one.
fn struct_in_variants(variants: usize) -> BodyBuf {
    let mut buf = BodyBuf::with_endianness(Endianness::LITTLE);

    for _ in 1..variants {
        buf.extend_from_slice(b"\x01v\x00");
    }

    buf.extend_from_slice(b"\x04(au)\x00");
    buf.align_mut::<u64>();
    buf.extend_from_slice(b"\x00\x00\x00\x00");
    buf
}

#[test]
fn skip_containers_in_nested_variants() -> Result<()> {
    // The array is the 64th container.
    let buf = struct_in_variants(62);
    let mut body = buf.as_body();
    body.skip_variant()?;
    assert!(body.is_empty());

    let buf = struct_in_variants(63);
    let mut body = buf.as_body();
    assert_error(body.skip_variant(), ErrorKind::NestingTooDeep);
    Ok(())
}
