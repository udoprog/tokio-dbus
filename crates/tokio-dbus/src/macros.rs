/// Helper to efficiently repeat type parameters.
macro_rules! repeat {
    ($macro:path) => {
        $macro!(_A);
        $macro!(_A, _B);
        $macro!(_A, _B, _C);
        $macro!(_A, _B, _C, _D);
        $macro!(_A, _B, _C, _D, _E);
        $macro!(_A, _B, _C, _D, _E, _F);
        $macro!(_A, _B, _C, _D, _E, _F, _G);
        $macro!(_A, _B, _C, _D, _E, _F, _G, _H);
        $macro!(_A, _B, _C, _D, _E, _F, _G, _H, _I);
        $macro!(_A, _B, _C, _D, _E, _F, _G, _H, _I, _J);
        $macro!(_A, _B, _C, _D, _E, _F, _G, _H, _I, _J, _K);
        $macro!(_A, _B, _C, _D, _E, _F, _G, _H, _I, _J, _K, _L);
        $macro!(_A, _B, _C, _D, _E, _F, _G, _H, _I, _J, _K, _L, _M);
        $macro!(_A, _B, _C, _D, _E, _F, _G, _H, _I, _J, _K, _L, _M, _N);
        $macro!(_A, _B, _C, _D, _E, _F, _G, _H, _I, _J, _K, _L, _M, _N, _O);
        $macro!(
            _A, _B, _C, _D, _E, _F, _G, _H, _I, _J, _K, _L, _M, _N, _O, _P
        );
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
            fn extend_to<B>(&self, buf: &mut B) -> $crate::error::Result<()>
            where
                B: ?Sized + $crate::WriteAligned,
            {
                buf.store(*self)
            }

            #[inline]
            fn buf_to<B>(&self, buf: &mut B)
            where
                B: ?Sized + $crate::WriteAligned,
            {
                buf.store_frame(*self);
            }
        }

        impl $crate::storable::sealed::Sealed for $ty {}

        impl $crate::storable::Storable for $ty {
            #[inline]
            fn store_to<B>(self, buf: &mut B)
            where
                B: ?Sized + $crate::WriteAligned,
            {
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
            fn store_to<B>(self, buf: &mut B)
            where
                B: ?Sized + $crate::WriteAligned,
            {
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
            fn extend_to<B>(&self, buf: &mut B) -> $crate::error::Result<()>
            where
                B: ?Sized + $crate::WriteAligned,
            {
                buf.store(self)
            }

            #[inline]
            fn buf_to<B>(&self, buf: &mut B)
            where
                B: ?Sized + $crate::WriteAligned,
            {
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
    ($remote:ty, $($ty:ty),* $(,)?) => {
        $(
            impl crate::frame::sealed::Sealed for $ty {}

            unsafe impl crate::frame::Frame for $ty {
                const SIGNATURE: &'static $crate::signature::Signature = <$remote as $crate::frame::Frame>::SIGNATURE;

                #[inline]
                fn adjust(&mut self, endianness: $crate::proto::Endianness) {
                    self.private_mut().adjust(endianness);
                }
            }

            impl_traits_for_frame!($ty);
        )*
    }
}

macro_rules! raw_enum {
    (
        $(#[doc = $doc:literal])*
        #[repr($repr:ty)]
        $vis:vis enum $name:ident { $($fields:tt)* }
    ) => {
        ::tokio_dbus_core::raw_enum! {
            $(#[doc = $doc])*
            #[repr($repr)]
            $vis enum $name { $($fields)* }
        }

        implement_remote!($repr, $name);
    }
}

macro_rules! raw_set {
    (
        $(#[doc = $doc:literal])*
        #[repr($repr:ty)]
        $vis:vis enum $name:ident { $($fields:tt)* }
    ) => {
        ::tokio_dbus_core::raw_set! {
            $(#[doc = $doc])*
            #[repr($repr)]
            $vis enum $name { $($fields)* }
        }

        implement_remote!($repr, $name);
    }
}
