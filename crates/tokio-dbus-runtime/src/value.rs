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
        match self {
            Value::Bool(..) => builder.try_extend_from_signature(Signature::BOOLEAN)?,
            Value::U8(..) => builder.try_extend_from_signature(Signature::BYTE)?,
            Value::I16(..) => builder.try_extend_from_signature(Signature::INT16)?,
            Value::U16(..) => builder.try_extend_from_signature(Signature::UINT16)?,
            Value::I32(..) => builder.try_extend_from_signature(Signature::INT32)?,
            Value::U32(..) => builder.try_extend_from_signature(Signature::UINT32)?,
            Value::I64(..) => builder.try_extend_from_signature(Signature::INT64)?,
            Value::U64(..) => builder.try_extend_from_signature(Signature::UINT64)?,
            Value::F64(..) => builder.try_extend_from_signature(Signature::DOUBLE)?,
            Value::String(..) => builder.try_extend_from_signature(Signature::STRING)?,
            Value::ObjectPath(..) => builder.try_extend_from_signature(Signature::OBJECT_PATH)?,
            Value::Signature(..) => builder.try_extend_from_signature(Signature::SIGNATURE)?,
            Value::Array { element, .. } => {
                builder.open_array()?;
                builder.try_extend_from_signature(single_type(element)?)?;
                builder.close_array()?;
            }
            Value::Dict { key, value, .. } => {
                builder.open_array()?;
                builder.open_dict()?;
                builder.try_extend_from_signature(single_type(key)?)?;
                builder.try_extend_from_signature(single_type(value)?)?;
                builder.close_dict()?;
                builder.close_array()?;
            }
            Value::Struct(fields) => {
                builder.open_struct()?;

                for field in fields {
                    field.write_signature(builder)?;
                }

                builder.close_struct()?;
            }
            Value::Variant(..) => builder.try_extend_from_signature(Signature::VARIANT)?,
        }

        Ok(())
    }

    /// Check that this value can be encoded.
    ///
    /// This fails if its [`signature()`] cannot be built, such as for an empty
    /// [`Value::Struct`] or a declared element type which does not name exactly
    /// one type, if an element, key or value does not match the type declared
    /// for it, or if containers are nested more deeply than D-Bus allows.
    ///
    /// [`Arguments`] performs this check before writing a value, and refuses to
    /// send arguments for which it failed.
    ///
    /// [`signature()`]: Self::signature
    /// [`Arguments`]: crate::Arguments
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::Signature;
    /// use tokio_dbus_runtime::Value;
    ///
    /// let value = Value::array("s")?;
    /// assert!(value.validate().is_ok());
    ///
    /// let value = Value::Array {
    ///     element: Signature::STRING.to_owned(),
    ///     values: vec![Value::U8(1)],
    /// };
    ///
    /// assert!(value.validate().is_err());
    /// assert!(Value::Struct(vec![]).validate().is_err());
    /// # Ok::<_, tokio_dbus_runtime::Error>(())
    /// ```
    pub fn validate(&self) -> Result<()> {
        // NB: A value is encoded inside of a variant, which is a container.
        self.validate_at(1)
    }

    /// Validate a value which is written along with its own signature, where
    /// `depth` is the number of containers enclosing it.
    fn validate_at(&self, depth: usize) -> Result<()> {
        self.signature()?;
        self.validate_contents(depth)
    }

    /// Validate what this value holds, once its own signature is known to be
    /// valid.
    fn validate_contents(&self, depth: usize) -> Result<()> {
        match self {
            Value::Array { element, values } => {
                let depth = enter(depth)?;

                for value in values {
                    validate_element(value, element, depth)?;
                }
            }
            Value::Dict {
                key,
                value: signature,
                entries,
            } => {
                // NB: Both the array and each dict entry are containers.
                let depth = enter(enter(depth)?)?;

                for (key_value, value) in entries {
                    validate_element(key_value, key, depth)?;
                    validate_element(value, signature, depth)?;
                }
            }
            Value::Struct(fields) => {
                let depth = enter(depth)?;

                for field in fields {
                    field.validate_contents(depth)?;
                }
            }
            Value::Variant(value) => {
                value.validate_at(enter(depth)?)?;
            }
            _ => {}
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
                key: key_signature,
                value: signature,
                entries,
            } => {
                let mut array = raw.store_array(Alignment::U64);

                for (key, value) in entries {
                    let mut entry = array.as_raw();
                    entry.align(Alignment::U64);
                    encode_element(key, key_signature, &mut entry);
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
                value.encode(raw);
            }
        }
    }

    /// Read a value whose type is described by `signature`, which must name
    /// exactly one type. `depth` is the number of containers (variants
    /// included) enclosing it.
    pub(crate) fn decode_as(
        body: &mut Body<'_>,
        signature: &Signature,
        depth: usize,
    ) -> Result<Value> {
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

        Value::decode_type(body, ty, depth)
    }

    /// Decode a single type, where `depth` is the number of containers
    /// (variants included) enclosing it.
    // NB: Each kind of value is decoded by a function of its own, because a
    // `Value` is large and an unoptimized frame holding a temporary for every
    // kind of value would overflow the stack before the depth limit is reached.
    fn decode_type(body: &mut Body<'_>, ty: signature::Type<'_>, depth: usize) -> Result<Value> {
        match ty {
            signature::Type::Signature(signature) if signature == Signature::VARIANT => {
                decode_variant(body, depth)
            }
            signature::Type::Signature(signature) => decode_basic(body, signature),
            signature::Type::Array(element) => decode_array(body, element, depth),
            signature::Type::Struct(fields) => decode_struct(body, fields, depth),
            signature::Type::Dict(key, value) => decode_dict_entry(body, key, value, depth),
        }
    }
}

