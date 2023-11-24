use core::mem::MaybeUninit;
use core::ptr;

#[doc(hidden)]
pub(crate) struct Stack<T, const N: usize> {
    pub(crate) data: [MaybeUninit<T>; N],
    pub(crate) len: usize,
}

impl<T, const N: usize> Stack<T, N>
where
    T: Copy,
{
    pub(super) const fn new() -> Self {
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    #[inline]
    pub(super) const fn capacity(&self) -> usize {
        N
    }

    #[inline]
    pub(super) fn try_push(&mut self, value: T) -> bool {
        if self.len == self.capacity() {
            return false;
        }

        // SAFETY: We're writing to an uninitialized slice.
        unsafe {
            self.data
                .as_mut_ptr()
                .cast::<T>()
                .add(self.len)
                .write(value);
        }

        self.len += 1;
        true
    }

    #[inline]
    pub(super) fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        let new_len = self.len - 1;
        self.len = new_len;

        // SAFETY: We're reading into the known initialized slice.
        unsafe {
            let value = ptr::read(self.data.as_ptr().add(new_len));
            Some(value.assume_init())
        }
    }

    #[inline]
    pub(super) fn peek(&mut self) -> Option<&T> {
        if self.len == 0 {
            return None;
        }

        // SAFETY: Since len defines the initialized slice, we can safely read a
        // reference to it here.
        unsafe { Some(&*self.data.as_ptr().add(self.len - 1).cast::<T>()) }
    }
}
