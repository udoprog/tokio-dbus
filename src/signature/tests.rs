use crate::error::Result;
use crate::ty;
use crate::{buf::AlignedBuf, BodyBuf};

use super::{Signature, SignatureErrorKind, MAX_SIGNATURE};

use SignatureErrorKind::*;

macro_rules! test {
    ($input:expr, $expected:pat) => {{
        let actual = Signature::new($input).map_err(|e| e.kind);

        assert!(
            matches!(actual, $expected),
            "{actual:?} does not match {}",
            stringify!($expected)
        );
    }};
}

#[test]
fn signature_tests() {
    test!(b"", Ok(..));
    test!(b"sss", Ok(..));
    test!(b"i", Ok(..));
    test!(b"b", Ok(..));
    test!(b"ai", Ok(..));
    test!(b"(i)", Ok(..));
    test!(b"w", Err(UnknownTypeCode(..)));
    test!(b"a", Err(MissingArrayElementType));
    test!(b"aaaaaa", Err(MissingArrayElementType));
    test!(b"ii(ii)a", Err(MissingArrayElementType));
    test!(b"ia", Err(MissingArrayElementType));
    test!(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaai", Ok(..));
    test!(
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaai",
        Err(ExceededMaximumArrayRecursion)
    );
    test!(b")", Err(StructEndedButNotStarted));
    test!(b"}", Err(DictEndedButNotStarted));
    test!(b"i)", Err(StructEndedButNotStarted));
    test!(b"a)", Err(MissingArrayElementType));
    test!(b"(", Err(StructStartedButNotEnded));
    test!(b"(i", Err(StructStartedButNotEnded));
    test!(b"(iiiii", Err(StructStartedButNotEnded));
    test!(b"(ai", Err(StructStartedButNotEnded));
    test!(b"()", Err(StructHasNoFields));
    test!(b"(())", Err(StructHasNoFields));
    test!(b"a()", Err(StructHasNoFields));
    test!(b"i()", Err(StructHasNoFields));
    test!(b"()i", Err(StructHasNoFields));
    test!(b"(a)", Err(MissingArrayElementType));
    test!(b"a{ia}", Err(MissingArrayElementType));
    test!(b"a{}", Err(DictEntryHasNoFields));
    test!(b"a{aii}", Err(DictKeyMustBeBasicType));
    test!(b" ", Err(UnknownTypeCode(..)));
    test!(b"not a valid signature", Err(UnknownTypeCode(..)));
    test!(b"123", Err(UnknownTypeCode(..)));
    test!(b".", Err(UnknownTypeCode(..)));
    /* https://bugs.freedesktop.org/show_bug.cgi?id=17803 */
    test!(b"a{(ii)i}", Err(DictKeyMustBeBasicType));
    test!(b"a{i}", Err(DictEntryHasOnlyOneField));
    test!(b"{is}", Err(DictEntryNotInsideArray));
    test!(b"a{isi}", Err(DictEntryHasTooManyFields));
    test!(&[b'i'; 255], Ok(..));
    test!(&[b'i'; MAX_SIGNATURE], Err(SignatureTooLong));
    test! {
        b"((((((((((((((((((((((((((((((((ii))))))))))))))))))))))))))))))))",
        Ok(..)
    };
    test! {
        b"(((((((((((((((((((((((((((((((((ii))))))))))))))))))))))))))))))))",
        Err(ExceededMaximumStructRecursion)
    };
}

#[test]
fn signature_skip() -> Result<()> {
    let mut buf = AlignedBuf::new();
    buf.write("Hello");
    buf.write("World");

    let sig = Signature::new_const(b"s");

    let mut read_buf = buf.read();

    sig.skip(&mut read_buf)?;

    let _ = read_buf.read::<str>()?;

    assert!(read_buf.is_empty(), "{:?}", read_buf.get());
    Ok(())
}

#[test]
fn signature_skip_array() -> Result<()> {
    let mut buf = BodyBuf::new();

    let mut array = buf.write_array::<ty::Array<ty::Str>>()?;

    let mut first = array.write_array();
    first.write("A");
    first.write("B");
    first.write("C");
    first.finish();

    let mut second = array.write_array();
    second.write("D");
    second.write("E");
    second.finish();

    array.finish();

    let sig = Signature::new_const(b"aas");
    assert_eq!(sig, buf.signature());

    let mut read_buf = buf.read();

    sig.skip(&mut read_buf)?;

    assert!(read_buf.is_empty(), "{:?}", read_buf.get());
    Ok(())
}