fn decode_basic(body: &mut Body<'_>, signature: &Signature) -> Result<Value> {
    match signature.as_bytes() {
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
        _ => Err(Error::new(ErrorKind::UnsupportedType(Box::new(
            signature.to_owned(),
        )))),
    }
}

fn decode_variant(body: &mut Body<'_>, depth: usize) -> Result<Value> {
    let depth = enter(depth)?;
    let inner = body.read::<Signature>()?;
    let value = Value::decode_as(body, inner, depth)?;
    Ok(Value::Variant(Box::new(value)))
}

fn decode_array(body: &mut Body<'_>, element: &Signature, depth: usize) -> Result<Value> {
    let depth = enter(depth)?;
    let mut array = body.load_raw_array(alignment_of(element))?;

    // NB: A dict entry is only legal as the element type of an array, which is
    // why it is handled here rather than as a type of its own.
    if let Some(signature::Type::Dict(key, value)) = single(element) {
        let mut entries = Vec::new();

        while !array.is_empty() {
            array.align_to(Alignment::U64)?;
            // NB: The dict entry is a container of its own.
            let depth = enter(depth)?;
            let key = Value::decode_as(&mut array, key, depth)?;
            let value = Value::decode_as(&mut array, value, depth)?;
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
        values.push(Value::decode_as(&mut array, element, depth)?);
    }

    Ok(Value::Array {
        element: element.to_owned(),
        values,
    })
}

fn decode_struct(body: &mut Body<'_>, fields: &Signature, depth: usize) -> Result<Value> {
    let depth = enter(depth)?;
    body.align_to(Alignment::U64)?;
    let mut values = Vec::new();

    for field in fields.iter() {
        values.push(Value::decode_type(body, field, depth)?);
    }

    Ok(Value::Struct(values))
}

fn decode_dict_entry(
    body: &mut Body<'_>,
    key: &Signature,
    value: &Signature,
    depth: usize,
) -> Result<Value> {
    let depth = enter(depth)?;
    body.align_to(Alignment::U64)?;
    let key = Value::decode_as(body, key, depth)?;
    let value = Value::decode_as(body, value, depth)?;
    Ok(Value::Struct(vec![key, value]))
}

/// Enter a container at `depth`, returning the depth of its contents.
///
/// Bounds the recursion of decoding, since the nesting of variants is only
/// limited by the size of the message.
fn enter(depth: usize) -> Result<usize> {
    if depth >= signature::MAX_DEPTH {
        return Err(Error::new(ErrorKind::NestingTooDeep));
    }

    Ok(depth + 1)
}

/// The signature of an element, key or value of a container, which must name
/// exactly one type.
fn single_type(signature: &Signature) -> Result<&Signature> {
    if single(signature).is_none() {
        return Err(Error::new(ErrorKind::UnsupportedType(Box::new(
            signature.to_owned(),
        ))));
    }

    Ok(signature)
}

/// Validate one element of a container whose declared element type is
/// `signature`, where `depth` is the number of containers enclosing it.
///
/// This accepts what [`encode_element`] accepts.
fn validate_element(value: &Value, signature: &Signature, depth: usize) -> Result<()> {
    if signature.as_bytes() == b"v" {
        let depth = enter(depth)?;

        return match value {
            Value::Variant(inner) => inner.validate_at(depth),
            value => value.validate_at(depth),
        };
    }

    let actual = value.signature()?;

    if actual != *signature {
        return Err(Error::new(ErrorKind::UnexpectedSignature(Box::new((
            signature.to_owned(),
            actual,
        )))));
    }

    value.validate_contents(depth)
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

    /// Write the value along with its signature.
    ///
    /// The value must be checked with [`validate()`] first, which [`Arguments`]
    /// does. What is written for an invalid value is not a valid variant.
    ///
    /// [`validate()`]: Value::validate
    /// [`Arguments`]: crate::Arguments
    fn encode(&self, raw: &mut Raw<'_>) {
        match self.signature() {
            Ok(signature) => {
                raw.store_signature(&signature);
                self.encode_value(raw);
            }
            Err(..) => {
                raw.store_signature(Signature::empty());
            }
        }
    }

    #[inline]
    fn validate(&self) -> Result<()> {
        Value::validate(self)
    }
}

impl Decode for Value {
    const ALIGNMENT: Alignment = Alignment::BYTE;

    fn decode(body: &mut Body<'_>) -> Result<Self> {
        let signature = body.read::<Signature>()?;
        // NB: The variant this value was read from is a container.
        Value::decode_as(body, signature, 1)
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
