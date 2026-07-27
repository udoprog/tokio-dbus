use crate::signature::SignatureBuilder;
use crate::{ObjectPath, Signature, Storable, WriteAligned};

/// A variant holding a value of a basic D-Bus type.
///
/// A variant is a value which is prefixed by the [`Signature`] of the value it
/// contains, which allows a container to hold values of a type which is not
/// known ahead of time.
///
/// Containers inside of a variant cannot be represented by this type. To write
/// one use [`BodyBuf::store_variant`], and to read one use
/// [`Body::skip_variant`] to skip over it.
///
/// [`BodyBuf::store_variant`]: crate::BodyBuf::store_variant
/// [`Body::skip_variant`]: crate::Body::skip_variant
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Variant};
///
/// let mut body = BodyBuf::new();
///
/// body.store(Variant::String("Hello World!"))?;
/// body.store(Variant::Bool(true))?;
///
/// assert_eq!(body.signature(), "vv");
///
/// let mut body = body.as_body();
/// assert!(matches!(body.read_variant()?, Variant::String("Hello World!")));
/// assert!(matches!(body.read_variant()?, Variant::Bool(true)));
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Variant<'de> {
    /// A boolean variant.
    Bool(bool),
    /// A single byte variant.
    U8(u8),
    /// A signed 16-bit variant.
    I16(i16),
    /// An unsigned 16-bit variant.
    U16(u16),
    /// A signed 32-bit variant.
    I32(i32),
    /// An unsigned 32-bit variant.
    U32(u32),
    /// A signed 64-bit variant.
    I64(i64),
    /// An unsigned 64-bit variant.
    U64(u64),
    /// A 64-bit floating point variant.
    F64(f64),
    /// A string variant.
    String(&'de str),
    /// An object path variant.
    ObjectPath(&'de ObjectPath),
    /// A stored signature.
    Signature(&'de Signature),
}

impl Variant<'_> {
    /// The signature of the value contained in this variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{Signature, Variant};
    ///
    /// assert_eq!(Variant::Bool(true).contained_signature(), Signature::BOOLEAN);
    /// assert_eq!(Variant::String("hi").contained_signature(), Signature::STRING);
    /// ```
    pub fn contained_signature(&self) -> &'static Signature {
        match self {
            Variant::Bool(..) => Signature::BOOLEAN,
            Variant::U8(..) => Signature::BYTE,
            Variant::I16(..) => Signature::INT16,
            Variant::U16(..) => Signature::UINT16,
            Variant::I32(..) => Signature::INT32,
            Variant::U32(..) => Signature::UINT32,
            Variant::I64(..) => Signature::INT64,
            Variant::U64(..) => Signature::UINT64,
            Variant::F64(..) => Signature::DOUBLE,
            Variant::String(..) => Signature::STRING,
            Variant::ObjectPath(..) => Signature::OBJECT_PATH,
            Variant::Signature(..) => Signature::SIGNATURE,
        }
    }
}

impl crate::storable::sealed::Sealed for Variant<'_> {}

impl Storable for Variant<'_> {
    #[inline]
    fn store_to<B>(self, buf: &mut B)
    where
        B: ?Sized + WriteAligned,
    {
        buf.write_only(self.contained_signature());

        match self {
            Variant::Bool(value) => buf.store_frame(u32::from(value)),
            Variant::U8(value) => buf.store_frame(value),
            Variant::I16(value) => buf.store_frame(value),
            Variant::U16(value) => buf.store_frame(value),
            Variant::I32(value) => buf.store_frame(value),
            Variant::U32(value) => buf.store_frame(value),
            Variant::I64(value) => buf.store_frame(value),
            Variant::U64(value) => buf.store_frame(value),
            Variant::F64(value) => buf.store_frame(value),
            Variant::String(value) => buf.write_only(value),
            Variant::ObjectPath(value) => buf.write_only(value),
            Variant::Signature(value) => buf.write_only(value),
        }
    }

    #[inline]
    fn write_signature(builder: &mut SignatureBuilder) -> bool {
        builder.extend_from_signature(Signature::VARIANT)
    }
}
