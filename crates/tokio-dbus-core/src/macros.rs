#[doc(hidden)]
#[macro_export]
macro_rules! raw_enum {
    (
        $(#[doc = $doc:literal])*
        #[repr($repr:ty)]
        $vis:vis enum $name:ident {
            $(
                $(#[$($variant_meta:meta)*])*
                $variant:ident = $value:expr
            ),* $(,)?
        }
    ) => {
        $(#[doc = $doc])*
        #[derive(Clone, Copy, PartialEq, Eq)]
        #[repr(transparent)]
        $vis struct $name($repr);

        impl $name {
            /// Construct a new instance using the underlying repr.
            #[doc(hidden)]
            pub const fn new(value: $repr) -> Self {
                Self(value)
            }

            /// Access the underlying representation mutably.
            #[doc(hidden)]
            pub fn as_mut(&mut self) -> &mut $repr {
                &mut self.0
            }
        }

        impl $name {
            $(
                $(#[$($variant_meta)*])*
                $vis const $variant: Self = Self($value);
            )*
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match *self {
                    $(Self::$variant => f.write_str(stringify!($variant)),)*
                    b => write!(f, "INVALID({:02x})", b.0),
                }
            }
        }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! raw_set {
    (
        $(#[doc = $doc:literal])*
        #[repr($repr:ty)]
        $vis:vis enum $name:ident {
            $(
                $(#[$($variant_meta:meta)*])*
                $variant:ident = $value:expr
            ),* $(,)?
        }
    ) => {
        $(#[doc = $doc])*
        #[derive(Default, Clone, Copy, PartialEq, Eq)]
        #[repr(transparent)]
        $vis struct $name($repr);

        impl $name {
            /// Construct a new instance using the underlying repr.
            #[doc(hidden)]
            pub const fn new(value: $repr) -> Self {
                Self(value)
            }

            /// Access the underlying representation mutably.
            #[doc(hidden)]
            pub fn as_mut(&mut self) -> &mut $repr {
                &mut self.0
            }
        }

        impl $name {
            $(
                $(#[$($variant_meta)*])*
                $vis const $variant: Self = Self($value);
            )*
        }

        impl ::core::fmt::Debug for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                struct Raw(&'static str);

                impl ::core::fmt::Debug for Raw {
                    #[inline]
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        write!(f, "{}", self.0)
                    }
                }

                struct Bits($repr);

                impl ::core::fmt::Debug for Bits {
                    #[inline]
                    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        write!(f, "{:b}", self.0)
                    }
                }

                let mut f = f.debug_set();

                let mut this = *self;

                $(
                    if this & Self::$variant {
                        f.entry(&Raw(stringify!($variant)));
                        this = this ^ Self::$variant;
                    }
                )*

                if this.0 != 0 {
                    f.entry(&Bits(this.0));
                }

                f.finish()
            }
        }

        impl ::core::ops::BitOr<$name> for $name {
            type Output = Self;

            #[inline]
            fn bitor(self, rhs: $name) -> Self::Output {
                Self(self.0 | rhs.0)
            }
        }

        impl ::core::ops::BitAnd<$name> for $name {
            type Output = bool;

            #[inline]
            fn bitand(self, rhs: $name) -> Self::Output {
                self.0 & rhs.0 != 0
            }
        }

        impl ::core::ops::BitXor<$name> for $name {
            type Output = Self;

            #[inline]
            fn bitxor(self, rhs: $name) -> Self::Output {
                Self(self.0 ^ rhs.0)
            }
        }
    }
}
