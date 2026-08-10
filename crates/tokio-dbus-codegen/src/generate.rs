//! Generation of client proxies and server traits from an interface.

use genco::prelude::*;
use tokio_dbus_core::signature::SignatureBuilder;
use tokio_dbus_xml::{Argument, Doc, Interface, Method, Property, Signal};

use crate::naming::{argument_name, module_name, pascal_case, snake_case};
use crate::types::{owned_type, parameter_type};
use crate::{Mode, Result};

/// The concatenated signature of a list of arguments, which is what a message
/// body declares up front.
fn signature_of<'a>(arguments: impl IntoIterator<Item = &'a Argument<'a>>) -> Result<String> {
    let mut builder = SignatureBuilder::new();

    for argument in arguments {
        if !builder.extend_from_signature(argument.ty) {
            return Err(crate::Error::from(
                tokio_dbus_core::signature::SignatureError::too_long(),
            ));
        }
    }

    Ok(builder.to_signature().as_str().to_owned())
}

/// A `Signature::new_const(b"...")` expression.
fn signature(signature: &str) -> rust::Tokens {
    quote!(Signature::new_const($(format!("b{signature:?}"))))
}

/// Emit documentation comments.
///
/// These are written literally rather than through the `quote!` macro, which
/// would turn them into `#[doc]` attributes.
fn lines<S>(lines: impl IntoIterator<Item = S>) -> rust::Tokens
where
    S: AsRef<str>,
{
    let mut tokens = rust::Tokens::new();

    for line in lines {
        let line = line.as_ref();

        if line.is_empty() {
            tokens.append(quote!($("///")));
        } else {
            tokens.append(quote!($(format!("/// {line}"))));
        }

        tokens.push();
    }

    tokens
}

/// Emit documentation comments for an element, preceded by a generated summary.
fn doc(doc: &Doc<'_>, extra: impl IntoIterator<Item = String>) -> rust::Tokens {
    let mut tokens = lines(extra);
    tokens.append(lines(doc.lines()));
    tokens
}

/// Generate the module for a single interface.
pub(crate) fn interface(interface: &Interface<'_>, mode: Mode) -> Result<rust::Tokens> {
    let module = module_name(interface.name);
    let name = pascal_case(interface.name);

    let header = doc(
        &interface.doc,
        [format!("Generated bindings for `{}`.", interface.name)],
    );

    let client = if mode.client {
        Some(client(interface, &name)?)
    } else {
        None
    };

    let signals = if interface.signals.is_empty() {
        None
    } else {
        Some(signals(interface)?)
    };

    let server = if mode.server {
        Some(server(interface, &name)?)
    } else {
        None
    };

    let match_rule = format!("type='signal',interface='{}'", interface.name);

    Ok(quote! {
        $header
        pub mod $module {
            #![allow(
                dead_code,
                unused_imports,
                unused_variables,
                clippy::too_many_arguments,
                clippy::type_complexity
            )]

            use std::collections::HashMap;

            use tokio_dbus_runtime::export::{ObjectPath, ObjectPathBuf, Signature, SignatureBuf};
            use tokio_dbus_runtime::{
                Arguments, Call, Connection, Decode, Encode, Error, Result, SignalMessage, Value,
            };

            $(lines(["The name of this interface."]))
            pub const INTERFACE: &str = $(quoted(interface.name));

            $(lines(["A match rule selecting every signal emitted by this interface."]))
            pub const MATCH_RULE: &str = $(quoted(match_rule));

            $(lines(["The standard interface through which properties are read and written."]))
            const PROPERTIES: &str =
                tokio_dbus_runtime::export::org_freedesktop_dbus::PROPERTIES_INTERFACE;

            $(if let Some(client) = client { $client })

            $(if let Some(signals) = signals { $signals })

            $(if let Some(server) = server { $server })
        }
    })
}

