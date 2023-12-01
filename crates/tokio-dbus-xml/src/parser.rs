use std::fmt::Write;

use tokio_dbus_core::signature::Signature;
use xmlparser::{ElementEnd, Token};

use crate::error::ErrorKind;
use crate::{Argument, Description, Direction, Doc, Error, Interface, Method, Node, Result};

/// Parse the contents of an interface file.
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
                    (None, "node") => {
                        stack.push(State::Node(NodeBuilder::default()));
                    }
                    (Some(State::Node(..)), "interface") => {
                        stack.push(State::Interface(InterfaceBuilder::default()));
                    }
                    (Some(State::Interface(..)), "method") => {
                        stack.push(State::Method(MethodBuilder::default()));
                    }
                    (Some(State::Method(..)), "arg") => {
                        stack.push(State::Argument(ArgumentBuilder::default()));
                    }
                    (Some(State::Argument(..) | State::Method(..)), "doc") => {
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
                    [.., State::Method(method), State::Argument(..)] => {
                        let _ = write!(path, "[{}]", method.arguments.len());
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
                        root.interfaces.extend(node.interfaces);
                        root.nodes.extend(node.nodes);
                    }
                    ([.., State::Node(node)], State::Interface(builder)) => {
                        expect_end!(name, "interface");
                        node.interfaces.push(
                            builder
                                .build()
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    ([.., State::Node(node)], State::Node(builder)) => {
                        expect_end!(name, "interface");
                        node.nodes.push(builder.build());
                    }
                    ([.., State::Interface(interface)], State::Method(builder)) => {
                        expect_end!(name, "method");
                        interface.methods.push(
                            builder
                                .build()
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    ([.., State::Method(method)], State::Argument(builder)) => {
                        expect_end!(name, "arg");
                        method.arguments.push(
                            builder
                                .build()
                                .map_err(|kind| Error::new(path.as_str(), kind))?,
                        );
                    }
                    ([.., State::Argument(argument)], State::Doc(doc)) => {
                        expect_end!(name, "doc");
                        argument.doc = doc;
                    }
                    ([.., State::Method(method)], State::Doc(doc)) => {
                        expect_end!(name, "doc");
                        method.doc = doc;
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
                    ([.., State::Interface(builder)], _, "name") => {
                        builder.name = Some(value.as_str());
                    }
                    ([.., State::Method(builder)], _, "name") => {
                        builder.name = Some(value.as_str());
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
                                ))
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
    interfaces: Vec<Interface<'a>>,
    nodes: Vec<Node<'a>>,
}

impl<'a> NodeBuilder<'a> {
    fn build(self) -> Node<'a> {
        Node {
            interfaces: self.interfaces.into(),
            nodes: self.nodes.into(),
        }
    }
}

#[derive(Debug, Default)]
struct InterfaceBuilder<'a> {
    name: Option<&'a str>,
    methods: Vec<Method<'a>>,
}

impl<'a> InterfaceBuilder<'a> {
    fn build(self) -> Result<Interface<'a>, ErrorKind> {
        let name = self.name.ok_or(ErrorKind::MissingInterfaceName)?;
        Ok(Interface {
            name,
            methods: self.methods.into(),
        })
    }
}

#[derive(Debug, Default)]
struct MethodBuilder<'a> {
    name: Option<&'a str>,
    arguments: Vec<Argument<'a>>,
    doc: Doc<'a>,
}

impl<'a> MethodBuilder<'a> {
    fn build(self) -> Result<Method<'a>, ErrorKind> {
        let name = self.name.ok_or(ErrorKind::MissingMethodName)?;
        Ok(Method {
            name,
            arguments: self.arguments.into(),
        })
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
    fn build(self) -> Result<Argument<'a>, ErrorKind> {
        let ty = self.ty.ok_or(ErrorKind::MissingArgumentType)?;
        let direction = self.direction.ok_or(ErrorKind::MissingArgumentDirection)?;

        Ok(Argument {
            name: self.name,
            ty,
            direction,
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
    Argument(ArgumentBuilder<'a>),
    Doc(Doc<'a>),
    Description(Description<'a>),
    String(&'static str, StringBuilder<'a>),
}
