use crate::buf::Alloc;
use crate::{Alignment, BodyBuf, Signature, Storable};

/// A writer for values whose shape is only known at runtime.
///
/// The typed writers in this crate derive alignment and signatures from a
/// [`ty::Marker`], which requires the shape of a value to be known when the code
/// is written. A [`Raw`] writer instead takes the alignment of each container it
/// opens as an argument, which is what makes it usable from code that is generic
/// over, or generated for, arbitrary D-Bus types.
///
/// Nothing written through a [`Raw`] writer is reflected in the signature of the
/// underlying buffer. The signature is instead declared up front, when the
/// writer is constructed with [`BodyBuf::store_raw`].
///
/// [`ty::Marker`]: crate::ty::Marker
///
/// # Examples
///
/// ```
/// use tokio_dbus::{Alignment, BodyBuf, Signature};
///
/// let mut buf = BodyBuf::new();
///
/// let mut raw = buf.store_raw(Signature::new("a(us)")?)?;
/// let mut array = raw.store_array(Alignment::U64);
///
/// for (n, name) in [(1u32, "one"), (2u32, "two")] {
///     let mut entry = array.as_raw();
///     entry.align(Alignment::U64);
///     entry.store(n);
///     entry.store(name);
/// }
///
/// array.finish();
///
/// assert_eq!(buf.signature(), "a(us)");
/// # Ok::<_, tokio_dbus::Error>(())
/// ```
pub struct Raw<'a> {
    buf: &'a mut BodyBuf,
}

impl<'a> Raw<'a> {
    #[inline]
    pub(crate) fn new(buf: &'a mut BodyBuf) -> Self {
        Self { buf }
    }

    /// Reborrow this writer.
    ///
    /// This is needed to hand the writer to a function which takes it by value
    /// without giving up ownership of it.
    #[inline]
    pub fn as_raw(&mut self) -> Raw<'_> {
        Raw::new(self.buf)
    }

    /// Align the buffer to the given alignment.
    ///
    /// This must be called before the fields of a struct or a dict entry, both
    /// of which are aligned to [`Alignment::U64`].
    #[inline]
    pub fn align(&mut self, alignment: Alignment) {
        self.buf.align_mut_to(alignment.in_bytes());
    }

    /// Store a value, without recording anything in the signature of the
    /// buffer.
    #[inline]
    pub fn store<T>(&mut self, value: T)
    where
        T: Storable,
    {
        value.store_to(self.buf);
    }

    /// Write a signature inline, as the `g` type.
    ///
    /// A variant is a signature written this way followed by a value matching
    /// it.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokio_dbus::{BodyBuf, Signature, Variant};
    ///
    /// let mut buf = BodyBuf::new();
    ///
    /// let mut raw = buf.store_raw(Signature::VARIANT)?;
    /// raw.store_signature(Signature::UINT32);
    /// raw.store(42u32);
    ///
    /// let mut buf = buf.as_body();
    /// assert_eq!(buf.read_variant()?, Variant::U32(42));
    /// # Ok::<_, tokio_dbus::Error>(())
    /// ```
    #[inline]
    pub fn store_signature(&mut self, signature: &Signature) {
        self.buf.write_only(signature);
    }

    /// Extend the buffer with raw bytes, which is how the elements of a `ay` are
    /// written.
    #[inline]
    pub fn write_slice(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Open an array whose elements have the given alignment.
    ///
    /// The length of the array is written when the returned writer is finished
    /// or dropped.
    #[inline]
    pub fn store_array(&mut self, alignment: Alignment) -> RawArray<'_> {
        RawArray::new(self.buf, alignment)
    }

    /// Open an array, consuming this writer so that the array inherits its
    /// lifetime.
    ///
    /// This is what to use when the array has to outlive the writer it was
    /// opened from, such as when it is returned from a function.
    #[inline]
    pub fn into_array(self, alignment: Alignment) -> RawArray<'a> {
        RawArray::new(self.buf, alignment)
    }
}

/// A writer for the elements of an array whose shape is only known at runtime.
///
/// See [`Raw::store_array`].
pub struct RawArray<'a> {
    buf: &'a mut BodyBuf,
    len: Alloc<u32>,
    start: usize,
}

impl<'a> RawArray<'a> {
    #[inline]
    fn new(buf: &'a mut BodyBuf, alignment: Alignment) -> Self {
        let len = buf.alloc();
        // NB: The length prefix is followed by padding up to the alignment of
        // the element type, which is present even when the array is empty and is
        // not counted towards the length.
        buf.align_mut_to(alignment.in_bytes());
        let start = buf.len();
        Self { buf, len, start }
    }

    /// Write the next element of the array.
    #[inline]
    pub fn as_raw(&mut self) -> Raw<'_> {
        Raw::new(self.buf)
    }

    /// Finish writing the array.
    ///
    /// This also happens implicitly when the writer is dropped.
    #[inline]
    pub fn finish(self) {}
}

impl Drop for RawArray<'_> {
    #[inline]
    fn drop(&mut self) {
        let len = (self.buf.len() - self.start) as u32;
        self.buf.store_at(self.len, len);
    }
}
