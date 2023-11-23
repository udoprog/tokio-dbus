use std::marker::PhantomData;

/// An allocated location in the buffer that can be written to later.
pub struct Alloc<T>(usize, PhantomData<T>);

impl<T> Alloc<T> {
    pub(crate) fn new(at: usize) -> Self {
        Self(at, PhantomData)
    }

    pub(crate) fn into_usize(self) -> usize {
        self.0
    }
}

impl<T> Clone for Alloc<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Alloc<T> {}
