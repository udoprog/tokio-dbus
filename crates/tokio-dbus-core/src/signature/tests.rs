use super::{Signature, SignatureError, SignatureErrorKind, Type, MAX_SIGNATURE};

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
fn test_iter() -> Result<(), SignatureError> {
    let s = Signature::new("aaa(as)yua{yy}")?;

    let mut it1 = s.iter();

    let Some(Type::Array(s2)) = it1.next() else {
        panic!("expected inner array");
    };

    assert_eq!(s2, "aa(as)");

    let Some(Type::Array(s3)) = s2.iter().next() else {
        panic!("expected inner array");
    };

    assert_eq!(s3, "a(as)");

    let Some(Type::Array(s4)) = s3.iter().next() else {
        panic!("expected inner struct");
    };

    assert_eq!(s4, "(as)");

    let Some(Type::Struct(s5)) = s4.iter().next() else {
        panic!("expected inner struct: {:?}", s4.iter().next());
    };

    assert_eq!(s5, "as");

    assert_eq!(it1.next(), Some(Type::Signature(Signature::BYTE)));
    assert_eq!(it1.next(), Some(Type::Signature(Signature::UINT32)));

    let Some(Type::Array(s6)) = it1.next() else {
        panic!("expected inner array");
    };

    let Some(Type::Dict(key, value)) = s6.iter().next() else {
        panic!("expected inner dict");
    };

    assert_eq!(key, Signature::BYTE);
    assert_eq!(value, Signature::BYTE);
    Ok(())
}
