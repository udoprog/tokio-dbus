use std::fmt::Write;

use tokio_dbus_core::signature::Signature;
use xmlparser::{ElementEnd, Token};

use crate::error::ErrorKind;
use crate::{
    Access, Annotation, Argument, Description, Direction, Doc, Error, Interface, Method, Node,
    Property, Result, Signal,
};

/// Parse the contents of an interface file.
///
/// The format is the one described by the [D-Bus introspection format], which is
/// also what `org.freedesktop.DBus.Introspectable.Introspect` returns.
///
/// [D-Bus introspection format]:
///     https://dbus.freedesktop.org/doc/dbus-specification.html#introspection-format
///
/// # Examples
///
/// ```
/// use tokio_dbus_xml::{Access, parse_interface};
///
/// let node = parse_interface(r#"
/// <node>
///   <interface name="com.example.Example">
///     <method name="Add">
///       <arg name="a" type="i" direction="in"/>
///       <arg name="b" type="i" direction="in"/>
///       <arg name="sum" type="i" direction="out"/>
///     </method>
///     <signal name="Changed">
///       <arg name="value" type="i"/>
///     </signal>
///     <property name="Total" type="i" access="read"/>
///   </interface>
/// </node>
/// "#)?;
///
/// let interface = node.interface("com.example.Example").expect("Missing interface");
///
/// assert_eq!(interface.methods[0].name, "Add");
/// assert_eq!(interface.methods[0].inputs().count(), 2);
/// assert_eq!(interface.methods[0].outputs().count(), 1);
/// assert_eq!(interface.signals[0].name, "Changed");
/// assert_eq!(interface.properties[0].access, Access::Read);
/// # Ok::<_, tokio_dbus_xml::Error>(())
/// ```
pub fn parse_interface(interface: &str) -> Result<Node<'_>> {
    let tokenizer = xmlparser::Tokenizer::from(interface);

    let mut stack = vec![];
    let mut path = String::new();
    let mut root = NodeBuilder::default();

    macro_rules! expect_end {
        ($end:expr, $expected:literal) => {
            if let Some(end) = $end {
                if end != $expected {
                    return Err(Error::new(
                        path,
                        ErrorKind::MismatchingEnd {
                            expected: $expected.into(),
                            actual: end.into(),
                        },
                    ));
                }
            }
        };
    }

    for token in tokenizer {
        let token = match token {
            Ok(token) => token,
            Err(error) => return Err(Error::new(path, error)),
        };

        match token {
            Token::ElementStart { local, .. } => {
                match (stack.last(), local.as_str()) {
                    (None | Some(State::Node(..)), "node") => {
                        stack.push(State::Node(NodeBuilder::default()));
                    }
                    (Some(State::Node(..)), "interface") => {
                        stack.push(State::Interface(InterfaceBuilder::default()));
                    }
                    (Some(State::Interface(..)), "method") => {
                        stack.push(State::Method(MethodBuilder::default()));
                    }
                    (Some(State::Interface(..)), "signal") => {
                        stack.push(State::Signal(SignalBuilder::default()));
                    }
                    (Some(State::Interface(..)), "property") => {
                        stack.push(State::Property(PropertyBuilder::default()));
                    }
                    (Some(State::Method(..) | State::Signal(..)), "arg") => {
                        stack.push(State::Argument(ArgumentBuilder::default()));
                    }
                    (
                        Some(
                            State::Interface(..)
                            | State::Method(..)
                            | State::Signal(..)
                            | State::Property(..)
                            | State::Argument(..),
                        ),
                        "annotation",
                    ) => {
                        stack.push(State::Annotation(AnnotationBuilder::default()));
                    }
                    (
                        Some(
                            State::Interface(..)
                            | State::Method(..)
                            | State::Signal(..)
                            | State::Property(..)
                            | State::Argument(..),
                        ),
                        "doc",
                    ) => {
                        stack.push(State::Doc(Doc::default()));
                    }
                    (Some(State::Doc(..)), "summary") => {
                        stack.push(State::String("summary", StringBuilder::default()));
                    }
                    (Some(State::Doc(..)), "description") => {
                        stack.push(State::Description(Description::default()));
                    }
                    (Some(State::Description(..)), "para") => {
                        stack.push(State::String("para", StringBuilder::default()));
                    }
                    (_, element) => {
                        return Err(Error::new(
                            path,
                            ErrorKind::UnsupportedElementStart(element.into()),
                        ));
                    }
                }

                if !path.is_empty() {
                    path.push('/');
                }

                path.push_str(local.as_str());

                match &stack[..] {
                    [.., State::Node(node), State::Node(..)] => {
                        let _ = write!(path, "[{}]", node.nodes.len());
                    }
                    [.., State::Node(node), State::Interface(..)] => {
                        let _ = write!(path, "[{}]", node.interfaces.len());
                    }
                    [.., State::Interface(interface), State::Method(..)] => {
                        let _ = write!(path, "[{}]", interface.methods.len());
                    }
                    [.., State::Interface(interface), State::Signal(..)] => {
                        let _ = write!(path, "[{}]", interface.signals.len());
                    }
                    [.., State::Interface(interface), State::Property(..)] => {
                        let _ = write!(path, "[{}]", interface.properties.len());
                    }
                    [.., State::Method(method), State::Argument(..)] => {
                        let _ = write!(path, "[{}]", method.arguments.len());
                    }
                    [.., State::Signal(signal), State::Argument(..)] => {
                        let _ = write!(path, "[{}]", signal.arguments.len());
                    }
                    _ => {}
                }
            }
            Token::ElementEnd { end, .. } => {
                let name = match end {
                    ElementEnd::Open => {
                        continue;
                    }
                    ElementEnd::Close(_, name) => Some(name.as_str()),
                    ElementEnd::Empty => None,
                };

                let Some(top) = stack.pop() else {
                    return Err(Error::new(path, ErrorKind::UnsupportedElementEnd));
                };

                match (&mut stack[..], top) {
                    ([], State::Node(node)) => {
                        expect_end!(name, "node");
                        root.name = node.name;
                        root.interfaces.extend(node.interfaces);
                        root.nodes.extend(node.nodes);
                    }
                    ([.., State::Node(node)], State::Node(builder)) => {
                        expect_end!(name, "node");
                        node.nodes.push(builder.build());
                    }
                    ([.., State::Node(node)], State::Interface(builder)) => {
                        expect_end!(name, "interface");
                        node.interfaces.push(
                            builder
                                .build()
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    ([.., State::Interface(interface)], State::Method(builder)) => {
                        expect_end!(name, "method");
                        interface.methods.push(
                            builder
                                .build()
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    ([.., State::Interface(interface)], State::Signal(builder)) => {
                        expect_end!(name, "signal");
                        interface.signals.push(
                            builder
                                .build()
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    ([.., State::Interface(interface)], State::Property(builder)) => {
                        expect_end!(name, "property");
                        interface.properties.push(
                            builder
                                .build()
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    ([.., State::Method(method)], State::Argument(builder)) => {
                        expect_end!(name, "arg");
                        method.arguments.push(
                            builder
                                .build(Direction::In)
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    ([.., State::Signal(signal)], State::Argument(builder)) => {
                        expect_end!(name, "arg");
                        // NB: The arguments of a signal are always carried
                        // outwards, and the DTD does not permit a direction on
                        // them.
                        signal.arguments.push(
                            builder
                                .build(Direction::Out)
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    ([.., parent], State::Annotation(builder)) => {
                        expect_end!(name, "annotation");

                        let annotation = builder
                            .build()
                            .map_err(|kind| Error::new(path.as_str(), kind))?;

                        match parent {
                            State::Interface(interface) => interface.annotations.push(annotation),
                            State::Method(method) => method.annotations.push(annotation),
                            State::Signal(signal) => signal.annotations.push(annotation),
                            State::Property(property) => property.annotations.push(annotation),
                            // NB: An annotation on an argument carries nothing
                            // this crate models.
                            State::Argument(..) => {}
                            _ => return Err(Error::new(path, ErrorKind::UnsupportedElementEnd)),
                        }
                    }
                    ([.., parent], State::Doc(value)) => {
                        expect_end!(name, "doc");

                        match parent {
                            State::Interface(interface) => interface.doc = value,
                            State::Method(method) => method.doc = value,
                            State::Signal(signal) => signal.doc = value,
                            State::Property(property) => property.doc = value,
                            State::Argument(argument) => argument.doc = value,
                            _ => return Err(Error::new(path, ErrorKind::UnsupportedElementEnd)),
                        }
                    }
                    ([.., State::Doc(doc)], State::String("summary", string)) => {
                        expect_end!(name, "summary");
                        doc.summary = string.text;
                    }
                    ([.., State::Doc(doc)], State::Description(description)) => {
                        expect_end!(name, "description");
                        doc.description = description;
                    }
                    ([.., State::Description(description)], State::String("para", string)) => {
                        expect_end!(name, "para");
                        description.paragraph = string.text;
                    }
                    _ => return Err(Error::new(path, ErrorKind::UnsupportedElementEnd)),
                }

                if let Some(index) = path.rfind('/') {
                    path.truncate(index);
                } else {
                    path.clear();
                }
            }
            Token::Attribute {
                prefix,
                local,
                value,
                ..
            } => {
                let len = path.len();
                path.push(':');
                path.push_str(local.as_str());

                match (&mut stack[..], prefix.as_str(), local.as_str()) {
                    ([State::Node(..)], "xmlns", _) => {
                        // ignore xmlns attributes, while these would be good to
                        // validate, in practice they don't make much of a
                        // difference and are rarely used.
                    }
                    ([.., State::Node(builder)], _, "name") => {
                        builder.name = Some(value.as_str());
                    }
                    ([.., State::Interface(builder)], _, "name") => {
                        builder.name = Some(value.as_str());
                    }
                    ([.., State::Method(builder)], _, "name") => {
                        builder.name = Some(value.as_str());
                    }
                    ([.., State::Signal(builder)], _, "name") => {
                        builder.name = Some(value.as_str());
                    }
                    ([.., State::Property(builder)], _, "name") => {
                        builder.name = Some(value.as_str());
                    }
                    ([.., State::Property(builder)], _, "type") => {
                        builder.ty = Some(
                            Signature::new(value.as_str())
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    ([.., State::Property(builder)], _, "access") => {
                        builder.access = Some(match value.as_str() {
                            "read" => Access::Read,
                            "write" => Access::Write,
                            "readwrite" => Access::ReadWrite,
                            other => {
                                return Err(Error::new(
                                    path,
                                    ErrorKind::UnsupportedPropertyAccess(other.into()),
                                ));
                            }
                        });
                    }
                    ([.., State::Annotation(builder)], _, "name") => {
                        builder.name = Some(value.as_str());
                    }
                    ([.., State::Annotation(builder)], _, "value") => {
                        builder.value = Some(value.as_str());
                    }
                    ([.., State::Argument(builder)], _, "name") => {
                        builder.name = Some(value.as_str());
                    }
                    ([.., State::Argument(builder)], _, "direction") => {
                        builder.direction = Some(match value.as_str() {
                            "in" => Direction::In,
                            "out" => Direction::Out,
                            other => {
                                return Err(Error::new(
                                    path,
                                    ErrorKind::UnsupportedArgumentDirection(other.into()),
                                ));
                            }
                        });
                    }
                    ([.., State::Argument(builder)], _, "type") => {
                        builder.ty = Some(
                            Signature::new(value.as_str())
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    (_, _, name) => {
                        return Err(Error::new(
                            path,
                            ErrorKind::UnsupportedAttribute(name.into()),
                        ));
                    }
                }

                path.truncate(len);
            }
            Token::Text { text } => match stack.last_mut() {
                Some(State::String(_, string)) => {
                    string.text = Some(text.as_str());
                }
                _ => {
                    if !text.as_str().trim().is_empty() {
                        return Err(Error::new(path, ErrorKind::UnsupportedText));
                    }
                }
            },
            _ => {}
        }
    }

    Ok(root.build())
}

#[derive(Debug, Default)]
struct NodeBuilder<'a> {
    name: Option<&'a str>,
    interfaces: Vec<Interface<'a>>,
    nodes: Vec<Node<'a>>,
}

impl<'a> NodeBuilder<'a> {
    fn build(self) -> Node<'a> {
        Node {
            name: self.name,
            interfaces: self.interfaces.into(),
            nodes: self.nodes.into(),
        }
    }
}

#[derive(Debug, Default)]
struct InterfaceBuilder<'a> {
    name: Option<&'a str>,
    methods: Vec<Method<'a>>,
    signals: Vec<Signal<'a>>,
    properties: Vec<Property<'a>>,
    annotations: Vec<Annotation<'a>>,
    doc: Doc<'a>,
}

impl<'a> InterfaceBuilder<'a> {
    fn build(self) -> Result<Interface<'a>, ErrorKind> {
        let name = self.name.ok_or(ErrorKind::MissingInterfaceName)?;

        Ok(Interface {
            name,
            methods: self.methods.into(),
            signals: self.signals.into(),
            properties: self.properties.into(),
            annotations: self.annotations.into(),
            doc: self.doc,
        })
    }
}

#[derive(Debug, Default)]
struct MethodBuilder<'a> {
    name: Option<&'a str>,
    arguments: Vec<Argument<'a>>,
    annotations: Vec<Annotation<'a>>,
    doc: Doc<'a>,
}

impl<'a> MethodBuilder<'a> {
    fn build(self) -> Result<Method<'a>, ErrorKind> {
        let name = self.name.ok_or(ErrorKind::MissingMethodName)?;

        Ok(Method {
            name,
            arguments: self.arguments.into(),
            annotations: self.annotations.into(),
            doc: self.doc,
        })
    }
}

#[derive(Debug, Default)]
struct SignalBuilder<'a> {
    name: Option<&'a str>,
    arguments: Vec<Argument<'a>>,
    annotations: Vec<Annotation<'a>>,
    doc: Doc<'a>,
}

impl<'a> SignalBuilder<'a> {
    fn build(self) -> Result<Signal<'a>, ErrorKind> {
        let name = self.name.ok_or(ErrorKind::MissingSignalName)?;

        Ok(Signal {
            name,
            arguments: self.arguments.into(),
            annotations: self.annotations.into(),
            doc: self.doc,
        })
    }
}

#[derive(Debug, Default)]
struct PropertyBuilder<'a> {
    name: Option<&'a str>,
    ty: Option<&'a Signature>,
    access: Option<Access>,
    annotations: Vec<Annotation<'a>>,
    doc: Doc<'a>,
}

impl<'a> PropertyBuilder<'a> {
    fn build(self) -> Result<Property<'a>, ErrorKind> {
        let name = self.name.ok_or(ErrorKind::MissingPropertyName)?;
        let ty = self.ty.ok_or(ErrorKind::MissingPropertyType)?;
        let access = self.access.ok_or(ErrorKind::MissingPropertyAccess)?;

        Ok(Property {
            name,
            ty,
            access,
            annotations: self.annotations.into(),
            doc: self.doc,
        })
    }
}

#[derive(Debug, Default)]
struct AnnotationBuilder<'a> {
    name: Option<&'a str>,
    value: Option<&'a str>,
}

impl<'a> AnnotationBuilder<'a> {
    fn build(self) -> Result<Annotation<'a>, ErrorKind> {
        let name = self.name.ok_or(ErrorKind::MissingAnnotationName)?;
        let value = self.value.ok_or(ErrorKind::MissingAnnotationValue)?;
        Ok(Annotation { name, value })
    }
}

#[derive(Debug, Default)]
struct ArgumentBuilder<'a> {
    name: Option<&'a str>,
    ty: Option<&'a Signature>,
    direction: Option<Direction>,
    doc: Doc<'a>,
}

impl<'a> ArgumentBuilder<'a> {
    fn build(self, default: Direction) -> Result<Argument<'a>, ErrorKind> {
        let ty = self.ty.ok_or(ErrorKind::MissingArgumentType)?;

        Ok(Argument {
            name: self.name,
            ty,
            direction: self.direction.unwrap_or(default),
            doc: self.doc,
        })
    }
}

#[derive(Debug, Default)]
struct StringBuilder<'a> {
    text: Option<&'a str>,
}

#[derive(Debug)]
enum State<'a> {
    Node(NodeBuilder<'a>),
    Interface(InterfaceBuilder<'a>),
    Method(MethodBuilder<'a>),
    Signal(SignalBuilder<'a>),
    Property(PropertyBuilder<'a>),
    Argument(ArgumentBuilder<'a>),
    Annotation(AnnotationBuilder<'a>),
    Doc(Doc<'a>),
    Description(Description<'a>),
    String(&'static str, StringBuilder<'a>),
}
