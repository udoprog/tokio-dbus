use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasher, Hash};

use tokio_dbus::{Alignment, Body, ObjectPath, ObjectPathBuf, Signature, SignatureBuf};

use crate::Result;

/// A Rust value which can be read from a D-Bus message body.
///
/// # Examples
///
/// ```
/// use tokio_dbus::{BodyBuf, Signature};
/// use tokio_dbus_runtime::Decode;
///
/// let mut buf = BodyBuf::new();
/// buf.extend_signature(Signature::new("su")?)?;
/// buf.raw().store("Hello");
/// buf.raw().store(42u32);
///
/// let mut body = buf.as_body();
/// assert_eq!(String::decode(&mut body)?, "Hello");
/// assert_eq!(u32::decode(&mut body)?, 42);
/// # Ok::<_, tokio_dbus_runtime::Error>(())
/// ```
pub trait Decode: Sized {
    /// The alignment of the encoded value.
    const ALIGNMENT: Alignment;

    /// Read one value from the body.
    fn decode(body: &mut Body<'_>) -> Result<Self>;
}

macro_rules! decode_frame {
    ($($ty:ty, $alignment:ident),* $(,)?) => {
        $(
            impl Decode for $ty {
                const ALIGNMENT: Alignment = Alignment::$alignment;

                #[inline]
                fn decode(body: &mut Body<'_>) -> Result<Self> {
                    Ok(body.load::<$ty>()?)
                }
            }
        )*
    }
}

decode_frame! {
    u8, BYTE,
    i16, U16,
    u16, U16,
    i32, U32,
    u32, U32,
    i64, U64,
    u64, U64,
    f64, U64,
}

impl Decode for bool {
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn decode(body: &mut Body<'_>) -> Result<Self> {
        Ok(body.load_bool()?)
    }
}

impl Decode for String {
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn decode(body: &mut Body<'_>) -> Result<Self> {
        Ok(body.read::<str>()?.to_owned())
    }
}

impl Decode for ObjectPathBuf {
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn decode(body: &mut Body<'_>) -> Result<Self> {
        Ok(body.read::<ObjectPath>()?.to_owned())
    }
}

impl Decode for SignatureBuf {
    const ALIGNMENT: Alignment = Alignment::BYTE;

    #[inline]
    fn decode(body: &mut Body<'_>) -> Result<Self> {
        Ok(body.read::<Signature>()?.to_owned())
    }
}

impl<T> Decode for Vec<T>
where
    T: Decode,
{
    const ALIGNMENT: Alignment = Alignment::U32;

    fn decode(body: &mut Body<'_>) -> Result<Self> {
        let mut array = body.load_raw_array(T::ALIGNMENT)?;
        let mut out = Vec::new();

        while !array.is_empty() {
            out.push(T::decode(&mut array)?);
        }

        Ok(out)
    }
}

/// Read an array of dict entries.
fn decode_entries<K, V, O>(body: &mut Body<'_>, mut insert: impl FnMut(&mut O, K, V)) -> Result<O>
where
    K: Decode,
    V: Decode,
    O: Default,
{
    // NB: Dict entries are aligned just like structs.
    let mut array = body.load_raw_array(Alignment::U64)?;
    let mut out = O::default();

    while !array.is_empty() {
        array.align_to(Alignment::U64)?;
        let key = K::decode(&mut array)?;
        let value = V::decode(&mut array)?;
        insert(&mut out, key, value);
    }

    Ok(out)
}

impl<K, V, S> Decode for HashMap<K, V, S>
where
    K: Decode + Eq + Hash,
    V: Decode,
    S: Default + BuildHasher,
{
    const ALIGNMENT: Alignment = Alignment::U32;

    fn decode(body: &mut Body<'_>) -> Result<Self> {
        decode_entries(body, |out: &mut Self, key, value| {
            out.insert(key, value);
        })
    }
}

impl<K, V> Decode for BTreeMap<K, V>
where
    K: Decode + Ord,
    V: Decode,
{
    const ALIGNMENT: Alignment = Alignment::U32;

    fn decode(body: &mut Body<'_>) -> Result<Self> {
        decode_entries(body, |out: &mut Self, key, value| {
            out.insert(key, value);
        })
    }
}

macro_rules! decode_tuple {
    ($($ty:ident),*) => {
        impl<$($ty,)*> Decode for ($($ty,)*)
        where
            $($ty: Decode,)*
        {
            // NB: Structs are aligned to 8 bytes.
            const ALIGNMENT: Alignment = Alignment::U64;

            #[inline]
            fn decode(body: &mut Body<'_>) -> Result<Self> {
                body.align_to(Alignment::U64)?;
                Ok(($($ty::decode(body)?,)*))
            }
        }
    }
}

decode_tuple!(A);
decode_tuple!(A, B);
decode_tuple!(A, B, C);
decode_tuple!(A, B, C, D);
decode_tuple!(A, B, C, D, E);
decode_tuple!(A, B, C, D, E, F);
decode_tuple!(A, B, C, D, E, F, G);
decode_tuple!(A, B, C, D, E, F, G, H);
decode_tuple!(A, B, C, D, E, F, G, H, I);
decode_tuple!(A, B, C, D, E, F, G, H, I, J);
decode_tuple!(A, B, C, D, E, F, G, H, I, J, K);
decode_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