/// The parameter list and the statements which write them into an argument
/// list.
fn inputs(method: &Method<'_>) -> Result<(rust::Tokens, rust::Tokens, rust::Tokens)> {
    let mut parameters = rust::Tokens::new();
    let mut stores = rust::Tokens::new();
    let mut names = rust::Tokens::new();

    for (index, argument) in method.inputs().enumerate() {
        let name = argument_name(argument.name, index);
        let ty = parameter_type(argument.ty.as_str())?;

        if index > 0 {
            parameters.append(quote!(,));
            parameters.space();
            names.append(quote!(,));
            names.space();
        }

        parameters.append(quote!($(name.clone()): $ty));
        names.append(quote!($(name.clone())));

        stores.append(quote!(__arguments.store($name);));
        stores.push();
    }

    Ok((parameters, stores, names))
}

/// The return type of a method, and the statements which read it.
fn outputs(method: &Method<'_>) -> Result<(rust::Tokens, rust::Tokens)> {
    let count = method.outputs().count();

    if count == 0 {
        return Ok((quote!(()), quote!(Ok(()))));
    }

    let mut types = rust::Tokens::new();
    let mut reads = rust::Tokens::new();

    for (index, argument) in method.outputs().enumerate() {
        let ty = owned_type(argument.ty.as_str())?;

        if index > 0 {
            types.append(quote!(,));
            types.space();
            reads.append(quote!(,));
            reads.space();
        }

        types.append(ty.clone());
        reads.append(quote!(<$ty as Decode>::decode(&mut __body)?));
    }

    if count == 1 {
        // NB: Returned directly rather than wrapped, since a single value is
        // already the return type of the method.
        let mut read = rust::Tokens::new();

        for argument in method.outputs() {
            let ty = owned_type(argument.ty.as_str())?;
            read.append(quote!(<$ty as Decode>::decode(&mut __body)));
        }

        return Ok((types, read));
    }

    Ok((quote!(($types)), quote!(Ok(($reads)))))
}

fn client(interface: &Interface<'_>, name: &str) -> Result<rust::Tokens> {
    let mut methods = rust::Tokens::new();

    for method in &interface.methods {
        methods.append(client_method(method)?);
        methods.line();
    }

    for property in &interface.properties {
        methods.append(client_property(property)?);
        methods.line();
    }

    if !interface.properties.is_empty() {
        methods.append(quote! {
            $(lines(["Read every property of this interface at once."]))
            pub async fn all_properties(
                &self,
                conn: &mut Connection,
            ) -> Result<HashMap<String, Value>> {
                let mut __arguments = Arguments::new_const(Signature::STRING);
                __arguments.store(INTERFACE);

                let __reply = conn
                    .call(&self.destination, &self.path, PROPERTIES, "GetAll", &__arguments)
                    .await?;

                __reply.read::<HashMap<String, Value>>()
            }
        });
    }

    Ok(quote! {
        $(lines([
            format!("An asynchronous client for `{}`.", interface.name),
            String::new(),
            String::from("Every call is made against the `destination` and `path` this client"),
            String::from("was constructed with, and is driven by the [`Connection`] passed to"),
            String::from("it."),
        ]))
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            destination: String,
            path: ObjectPathBuf,
        }

        impl $name {
            $(lines(["Construct a client which addresses `path` on `destination`."]))
            pub fn new(destination: impl AsRef<str>, path: &ObjectPath) -> Self {
                Self {
                    destination: destination.as_ref().to_owned(),
                    path: path.to_owned(),
                }
            }

            $(lines(["The name calls are sent to."]))
            pub fn destination(&self) -> &str {
                &self.destination
            }

            $(lines(["The object calls are sent to."]))
            pub fn path(&self) -> &ObjectPath {
                &self.path
            }

            $methods
        }
    })
}

