use std::fmt::{self, Display};

use tokio_dbus::{
    Alignment, Body, ObjectPath, ObjectPathBuf, Raw, Signature, SignatureBuf, SignatureBuilder,
};
use tokio_dbus_core::signature;

use crate::error::ErrorKind;
use crate::{Decode, Encode, Error, Result};

/// A D-Bus value whose type is only known at runtime.
///
/// This is what a `v` in a signature is decoded into, and holds the value that
/// was inside the variant rather than the variant itself. A [`Value::Variant`]
/// is therefore only produced for a variant nested inside another one.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
///
/// use tokio_dbus_runtime::Value;
///
/// let mut hints = HashMap::new();
/// hints.insert(String::from("urgency"), Value::U8(2));
/// hints.insert(String::from("category"), Value::String("device".into()));
///
/// assert_eq!(hints["urgency"].signature()?, "y");
/// assert_eq!(hints["category"].signature()?, "s");
/// # Ok::<_, tokio_dbus_runtime::Error>(())
/// ```
// NB: The container variants carry the signature of what they hold, which makes
// them a good deal larger than the scalar ones. Boxing them would cost an
// allocation on every value in a message for no practical gain.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Value {
    /// A boolean.
    Bool(bool),
    /// A single byte.
    U8(u8),
    /// A signed 16-bit integer.
    I16(i16),
    /// An unsigned 16-bit integer.
    U16(u16),
    /// A signed 32-bit integer.
    I32(i32),
    /// An unsigned 32-bit integer.
    U32(u32),
    /// A signed 64-bit integer.
    I64(i64),
    /// An unsigned 64-bit integer.
    U64(u64),
    /// A 64-bit floating point number.
    F64(f64),
    /// A string.
    String(String),
    /// An object path.
    ObjectPath(ObjectPathBuf),
    /// A signature.
    Signature(SignatureBuf),
    /// An array.
    ///
    /// The signature of the element type is carried along, since it cannot be
    /// recovered from the values when the array is empty.
    Array {
        /// The type of each element.
        element: SignatureBuf,
        /// The elements.
        values: Vec<Value>,
    },
    /// A dictionary, which on the wire is an array of key and value pairs.
    Dict {
        /// The type of each key.
        key: SignatureBuf,
        /// The type of each value.
        value: SignatureBuf,
        /// The entries, in the order they appeared.
        entries: Vec<(Value, Value)>,
    },
    /// A struct.
    Struct(Vec<Value>),
    /// A variant nested inside of another value.
    Variant(Box<Value>),
}

impl Value {
    /// Construct an empty array of the given element type.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus_runtime::Value;
    ///
    /// let value = Value::array("(iiay)")?;
    /// assert_eq!(value.signature()?, "a(iiay)");
    /// # Ok::<_, tokio_dbus_runtime::Error>(())
    /// ```
    pub fn array(element: &str) -> Result<Self> {
        Ok(Value::Array {
            element: Signature::new(element)?.to_owned(),
            values: Vec::new(),
        })
    }

    /// The signature of this value.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus_runtime::Value;
    ///
    /// let value = Value::Struct(vec![Value::I32(1), Value::String("hi".into())]);
    /// assert_eq!(value.signature()?, "(is)");
    /// # Ok::<_, tokio_dbus_runtime::Error>(())
    /// ```
    pub fn signature(&self) -> Result<SignatureBuf> {
        let mut builder = SignatureBuilder::new();
        self.write_signature(&mut builder)?;
        Ok(builder.to_signature().to_owned())
    }

    fn write_signature(&self, builder: &mut SignatureBuilder) -> Result<()> {
        fn extend(builder: &mut SignatureBuilder, signature: &Signature) -> Result<()> {
            if !builder.extend_from_signature(signature) {
                return Err(Error::from(tokio_dbus::SignatureError::too_long()));
            }

            Ok(())
        }

        match self {
            Value::Bool(..) => extend(builder, Signature::BOOLEAN)?,
            Value::U8(..) => extend(builder, Signature::BYTE)?,
            Value::I16(..) => extend(builder, Signature::INT16)?,
            Value::U16(..) => extend(builder, Signature::UINT16)?,
            Value::I32(..) => extend(builder, Signature::INT32)?,
            Value::U32(..) => extend(builder, Signature::UINT32)?,
            Value::I64(..) => extend(builder, Signature::INT64)?,
            Value::U64(..) => extend(builder, Signature::UINT64)?,
            Value::F64(..) => extend(builder, Signature::DOUBLE)?,
            Value::String(..) => extend(builder, Signature::STRING)?,
            Value::ObjectPath(..) => extend(builder, Signature::OBJECT_PATH)?,
            Value::Signature(..) => extend(builder, Signature::SIGNATURE)?,
            Value::Array { element, .. } => {
                builder.open_array()?;
                extend(builder, element)?;
                builder.close_array();
            }
            Value::Dict { key, value, .. } => {
                builder.open_array()?;
                builder.open_dict()?;
                extend(builder, key)?;
                extend(builder, value)?;
                builder.close_dict()?;
                builder.close_array();
            }
            Value::Struct(fields) => {
                builder.open_struct()?;

                for field in fields {
                    field.write_signature(builder)?;
                }

                builder.close_struct()?;
            }
            Value::Variant(..) => extend(builder, Signature::VARIANT)?,
        }

        Ok(())
    }

