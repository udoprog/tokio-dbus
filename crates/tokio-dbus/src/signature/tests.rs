use crate::error::Result;
use crate::ty;
use crate::BodyBuf;

use super::Signature;

#[test]
fn signature_skip() -> Result<()> {
    let mut buf = BodyBuf::new();
    buf.store("Hello")?;
    buf.store("World")?;

    let sig = Signature::new_const(b"s");

    let mut read_buf = buf.as_body();

    super::skip(sig, &mut read_buf)?;

    let _ = read_buf.read::<str>()?;

    assert!(read_buf.is_empty(), "{:?}", read_buf.get());
    Ok(())
}

#[test]
fn signature_skip_array() -> Result<()> {
    let mut buf = BodyBuf::new();

    let mut array = buf.store_array::<ty::Array<ty::Str>>()?;

    let mut first = array.store_array();
    first.store("A");
    first.store("B");
    first.store("C");
    first.finish();

    let mut second = array.store_array();
    second.store("D");
    second.store("E");
    second.finish();

    array.finish();

    let sig = Signature::new_const(b"aas");
    assert_eq!(sig, buf.signature());

    let mut read_buf = buf.as_body();

    super::skip(sig, &mut read_buf)?;

    assert!(read_buf.is_empty(), "{:?}", read_buf.get());
    Ok(())
}
