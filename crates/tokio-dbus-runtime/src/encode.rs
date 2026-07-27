use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

use tokio_dbus::{Alignment, ObjectPath, ObjectPathBuf, Raw, Signature, SignatureBuf};

/// A Rust value which can be written to a D-Bus message body.
///
/// This is implemented for the owned types that generated code uses, and for
/// their borrowed counterparts so that a client does not have to allocate in
/// order to make a call.
///
/// # Examples
///
/// ```
/// use tokio_dbus::Signature;
/// use tokio_dbus_runtime::Arguments;
///
/// let mut arguments = Arguments::new(Signature::new("sas")?)?;
/// arguments.store("Hello");
/// arguments.store(&["a", "b"][..]);
/// # Ok::<_, tokio_dbus_runtime::Error>(())
/// ```
pub trait Encode {
    /// The alignment of the encoded value, which the containers this value is
    /// nested inside need in order to pad correctly.
    const ALIGNMENT: Alignment;

    /// Write the value, without writing its signature.
    fn encode(&self, raw: &mut Raw<'_>);
}

impl<T> Encode for &T
where
    T: ?Sized + Encode,
{
    const ALIGNMENT: Alignment = T::ALIGNMENT;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        (**self).encode(raw);
    }
}

macro_rules! encode_frame {
    ($($ty:ty, $alignment:ident),* $(,)?) => {
        $(
            impl Encode for $ty {
                const ALIGNMENT: Alignment = Alignment::$alignment;

                #[inline]
                fn encode(&self, raw: &mut Raw<'_>) {
                    raw.store(*self);
                }
            }
        )*
    }
}

encode_frame! {
    u8, BYTE,
    bool, U32,
    i16, U16,
    u16, U16,
    i32, U32,
    u32, U32,
    i64, U64,
    u64, U64,
    f64, U64,
}

impl Encode for str {
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        raw.store(self);
    }
}

impl Encode for String {
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        raw.store(self.as_str());
    }
}

impl Encode for ObjectPath {
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        raw.store(self);
    }
}

impl Encode for ObjectPathBuf {
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        raw.store(&**self);
    }
}

impl Encode for Signature {
    const ALIGNMENT: Alignment = Alignment::BYTE;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        raw.store(self);
    }
}

impl Encode for SignatureBuf {
    const ALIGNMENT: Alignment = Alignment::BYTE;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        raw.store(&**self);
    }
}

impl<T> Encode for [T]
where
    T: Encode,
{
    // NB: An array starts with a 32-bit length prefix, regardless of the
    // alignment of its elements.
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        let mut array = raw.store_array(T::ALIGNMENT);

        for value in self {
            value.encode(&mut array.as_raw());
        }
    }
}

impl<T> Encode for Vec<T>
where
    T: Encode,
{
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        <[T] as Encode>::encode(self, raw);
    }
}

impl<T, const N: usize> Encode for [T; N]
where
    T: Encode,
{
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        <[T] as Encode>::encode(self, raw);
    }
}

/// Write a map as an array of dict entries.
fn encode_entries<'a, K, V, I>(raw: &mut Raw<'_>, entries: I)
where
    K: 'a + Encode,
    V: 'a + Encode,
    I: IntoIterator<Item = (&'a K, &'a V)>,
{
    let mut array = raw.store_array(Alignment::U64);

    for (key, value) in entries {
        let mut entry = array.as_raw();
        // NB: Dict entries are aligned just like structs.
        entry.align(Alignment::U64);
        key.encode(&mut entry);
        value.encode(&mut entry);
    }
}

impl<K, V, S> Encode for HashMap<K, V, S>
where
    K: Encode,
    V: Encode,
    S: BuildHasher,
{
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        encode_entries(raw, self);
    }
}

impl<K, V> Encode for BTreeMap<K, V>
where
    K: Encode,
    V: Encode,
{
    const ALIGNMENT: Alignment = Alignment::U32;

    #[inline]
    fn encode(&self, raw: &mut Raw<'_>) {
        encode_entries(raw, self);
    }
}

macro_rules! encode_tuple {
    ($($ty:ident $var:ident),*) => {
        impl<$($ty,)*> Encode for ($($ty,)*)
        where
            $($ty: Encode,)*
        {
            // NB: Structs are aligned to 8 bytes.
            const ALIGNMENT: Alignment = Alignment::U64;

            #[inline]
            fn encode(&self, raw: &mut Raw<'_>) {
                let ($($var,)*) = self;
                raw.align(Alignment::U64);
                $($var.encode(raw);)*
            }
        }
    }
}

encode_tuple!(A a);
encode_tuple!(A a, B b);
encode_tuple!(A a, B b, C c);
encode_tuple!(A a, B b, C c, D d);
encode_tuple!(A a, B b, C c, D d, E e);
encode_tuple!(A a, B b, C c, D d, E e, F f);
encode_tuple!(A a, B b, C c, D d, E e, F f, G g);
encode_tuple!(A a, B b, C c, D d, E e, F f, G g, H h);
encode_tuple!(A a, B b, C c, D d, E e, F f, G g, H h, I i);
encode_tuple!(A a, B b, C c, D d, E e, F f, G g, H h, I i, J j);
encode_tuple!(A a, B b, C c, D d, E e, F f, G g, H h, I i, J j, K k);
encode_tuple!(A a, B b, C c, D d, E e, F f, G g, H h, I i, J j, K k, L l);