fn client_method(method: &Method<'_>) -> Result<rust::Tokens> {
    let name = snake_case(method.name);
    let (parameters, stores, _) = inputs(method)?;
    let (ret, reads) = outputs(method)?;

    let in_signature = signature_of(method.inputs())?;
    let has_inputs = method.inputs().next().is_some();
    let has_outputs = method.outputs().next().is_some();

    let arguments = if has_inputs {
        let signature = signature(&in_signature);
        quote!(let mut __arguments = Arguments::new_const($signature);)
    } else {
        quote!(let __arguments = Arguments::empty();)
    };

    let documentation = doc(&method.doc, [format!("Call the `{}` method.", method.name)]);

    Ok(quote! {
        $documentation
        pub async fn $name(&self, conn: &mut Connection$(if has_inputs { , $parameters })) -> Result<$ret> {
            $arguments
            $stores

            let __reply = conn
                .call(
                    &self.destination,
                    &self.path,
                    INTERFACE,
                    $(quoted(method.name)),
                    &__arguments,
                )
                .await?;

            $(if has_outputs {
                let mut __body = __reply.body();
            } else {
                let _ = __reply;
            })
            $reads
        }
    })
}

fn client_property(property: &Property<'_>) -> Result<rust::Tokens> {
    let ty = owned_type(property.ty.as_str())?;
    let property_signature = signature(property.ty.as_str());
    let mut tokens = rust::Tokens::new();

    if property.access.is_readable() {
        let name = snake_case(property.name);

        let documentation = doc(
            &property.doc,
            [format!("Read the `{}` property.", property.name)],
        );

        tokens.append(quote! {
            $documentation
            pub async fn $name(&self, conn: &mut Connection) -> Result<$(ty.clone())> {
                let mut __arguments = Arguments::new_const($(signature("ss")));
                __arguments.store(INTERFACE);
                __arguments.store($(quoted(property.name)));

                let __reply = conn
                    .call(&self.destination, &self.path, PROPERTIES, "Get", &__arguments)
                    .await?;

                let mut __body = __reply.body();
                tokio_dbus_runtime::decode_variant::<$(ty.clone())>(
                    &mut __body,
                    $(property_signature.clone()),
                )
            }
        });

        tokens.line();
    }

    if property.access.is_writable() {
        let name = snake_case(&format!("set_{}", property.name));
        let parameter = parameter_type(property.ty.as_str())?;

        let documentation = doc(
            &property.doc,
            [format!("Write the `{}` property.", property.name)],
        );

        tokens.append(quote! {
            $documentation
            pub async fn $name(&self, conn: &mut Connection, value: $parameter) -> Result<()> {
                let mut __arguments = Arguments::new_const($(signature("ssv")));
                __arguments.store(INTERFACE);
                __arguments.store($(quoted(property.name)));
                __arguments.store_variant($property_signature, value);

                conn.call(&self.destination, &self.path, PROPERTIES, "Set", &__arguments)
                    .await?;

                Ok(())
            }
        });
    }

    Ok(tokens)
}

