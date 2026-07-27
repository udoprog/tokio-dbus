//! Mapping D-Bus types onto the owned Rust types generated code speaks in.

use genco::prelude::*;
use tokio_dbus_core::signature::{self, Signature};

use crate::error::ErrorKind;
use crate::{Error, Result};

/// The single type named by a signature.
fn single(signature: &Signature) -> Result<signature::Type<'_>> {
    let mut iter = signature.iter();

    let Some(ty) = iter.next() else {
        return Err(Error::new(ErrorKind::EmptyType));
    };

    if iter.next().is_some() {
        return Err(Error::new(ErrorKind::CompoundType(signature.to_owned())));
    }

    Ok(ty)
}

/// The owned Rust type a value of this signature is decoded into.
///
/// # Examples
///
/// ```
/// use tokio_dbus_codegen::owned_type;
///
/// assert_eq!(owned_type("s")?.to_string()?, "String");
/// assert_eq!(owned_type("as")?.to_string()?, "Vec<String>");
/// assert_eq!(owned_type("a{sv}")?.to_string()?, "HashMap<String, Value>");
/// assert_eq!(owned_type("(iiay)")?.to_string()?, "(i32, i32, Vec<u8>)");
/// # Ok::<_, tokio_dbus_codegen::Error>(())
/// ```
pub fn owned_type(signature: &str) -> Result<rust::Tokens> {
    let signature = Signature::new(signature)?;
    owned(single(signature)?)
}

/// The Rust type a client takes for an argument of this signature, which
/// borrows where borrowing is free.
///
/// # Examples
///
/// ```
/// use tokio_dbus_codegen::parameter_type;
///
/// assert_eq!(parameter_type("s")?.to_string()?, "&str");
/// assert_eq!(parameter_type("u")?.to_string()?, "u32");
/// assert_eq!(parameter_type("as")?.to_string()?, "&[String]");
/// assert_eq!(parameter_type("a{sv}")?.to_string()?, "&HashMap<String, Value>");
/// # Ok::<_, tokio_dbus_codegen::Error>(())
/// ```
pub fn parameter_type(signature: &str) -> Result<rust::Tokens> {
    let signature = Signature::new(signature)?;
    parameter(single(signature)?)
}

fn owned(ty: signature::Type<'_>) -> Result<rust::Tokens> {
    Ok(match ty {
        signature::Type::Signature(signature) => basic(signature)?,
        signature::Type::Array(element) => {
            // NB: A dict entry is only legal as the element type of an array,
            // which is why a map is recognised here rather than on its own.
            if let Ok(signature::Type::Dict(key, value)) = single(element) {
                let key = owned(single(key)?)?;
                let value = owned(single(value)?)?;
                quote!(HashMap<$key, $value>)
            } else {
                let element = owned(single(element)?)?;
                quote!(Vec<$element>)
            }
        }
        signature::Type::Struct(fields) => {
            let mut out = rust::Tokens::new();

            for (index, field) in fields.iter().enumerate() {
                if index > 0 {
                    out.append(quote!(,));
                    out.space();
                }

                out.append(owned(field)?);
            }

            // NB: A one field struct needs a trailing comma to stay a tuple.
            if fields.iter().count() == 1 {
                out.append(quote!(,));
            }

            quote!(($out))
        }
        signature::Type::Dict(..) => {
            return Err(Error::new(ErrorKind::LooseDictEntry));
        }
    })
}

fn parameter(ty: signature::Type<'_>) -> Result<rust::Tokens> {
    Ok(match ty {
        signature::Type::Signature(signature) => match signature.as_bytes() {
            b"s" => quote!(&str),
            b"o" => quote!(&ObjectPath),
            b"g" => quote!(&Signature),
            b"v" => quote!(&Value),
            // NB: Everything else is a scalar, which is cheaper to pass by
            // value than by reference.
            _ => basic(signature)?,
        },
        signature::Type::Array(element) => {
            if single(element).is_ok_and(|t| matches!(t, signature::Type::Dict(..))) {
                let map = owned(ty)?;
                quote!(&$map)
            } else {
                let element = owned(single(element)?)?;
                quote!(&[$element])
            }
        }
        signature::Type::Struct(..) => {
            let fields = owned(ty)?;
            quote!(&$fields)
        }
        signature::Type::Dict(..) => {
            return Err(Error::new(ErrorKind::LooseDictEntry));
        }
    })
}

fn basic(signature: &Signature) -> Result<rust::Tokens> {
    Ok(match signature.as_bytes() {
        b"y" => quote!(u8),
        b"b" => quote!(bool),
        b"n" => quote!(i16),
        b"q" => quote!(u16),
        b"i" => quote!(i32),
        b"u" => quote!(u32),
        b"x" => quote!(i64),
        b"t" => quote!(u64),
        b"d" => quote!(f64),
        b"s" => quote!(String),
        b"o" => quote!(ObjectPathBuf),
        b"g" => quote!(SignatureBuf),
        b"v" => quote!(Value),
        // NB: Passing a file descriptor requires sending it out of band, which
        // this implementation does not support.
        b"h" => return Err(Error::new(ErrorKind::UnixFd)),
        _ => {
            return Err(Error::new(ErrorKind::UnknownType(signature.to_owned())));
        }
    })
}
