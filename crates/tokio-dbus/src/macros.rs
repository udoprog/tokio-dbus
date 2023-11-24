/// Helper to efficiently repeat type parameters.
macro_rules! repeat {
    ($macro:path) => {
        $macro!(A);
        $macro!(A, B);
        $macro!(A, B, C);
        $macro!(A, B, C, D);
        $macro!(A, B, C, D, E);
        $macro!(A, B, C, D, E, F);
        $macro!(A, B, C, D, E, F, G);
        $macro!(A, B, C, D, E, F, G, H);
        $macro!(A, B, C, D, E, F, G, H, I);
        $macro!(A, B, C, D, E, F, G, H, I, J);
        $macro!(A, B, C, D, E, F, G, H, I, J, K);
        $macro!(A, B, C, D, E, F, G, H, I, J, K, L);
        $macro!(A, B, C, D, E, F, G, H, I, J, K, L, M);
        $macro!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
        $macro!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
        $macro!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);
    };
}

macro_rules! impl_traits_for_frame {
    ($ty:ty) => {
        impl $crate::ty::aligned::sealed::Sealed for $ty {}

        impl $crate::ty::Aligned for $ty {
            type Alignment = $ty;
        }

        impl $crate::ty::marker::sealed::Sealed for $ty {}

        impl $crate::ty::Marker for $ty {
            type Return<'de> = $ty;

            #[inline]
            fn load_struct<'de>(buf: &mut $crate::Body<'de>) -> $crate::Result<Self::Return<'de>> {
                buf.load()
            }

            #[inline]
            fn write_signature(
                signature: &mut $crate::signature::SignatureBuilder,
            ) -> Result<(), $crate::signature::SignatureError> {
                if !signature.extend_from_signature(<$ty as $crate::Frame>::SIGNATURE) {
                    return Err($crate::signature::SignatureError::too_long());
                }

                Ok(())
            }
        }

        impl $crate::arguments::sealed::Sealed for $ty {}

        impl $crate::arguments::Arguments for $ty {
            #[inline]
            fn extend_to(&self, buf: &mut $crate::BodyBuf) -> $crate::error::Result<()> {
                buf.store(*self)
            }

            #[inline]
            fn buf_to(&self, buf: &mut $crate::BodyBuf) {
                buf.store_frame(*self);
            }
        }

        impl $crate::storable::sealed::Sealed for $ty {}

        impl $crate::storable::Storable for $ty {
            #[inline]
            fn store_to(self, buf: &mut $crate::BodyBuf) {
                buf.store_frame(self)
            }

            #[inline]
            fn write_signature(signature: &mut $crate::signature::SignatureBuilder) -> bool {
                signature.extend_from_signature(<$ty as $crate::Frame>::SIGNATURE)
            }
        }
    };
}

macro_rules! impl_traits_for_write {
    ($ty:ty, $example:expr, $signature:expr $(, $import:ident)?) => {
        impl $crate::storable::sealed::Sealed for &$ty {}

        #[doc = concat!("[`Storable`] implementation for `&", stringify!($ty), "`.")]
        ///
        /// [`Storable`]: crate::Storable
        ///
        /// # Examples
        ///
        /// ```
        /// use tokio_dbus::BodyBuf;
        $(#[doc = concat!("use tokio_dbus::", stringify!($import), ";")])*
        ///
        /// let mut body = BodyBuf::new();
        ///
        /// body.store(10u16)?;
        #[doc = concat!("body.store(", stringify!($example) ,")?;")]
        ///
        #[doc = concat!("assert_eq!(body.signature(), ", stringify!($signature) ,");")]
        /// # Ok::<_, tokio_dbus::Error>(())
        /// ```
        impl $crate::storable::Storable for &$ty {
            #[inline]
            fn store_to(self, buf: &mut $crate::BodyBuf) {
                buf.write_only(self);
            }

            #[inline]
            fn write_signature(builder: &mut $crate::signature::SignatureBuilder) -> bool {
                builder.extend_from_signature(<$ty as $crate::write::Write>::SIGNATURE)
            }
        }

        impl $crate::arguments::sealed::Sealed for $ty {}

        impl $crate::arguments::Arguments for $ty {
            #[inline]
            fn extend_to(&self, buf: &mut $crate::BodyBuf) -> $crate::error::Result<()> {
                buf.store(self)
            }

            #[inline]
            fn buf_to(&self, buf: &mut $crate::BodyBuf) {
                Write::write_to(self, buf);
            }
        }
    };
}

macro_rules! impl_trait_unsized_marker {
    ($ty:ty, $type:ty, $return:ty, $signature:ident) => {
        impl $crate::ty::r#unsized::sealed::Sealed for $ty {}

        impl $crate::ty::r#unsized::Unsized for $ty {
            type Target = $return;
        }

        impl $crate::ty::aligned::sealed::Sealed for $ty {}

        impl $crate::ty::Aligned for $ty {
            type Alignment = $type;
        }

        impl $crate::ty::marker::sealed::Sealed for $ty {}

        impl $crate::ty::Marker for $ty {
            type Return<'de> = &'de $return;

            #[inline]
            fn load_struct<'de>(buf: &mut $crate::Body<'de>) -> $crate::Result<Self::Return<'de>> {
                buf.read()
            }

            #[inline]
            fn write_signature(
                signature: &mut $crate::signature::SignatureBuilder,
            ) -> Result<(), $crate::SignatureError> {
                if !signature.extend_from_signature($crate::Signature::$signature) {
                    return Err($crate::SignatureError::too_long());
                }

                Ok(())
            }
        }
    };
}

macro_rules! implement_remote {
    ($($ty:ty),* $(,)?) => {
        $(
            impl crate::frame::sealed::Sealed for $ty {}

            unsafe impl crate::frame::Frame for $ty {
                const SIGNATURE: &'static $crate::signature::Signature = <u8 as $crate::frame::Frame>::SIGNATURE;

                #[inline]
                fn adjust(&mut self, endianness: $crate::proto::Endianness) {
                    self.as_mut().adjust(endianness);
                }
            }

            impl_traits_for_frame!($ty);
        )*
    }
}

macro_rules! raw_enum {
    (
        $(#[$($meta:tt)*])*
        $vis:vis enum $name:ident { $($fields:tt)* }
    ) => {
        ::tokio_dbus_core::raw_enum! {
            $(#[$($meta)*])*
            $vis enum $name { $($fields)* }
        }

        implement_remote!($name);
    }
}

macro_rules! raw_set {
    (
        $(#[$($meta:tt)*])*
        $vis:vis enum $name:ident { $($fields:tt)* }
    ) => {
        ::tokio_dbus_core::raw_set! {
            $(#[$($meta)*])*
            $vis enum $name { $($fields)* }
        }

        implement_remote!($name);
    }
}