fn signals(interface: &Interface<'_>) -> Result<rust::Tokens> {
    let mut variants = rust::Tokens::new();
    let mut decodes = rust::Tokens::new();
    let mut members = rust::Tokens::new();
    let mut emits = rust::Tokens::new();

    for signal in &interface.signals {
        let variant = pascal_case(signal.name);
        let has_arguments = !signal.arguments.is_empty();

        let mut fields = rust::Tokens::new();
        let mut reads = rust::Tokens::new();
        let mut names = rust::Tokens::new();
        let mut stores = rust::Tokens::new();

        for (index, argument) in signal.arguments.iter().enumerate() {
            let name = argument_name(argument.name, index);
            let ty = owned_type(argument.ty.as_str())?;

            fields.append(doc(&argument.doc, None));
            fields.append(quote!($(name.clone()): $(ty.clone()),));
            fields.push();

            reads.append(quote!($(name.clone()): <$ty as Decode>::decode(&mut __body)?,));
            reads.push();

            if index > 0 {
                names.append(quote!(,));
                names.space();
            }

            names.append(quote!($(name.clone())));
            stores.append(quote!(__arguments.store($name);));
            stores.push();
        }

        let documentation = doc(&signal.doc, [format!("The `{}` signal.", signal.name)]);

        if has_arguments {
            variants.append(quote! {
                $documentation
                $(variant.clone()) {
                    $fields
                },
            });
        } else {
            variants.append(quote! {
                $documentation
                $(variant.clone()),
            });
        }

        variants.push();

        if has_arguments {
            decodes.append(quote! {
                $(quoted(signal.name)) => Signal::$(variant.clone()) {
                    $reads
                },
            });
        } else {
            decodes.append(quote!($(quoted(signal.name)) => Signal::$(variant.clone()),));
        }

        decodes.push();

        let pattern = if has_arguments {
            quote!(Signal::$(variant.clone()) { $(names.clone()) })
        } else {
            quote!(Signal::$(variant.clone()))
        };

        let ignored = if has_arguments {
            quote!(Signal::$(variant.clone()) { .. })
        } else {
            quote!(Signal::$(variant.clone()))
        };

        members.append(quote!($ignored => $(quoted(signal.name)),));
        members.push();

        let signature = signature_of(signal.arguments.iter())?;

        let arguments = if has_arguments {
            let signature = self::signature(&signature);
            quote!(let mut __arguments = Arguments::new_const($signature);)
        } else {
            quote!(let __arguments = Arguments::empty();)
        };

        emits.append(quote! {
            $pattern => {
                $arguments
                $stores
                conn.emit(path, INTERFACE, $(quoted(signal.name)), &__arguments)
            }
        });

        emits.push();
    }

    Ok(quote! {
        $(lines(["A signal emitted by this interface."]))
        #[derive(Debug, Clone, PartialEq)]
        #[non_exhaustive]
        pub enum Signal {
            $variants
        }

        impl Signal {
            $(lines([
                "Decode a signal emitted by this interface.",
                "",
                "Returns `None` when the message belongs to another interface, or",
                "names a signal which was not in the interface file.",
            ]))
            pub fn decode(message: &SignalMessage) -> Result<Option<Self>> {
                if message.interface() != Some(INTERFACE) {
                    return Ok(None);
                }

                #[allow(unused_mut)]
                let mut __body = message.body();

                Ok(Some(match message.member() {
                    $decodes
                    _ => return Ok(None),
                }))
            }

            $(lines(["The member name of this signal."]))
            pub fn member(&self) -> &'static str {
                match self {
                    $members
                }
            }

            $(lines(["Emit this signal from `path`."]))
            pub fn emit(&self, conn: &mut Connection, path: &ObjectPath) -> Result<()> {
                match self {
                    $emits
                }
            }
        }
    })
}

