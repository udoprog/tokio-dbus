use super::{MAX_SIGNATURE, Signature, SignatureError, SignatureErrorKind, Type};

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

#[test]
fn signature_buf_raw_parts_round_trip() {
    use super::{SignatureBuf, SignatureBuilder};

    let sig = SignatureBuf::new(b"a(is)").unwrap();
    let (_, init) = sig.clone().into_raw_parts();
    assert_eq!(init, 5);

    let builder = SignatureBuilder::from_owned_signature(sig);
    assert_eq!(builder.to_signature().as_bytes(), b"a(is)");
}

/// Arrays count towards the limit by how deeply they are nested, not by how
/// many there are.
#[test]
fn array_limit_is_by_depth() {
    let mut many = [b'i'; 66];
    let mut n = 0;

    while n < many.len() {
        many[n] = b'a';
        n += 2;
    }

    test!(&many, Ok(..));
}

#[test]
fn builder_rejects_invalid_signatures() -> Result<(), SignatureError> {
    use super::SignatureBuilder;

    #[track_caller]
    fn fails(
        builder: &mut SignatureBuilder,
        op: impl FnOnce(&mut SignatureBuilder) -> Result<(), SignatureError>,
        kind: SignatureErrorKind,
    ) {
        let before = builder.clone();
        assert_eq!(op(builder).map_err(|e| e.kind), Err(kind));
        assert!(*builder == before, "Builder changed by failed operation");
    }

    let mut b = SignatureBuilder::new();

    fails(&mut b, |b| b.close_struct(), StructEndedButNotStarted);
    fails(&mut b, |b| b.close_dict(), DictEndedButNotStarted);
    fails(&mut b, |b| b.close_array(), ArrayEndedButNotStarted);
    fails(&mut b, |b| b.open_dict(), DictEntryNotInsideArray);

    b.open_struct()?;
    assert_eq!(b.to_signature(), "");
    fails(&mut b, |b| b.close_struct(), StructHasNoFields);
    fails(&mut b, |b| b.open_dict(), DictEntryNotInsideArray);
    b.open_array()?;
    fails(&mut b, |b| b.close_array(), MissingArrayElementType);
    fails(&mut b, |b| b.close_struct(), MissingArrayElementType);
    b.open_dict()?;
    fails(
        &mut b,
        |b| b.try_extend_from_signature(Signature::new("ai")?),
        DictKeyMustBeBasicType,
    );
    fails(&mut b, |b| b.close_dict(), DictEntryHasNoFields);
    b.try_extend_from_signature(Signature::STRING)?;
    fails(&mut b, |b| b.close_dict(), DictEntryHasOnlyOneField);
    b.try_extend_from_signature(Signature::VARIANT)?;
    fails(
        &mut b,
        |b| b.try_extend_from_signature(Signature::BYTE),
        DictEntryHasTooManyFields,
    );
    fails(&mut b, |b| b.close_array(), MissingArrayElementType);
    b.close_dict()?;
    assert_eq!(b.to_signature(), "");
    b.close_array()?;
    fails(&mut b, |b| b.close_array(), ArrayEndedButNotStarted);
    b.close_struct()?;
    assert_eq!(b.to_signature(), "(a{sv})");

    b.open_array()?;
    assert_eq!(b.to_signature(), "(a{sv})");
    assert!(b.extend_from_signature(Signature::new("a{sv}")?));
    b.close_array()?;
    assert_eq!(b.to_signature(), "(a{sv})aa{sv}");

    let mut b = SignatureBuilder::new();

    for _ in 0..32 {
        b.open_array()?;
    }

    fails(&mut b, |b| b.open_array(), ExceededMaximumArrayRecursion);
    b.try_extend_from_signature(Signature::BYTE)?;

    for _ in 0..32 {
        b.close_array()?;
    }

    assert_eq!(b.to_signature().len(), 33);

    let mut b = SignatureBuilder::new();

    for _ in 0..32 {
        b.open_struct()?;
    }

    fails(&mut b, |b| b.open_struct(), ExceededMaximumStructRecursion);
    Ok(())
}

/// A signature on the wire is prefixed by its length as a single byte.
#[test]
fn builder_length_limit() -> Result<(), SignatureError> {
    use super::SignatureBuilder;

    let mut b = SignatureBuilder::new();
    b.try_extend_from_signature(Signature::new(&[b'i'; 254])?)?;

    assert_eq!(
        b.try_extend_from_signature(Signature::new("ai")?)
            .map_err(|e| e.kind),
        Err(SignatureTooLong)
    );

    b.try_extend_from_signature(Signature::BYTE)?;
    assert_eq!(b.to_signature().len(), 255);
    assert!(Signature::new(b.to_signature().as_bytes()).is_ok());

    assert_eq!(b.open_struct().map_err(|e| e.kind), Err(SignatureTooLong));
    assert!(!b.extend_from_signature(Signature::BYTE));
    assert_eq!(b.to_signature().len(), 255);
    Ok(())
}

/// Whatever sequence of operations is applied to a builder, the signature it
/// hands out is valid, and failed operations leave it unchanged.
#[test]
fn builder_signatures_are_valid() {
    use super::SignatureBuilder;

    type Op = fn(&mut SignatureBuilder) -> Result<(), SignatureError>;

    const OPS: [Op; 8] = [
        SignatureBuilder::open_array,
        SignatureBuilder::close_array,
        SignatureBuilder::open_struct,
        SignatureBuilder::close_struct,
        SignatureBuilder::open_dict,
        SignatureBuilder::close_dict,
        |b| b.try_extend_from_signature(Signature::INT32),
        |b| b.try_extend_from_signature(Signature::new_const(b"ai")),
    ];

    fn visit(builder: &SignatureBuilder, depth: usize) {
        if let Err(error) = Signature::new(builder.to_signature().as_bytes()) {
            panic!(
                "{:?} is invalid: {error}",
                builder.to_signature().as_bytes()
            );
        }

        if depth == 0 {
            return;
        }

        for op in OPS {
            let mut next = builder.clone();

            if op(&mut next).is_err() {
                assert!(next == *builder, "Builder changed by failed operation");
                assert_eq!(next.to_signature(), builder.to_signature());
            } else {
                visit(&next, depth - 1);
            }
        }
    }

    visit(&SignatureBuilder::new(), if cfg!(miri) { 3 } else { 6 });
}
