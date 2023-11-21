pub(crate) trait StackValue: Copy {
    const DEFAULT: Self;
}

pub(crate) struct Stack<T, const N: usize> {
    pub(crate) data: [T; N],
    pub(crate) len: usize,
}

macro_rules! stack_try_push {
    ($stack:expr, $value:expr) => {
        if $stack.len == $stack.capacity() {
            false
        } else {
            $stack.data[$stack.len] = $value;
            $stack.len += 1;
            true
        }
    };
}

macro_rules! stack_pop {
    ($stack:expr, $ty:ty) => {
        if $stack.len == 0 {
            None
        } else {
            let new_len = $stack.len - 1;
            $stack.len = new_len;
            let value = $stack.data[new_len];
            $stack.data[new_len] = <$ty>::DEFAULT;
            Some(value)
        }
    };
}

macro_rules! stack_peek {
    ($stack:expr) => {
        if $stack.len == 0 {
            None
        } else {
            Some(&$stack.data[$stack.len - 1])
        }
    };
}

impl<T, const N: usize> Stack<T, N>
where
    T: StackValue,
{
    pub const fn new() -> Self {
        Self {
            data: [T::DEFAULT; N],
            len: 0,
        }
    }

    #[inline]
    pub(crate) const fn capacity(&self) -> usize {
        N
    }
}