fn server(interface: &Interface<'_>, name: &str) -> Result<rust::Tokens> {
    let trait_name = format!("{name}Server");

    let mut trait_items = rust::Tokens::new();
    let mut decoders = rust::Tokens::new();
    let mut arms = rust::Tokens::new();

    for method in &interface.methods {
        let name = snake_case(method.name);
        let count = method.inputs().count();

        let mut parameters = rust::Tokens::new();
        let mut types = rust::Tokens::new();
        let mut reads = rust::Tokens::new();
        let mut names = rust::Tokens::new();

        for (index, argument) in method.inputs().enumerate() {
            let argument_name = argument_name(argument.name, index);
            let ty = owned_type(argument.ty.as_str())?;

            if index > 0 {
                parameters.append(quote!(,));
                parameters.space();
                types.append(quote!(,));
                types.space();
                names.append(quote!(,));
                names.space();
            }

            parameters.append(quote!($(argument_name.clone()): $(ty.clone())));

            types.append(ty.clone());
            names.append(quote!($argument_name));

            reads.append(quote!(<$ty as Decode>::decode(&mut __body)?,));
            reads.push();
        }

        let (ret, _) = outputs(method)?;

        let documentation = doc(
            &method.doc,
            [format!("Handle a call to the `{}` method.", method.name)],
        );

        trait_items.append(quote! {
            $documentation
            async fn $(name.clone())(&mut self$(if count > 0 { , $parameters })) -> Result<$(ret.clone())>;
        });

        trait_items.line();

        // The reply, which depends on how many values the method returns.
        let out_count = method.outputs().count();
        let out_signature = signature_of(method.outputs())?;

        let (pattern, writes) = if out_count == 0 {
            (
                quote!(Ok(())),
                quote!(let __arguments = Arguments::empty();),
            )
        } else {
            let mut pattern = rust::Tokens::new();
            let mut writes = rust::Tokens::new();

            writes.append(
                quote!(let mut __arguments = Arguments::new_const($(signature(&out_signature)));),
            );
            writes.push();

            for index in 0..out_count {
                let value = format!("__out{index}");

                if index > 0 {
                    pattern.append(quote!(,));
                    pattern.space();
                }

                pattern.append(quote!($(value.clone())));
                writes.append(quote!(__arguments.store($value);));
                writes.push();
            }

            if out_count == 1 {
                (quote!(Ok($pattern)), writes)
            } else {
                (quote!(Ok(($pattern))), writes)
            }
        };

        let call = if count == 0 {
            quote!(handler.$(name.clone())().await)
        } else {
            let decoder = format!("__decode_{name}");

            decoders.append(quote! {
                $(lines([format!("Read the arguments of a call to `{}`.", method.name)]))
                fn $(decoder.clone())(call: &Call) -> Result<($types$(if count == 1 { , }))> {
                    let mut __body = call.body();
                    Ok(($reads))
                }
            });

            decoders.line();

            quote! {
                match $decoder(call) {
                    Ok(($(names.clone())$(if count == 1 { , }))) => {
                        handler.$(name.clone())($names).await
                    }
                    Err(__error) => Err(__error),
                }
            }
        };

        arms.append(quote! {
            $(quoted(method.name)) => {
                let __result = $call;

                match __result {
                    $pattern => {
                        $writes
                        conn.reply(call, &__arguments)?;
                    }
                    Err(__error) => {
                        conn.reply_error(call, &__error)?;
                    }
                }

                Ok(true)
            }
        });

        arms.push();
    }

    for property in &interface.properties {
        let ty = owned_type(property.ty.as_str())?;

        if property.access.is_readable() {
            let name = snake_case(property.name);

            let documentation = doc(
                &property.doc,
                [format!("Read the `{}` property.", property.name)],
            );

            trait_items.append(quote! {
                $documentation
                async fn $name(&mut self) -> Result<$(ty.clone())>;
            });

            trait_items.line();
        }

        if property.access.is_writable() {
            let name = snake_case(&format!("set_{}", property.name));

            let documentation = doc(
                &property.doc,
                [format!("Write the `{}` property.", property.name)],
            );

            trait_items.append(quote! {
                $documentation
                async fn $name(&mut self, value: $ty) -> Result<()>;
            });

            trait_items.line();
        }
    }

    let properties = server_properties(interface, &trait_name)?;
    let properties_changed = server_properties_changed(interface, &trait_name)?;

    Ok(quote! {
        $(lines([
            format!("A server implementation of `{}`.", interface.name),
            String::new(),
            String::from("Pass an implementation of this to [`dispatch`] to serve the"),
            String::from("interface."),
        ]))
        #[allow(async_fn_in_trait)]
        pub trait $(trait_name.clone()) {
            $trait_items
        }

        $decoders

        $(lines([
            format!("Handle a call addressed to `{}`.", interface.name),
            String::new(),
            String::from("Returns `true` when the call was handled, in which case a reply has"),
            String::from("already been written. A `false` return means the call was for another"),
            String::from("interface or an unknown member, and the caller should keep looking."),
        ]))
        pub async fn dispatch<T>(handler: &mut T, conn: &mut Connection, call: &Call) -> Result<bool>
        where
            T: ?Sized + $(trait_name.clone()),
        {
            match call.interface() {
                Some(INTERFACE) => {}
                Some(PROPERTIES) => return dispatch_properties(handler, conn, call).await,
                _ => return Ok(false),
            }

            #[allow(clippy::match_single_binding)]
            match call.member() {
                $arms
                _ => Ok(false),
            }
        }

        $properties

        $(if let Some(properties_changed) = properties_changed { $properties_changed })
    })
}