    /// Write the value without its signature.
    pub(crate) fn encode_value(&self, raw: &mut Raw<'_>) {
        match self {
            Value::Bool(value) => raw.store(*value),
            Value::U8(value) => raw.store(*value),
            Value::I16(value) => raw.store(*value),
            Value::U16(value) => raw.store(*value),
            Value::I32(value) => raw.store(*value),
            Value::U32(value) => raw.store(*value),
            Value::I64(value) => raw.store(*value),
            Value::U64(value) => raw.store(*value),
            Value::F64(value) => raw.store(*value),
            Value::String(value) => raw.store(value.as_str()),
            Value::ObjectPath(value) => raw.store(&**value),
            Value::Signature(value) => raw.store(&**value),
            Value::Array { element, values } => {
                let mut array = raw.store_array(alignment_of(element));

                for value in values {
                    encode_element(value, element, &mut array.as_raw());
                }
            }
            Value::Dict {
                value: signature,
                entries,
                ..
            } => {
                let mut array = raw.store_array(Alignment::U64);

                for (key, value) in entries {
                    let mut entry = array.as_raw();
                    entry.align(Alignment::U64);
                    key.encode_value(&mut entry);
                    encode_element(value, signature, &mut entry);
                }
            }
            Value::Struct(fields) => {
                raw.align(Alignment::U64);

                for field in fields {
                    field.encode_value(raw);
                }
            }
            Value::Variant(value) => {
                // NB: This is the signature of the value inside the nested
                // variant, which is data rather than part of the signature of
                // the surrounding buffer.
                if let Ok(signature) = value.signature() {
                    raw.store_signature(&signature);
                    value.encode_value(raw);
                }
            }
        }
    }

    /// Read a value whose type is described by `signature`, which must name
    /// exactly one type.
    pub(crate) fn decode_as(body: &mut Body<'_>, signature: &Signature) -> Result<Value> {
        let mut iter = signature.iter();

        let Some(ty) = iter.next() else {
            return Err(Error::new(ErrorKind::UnsupportedType(Box::new(
                signature.to_owned(),
            ))));
        };

        if iter.next().is_some() {
            // NB: A variant holds exactly one value, so a signature naming more
            // than one type cannot be decoded into a single value.
            return Err(Error::new(ErrorKind::UnsupportedType(Box::new(
                signature.to_owned(),
            ))));
        }

        Value::decode_type(body, ty)
    }

    fn decode_type(body: &mut Body<'_>, ty: signature::Type<'_>) -> Result<Value> {
        match ty {
            signature::Type::Signature(signature) => match signature.as_bytes() {
                b"b" => Ok(Value::Bool(body.load_bool()?)),
                b"y" => Ok(Value::U8(body.load()?)),
                b"n" => Ok(Value::I16(body.load()?)),
                b"q" => Ok(Value::U16(body.load()?)),
                b"i" => Ok(Value::I32(body.load()?)),
                b"u" => Ok(Value::U32(body.load()?)),
                b"x" => Ok(Value::I64(body.load()?)),
                b"t" => Ok(Value::U64(body.load()?)),
                b"d" => Ok(Value::F64(body.load()?)),
                b"s" => Ok(Value::String(body.read::<str>()?.to_owned())),
                b"o" => Ok(Value::ObjectPath(body.read::<ObjectPath>()?.to_owned())),
                b"g" => Ok(Value::Signature(body.read::<Signature>()?.to_owned())),
                b"v" => {
                    let inner = body.read::<Signature>()?;
                    Ok(Value::Variant(Box::new(Value::decode_as(body, inner)?)))
                }
                _ => Err(Error::new(ErrorKind::UnsupportedType(Box::new(
                    signature.to_owned(),
                )))),
            },
            signature::Type::Array(element) => {
                let mut array = body.load_raw_array(alignment_of(element))?;

                // NB: A dict entry is only legal as the element type of an
                // array, which is why it is handled here rather than as a type
                // of its own.
                if let Some(signature::Type::Dict(key, value)) = single(element) {
                    let mut entries = Vec::new();

                    while !array.is_empty() {
                        array.align_to(Alignment::U64)?;
                        let key = Value::decode_as(&mut array, key)?;
                        let value = Value::decode_as(&mut array, value)?;
                        entries.push((key, value));
                    }

                    return Ok(Value::Dict {
                        key: key.to_owned(),
                        value: value.to_owned(),
                        entries,
                    });
                }

                let mut values = Vec::new();

                while !array.is_empty() {
                    values.push(Value::decode_as(&mut array, element)?);
                }

                Ok(Value::Array {
                    element: element.to_owned(),
                    values,
                })
            }
            signature::Type::Struct(fields) => {
                body.align_to(Alignment::U64)?;
                let mut values = Vec::new();

                for field in fields.iter() {
                    values.push(Value::decode_type(body, field)?);
                }

                Ok(Value::Struct(values))
            }
            signature::Type::Dict(key, value) => {
                body.align_to(Alignment::U64)?;
                let key = Value::decode_as(body, key)?;
                let value = Value::decode_as(body, value)?;
                Ok(Value::Struct(vec![key, value]))
            }
        }
    }
}