/// The `Property` enum and the `PropertiesChanged` emitters, generated when an
/// interface has readable properties.
fn server_properties_changed(
    interface: &Interface<'_>,
    trait_name: &str,
) -> Result<Option<rust::Tokens>> {
    let mut variants = rust::Tokens::new();
    let mut names = rust::Tokens::new();
    let mut entries = rust::Tokens::new();

    for property in &interface.properties {
        if !property.access.is_readable() {
            continue;
        }

        let variant = pascal_case(property.name);
        let getter = snake_case(property.name);
        let signature = signature(property.ty.as_str());

        variants.append(quote! {
            $(doc(&property.doc, [format!("The `{}` property.", property.name)]))
            $(variant.clone()),
        });

        variants.push();

        names.append(quote!(Property::$(variant.clone()) => $(quoted(property.name)),));
        names.push();

        entries.append(quote! {
            Property::$(variant.clone()) => {
                __changed.entry($(quoted(property.name)), $signature, handler.$getter().await?);
            }
        });

        entries.push();
    }

    if variants.is_empty() {
        return Ok(None);
    }

    Ok(Some(quote! {
        $(lines([
            "A readable property of this interface, for announcing a change",
            "with [`properties_changed()`] or [`properties_invalidated()`].",
        ]))
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Property {
            $variants
        }

        impl Property {
            $(lines(["The D-Bus name of this property."]))
            pub fn name(&self) -> &'static str {
                match self {
                    $names
                }
            }
        }

        $(lines([
            "Emit `org.freedesktop.DBus.Properties.PropertiesChanged` from `path`,",
            "announcing the new values of the `changed` properties.",
            "",
            "Each value is read back through `handler`, so that what is announced",
            "cannot disagree with what a subsequent `Get` would answer.",
            "",
            "The signal is buffered and written out the next time the connection",
            "makes progress, see [`Connection::emit`].",
        ]))
        pub async fn properties_changed<T>(
            handler: &mut T,
            conn: &mut Connection,
            path: &ObjectPath,
            changed: &[Property],
        ) -> Result<()>
        where
            T: ?Sized + $trait_name,
        {
            let mut __arguments = Arguments::new_const($(signature("sa{sv}as")));
            __arguments.store(INTERFACE);

            let mut __changed = __arguments.store_variant_dict();

            for __property in changed {
                match __property {
                    $entries
                }
            }

            __changed.finish();
            __arguments.store(Vec::<String>::new());

            conn.emit(path, PROPERTIES, "PropertiesChanged", &__arguments)
        }

        $(lines([
            "Emit `org.freedesktop.DBus.Properties.PropertiesChanged` from `path`,",
            "announcing that the `invalidated` properties changed without sending",
            "their new values.",
        ]))
        pub fn properties_invalidated(
            conn: &mut Connection,
            path: &ObjectPath,
            invalidated: &[Property],
        ) -> Result<()> {
            let mut __arguments = Arguments::new_const($(signature("sa{sv}as")));
            __arguments.store(INTERFACE);
            __arguments.store_variant_dict().finish();

            let mut __names = Vec::<String>::new();

            for __property in invalidated {
                __names.push(String::from(__property.name()));
            }

            __arguments.store(__names);

            conn.emit(path, PROPERTIES, "PropertiesChanged", &__arguments)
        }
    }))
}

fn server_properties(interface: &Interface<'_>, trait_name: &str) -> Result<rust::Tokens> {
    let mut gets = rust::Tokens::new();
    let mut sets = rust::Tokens::new();
    let mut reads = rust::Tokens::new();
    let mut entries = rust::Tokens::new();
    let mut patterns = rust::Tokens::new();
    let mut readable = 0usize;

    for property in &interface.properties {
        let signature = signature(property.ty.as_str());
        let ty = owned_type(property.ty.as_str())?;

        if property.access.is_readable() {
            let name = snake_case(property.name);

            gets.append(quote! {
                $(quoted(property.name)) => match handler.$(name.clone())().await {
                    Ok(__value) => {
                        let mut __arguments = Arguments::new_const(Signature::VARIANT);
                        __arguments.store_variant($(signature.clone()), __value);
                        conn.reply(call, &__arguments)?;
                    }
                    Err(__error) => {
                        conn.reply_error(call, &__error)?;
                    }
                },
            });

            gets.push();

            reads.append(quote!(handler.$name().await?,));
            reads.push();

            let value = format!("__value{readable}");

            if readable > 0 {
                patterns.append(quote!(,));
                patterns.space();
            }

            patterns.append(quote!($(value.clone())));

            entries.append(quote! {
                __dict.entry($(quoted(property.name)), $(signature.clone()), $value);
            });

            entries.push();
            readable += 1;
        }

        if property.access.is_writable() {
            let name = snake_case(&format!("set_{}", property.name));

            sets.append(quote! {
                $(quoted(property.name)) => {
                    let __result = match tokio_dbus_runtime::decode_variant::<$ty>(
                        &mut __body,
                        $signature,
                    ) {
                        Ok(__value) => handler.$name(__value).await,
                        Err(__error) => Err(__error),
                    };

                    match __result {
                        Ok(()) => {
                            conn.reply(call, &Arguments::empty())?;
                        }
                        Err(__error) => {
                            conn.reply_error(call, &__error)?;
                        }
                    }
                }
            });

            sets.push();
        }
    }

    let get_all = if readable == 0 {
        quote!(Ok(false))
    } else {
        let pattern = if readable == 1 {
            quote!(Ok(($patterns,)))
        } else {
            quote!(Ok(($patterns)))
        };

        quote! {
            // NB: Every value is read up front so that a failing property is
            // reported as an error reply instead of a half built dictionary.
            let __values = async { Ok::<_, Error>(($reads)) }.await;

            match __values {
                $pattern => {
                    let mut __arguments = Arguments::new_const($(signature("a{sv}")));
                    let mut __dict = __arguments.store_variant_dict();
                    $entries
                    __dict.finish();
                    conn.reply(call, &__arguments)?;
                }
                Err(__error) => {
                    conn.reply_error(call, &__error)?;
                }
            }

            Ok(true)
        }
    };

    let get = if gets.is_empty() {
        quote!(Ok(false))
    } else {
        quote! {
            let __name = <String as Decode>::decode(&mut __body)?;

            match __name.as_str() {
                $gets
                _ => return Ok(false),
            }

            Ok(true)
        }
    };

    let set = if sets.is_empty() {
        quote!(Ok(false))
    } else {
        quote! {
            let __name = <String as Decode>::decode(&mut __body)?;

            match __name.as_str() {
                $sets
                _ => return Ok(false),
            }

            Ok(true)
        }
    };

    Ok(quote! {
        $(lines([
            "Handle a call to `org.freedesktop.DBus.Properties` which concerns",
            "this interface.",
        ]))
        async fn dispatch_properties<T>(
            handler: &mut T,
            conn: &mut Connection,
            call: &Call,
        ) -> Result<bool>
        where
            T: ?Sized + $trait_name,
        {
            #[allow(unused_mut, unused_variables)]
            let mut __body = call.body();

            let __interface = match <String as Decode>::decode(&mut __body) {
                Ok(__interface) => __interface,
                Err(..) => return Ok(false),
            };

            if __interface != INTERFACE {
                return Ok(false);
            }

            match call.member() {
                "Get" => {
                    $get
                }
                "GetAll" => {
                    $get_all
                }
                "Set" => {
                    $set
                }
                _ => Ok(false),
            }
        }
    })
}

/// Convenience for the unused `Signal` import when an interface has none.
#[allow(dead_code)]
fn unused(_: &Signal<'_>) {}