/// The single type named by a signature, if it names exactly one.
fn single(signature: &Signature) -> Option<signature::Type<'_>> {
    let mut iter = signature.iter();
    let ty = iter.next()?;

    if iter.next().is_some() {
        return None;
    }

    Some(ty)
}

/// Write one element of a container whose declared element type is
/// `signature`.
///
/// A value sitting in a variant position carries its own signature, which is
/// why both a value wrapped in [`Value::Variant`] and a bare one are accepted
/// there. Decoding produces the wrapped form.
fn encode_element(value: &Value, signature: &Signature, raw: &mut Raw<'_>) {
    if signature.as_bytes() != b"v" {
        value.encode_value(raw);
        return;
    }

    match value {
        Value::Variant(inner) => inner.encode(raw),
        value => value.encode(raw),
    }
}

/// The alignment of the first type in a signature.
pub(crate) fn alignment_of(signature: &Signature) -> Alignment {
    match signature.as_bytes().first() {
        Some(b'y' | b'g' | b'v') => Alignment::BYTE,
        Some(b'n' | b'q') => Alignment::U16,
        Some(b'x' | b't' | b'd' | b'(' | b'{') => Alignment::U64,
        _ => Alignment::U32,
    }
}

impl Encode for Value {
    // NB: A variant starts with the signature of the value it contains, which is
    // prefixed by a single byte holding its length.
    const ALIGNMENT: Alignment = Alignment::BYTE;

    fn encode(&self, raw: &mut Raw<'_>) {
        if let Ok(signature) = self.signature() {
            raw.store_signature(&signature);
            self.encode_value(raw);
        }
    }
}

impl Decode for Value {
    const ALIGNMENT: Alignment = Alignment::BYTE;

    fn decode(body: &mut Body<'_>) -> Result<Self> {
        let signature = body.read::<Signature>()?;
        Value::decode_as(body, signature)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(value) => value.fmt(f),
            Value::U8(value) => value.fmt(f),
            Value::I16(value) => value.fmt(f),
            Value::U16(value) => value.fmt(f),
            Value::I32(value) => value.fmt(f),
            Value::U32(value) => value.fmt(f),
            Value::I64(value) => value.fmt(f),
            Value::U64(value) => value.fmt(f),
            Value::F64(value) => value.fmt(f),
            Value::String(value) => value.fmt(f),
            Value::ObjectPath(value) => value.fmt(f),
            Value::Signature(value) => value.fmt(f),
            Value::Array { values, .. } => write_all(f, values.iter(), "[", "]"),
            Value::Dict { entries, .. } => {
                f.write_str("{")?;

                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }

                    write!(f, "{key}: {value}")?;
                }

                f.write_str("}")
            }
            Value::Struct(fields) => write_all(f, fields.iter(), "(", ")"),
            Value::Variant(value) => value.fmt(f),
        }
    }
}

fn write_all<'a, I>(f: &mut fmt::Formatter<'_>, values: I, open: &str, close: &str) -> fmt::Result
where
    I: Iterator<Item = &'a Value>,
{
    f.write_str(open)?;

    for (index, value) in values.enumerate() {
        if index > 0 {
            f.write_str(", ")?;
        }

        value.fmt(f)?;
    }

    f.write_str(close)
}
