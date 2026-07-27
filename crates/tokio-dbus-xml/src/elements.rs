use tokio_dbus_core::signature::Signature;

/// A D-Bus node.
#[derive(Debug, Clone, Default)]
pub struct Node<'a> {
    /// The name of the node, which for a root node is usually absent and for a
    /// child node is a relative object path.
    pub name: Option<&'a str>,
    /// Interfaces in the node.
    pub interfaces: Box<[Interface<'a>]>,
    /// Sub-nodes in the node.
    pub nodes: Box<[Node<'a>]>,
}

impl<'a> Node<'a> {
    /// Find an interface by name.
    ///
    /// This searches the node and, recursively, all of its children, which makes
    /// it possible to pick a single interface out of the introspection data of a
    /// whole object tree.
    pub fn interface(&self, name: &str) -> Option<&Interface<'a>> {
        if let Some(interface) = self.interfaces.iter().find(|i| i.name == name) {
            return Some(interface);
        }

        self.nodes.iter().find_map(|n| n.interface(name))
    }

    /// Iterate over every interface in this node and its children.
    pub fn all_interfaces(&self) -> impl Iterator<Item = &Interface<'a>> {
        let mut stack = vec![(self, 0usize)];

        core::iter::from_fn(move || {
            loop {
                let (node, index) = stack.last_mut()?;

                if let Some(interface) = node.interfaces.get(*index) {
                    *index += 1;
                    return Some(interface);
                }

                let node = *node;
                stack.pop();
                stack.extend(node.nodes.iter().map(|n| (n, 0)));
            }
        })
    }
}

/// A single interface.
#[derive(Debug, Clone)]
pub struct Interface<'a> {
    /// The name of the interface.
    pub name: &'a str,
    /// Methods associated with the interface.
    pub methods: Box<[Method<'a>]>,
    /// Signals associated with the interface.
    pub signals: Box<[Signal<'a>]>,
    /// Properties associated with the interface.
    pub properties: Box<[Property<'a>]>,
    /// Annotations applied to the interface.
    pub annotations: Box<[Annotation<'a>]>,
    /// Documentation associated with the interface.
    pub doc: Doc<'a>,
}

/// The direction of an argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Input argument.
    In,
    /// Output argument.
    Out,
}

/// A method or signal argument.
#[derive(Debug, Clone)]
pub struct Argument<'a> {
    /// The name of the argument.
    pub name: Option<&'a str>,
    /// The type of the argument.
    pub ty: &'a Signature,
    /// The direction of the argument.
    ///
    /// Arguments of a signal have no direction, and arguments of a method
    /// default to [`Direction::In`] when it is absent.
    pub direction: Direction,
    /// Documentation associated with the argument.
    pub doc: Doc<'a>,
}

/// A method on an interface.
#[derive(Debug, Clone)]
pub struct Method<'a> {
    /// The name of the method.
    pub name: &'a str,
    /// Arguments to the method, in both directions.
    pub arguments: Box<[Argument<'a>]>,
    /// Annotations applied to the method.
    pub annotations: Box<[Annotation<'a>]>,
    /// Documentation associated with the method.
    pub doc: Doc<'a>,
}

impl<'a> Method<'a> {
    /// Iterate over the arguments the caller passes in.
    pub fn inputs(&self) -> impl Iterator<Item = &Argument<'a>> {
        self.arguments
            .iter()
            .filter(|a| a.direction == Direction::In)
    }

    /// Iterate over the arguments the method returns.
    pub fn outputs(&self) -> impl Iterator<Item = &Argument<'a>> {
        self.arguments
            .iter()
            .filter(|a| a.direction == Direction::Out)
    }

    /// Test if the method is annotated as not returning a reply.
    pub fn no_reply(&self) -> bool {
        self.annotations
            .iter()
            .any(|a| a.name == "org.freedesktop.DBus.Method.NoReply" && a.value == "true")
    }
}

/// A signal emitted by an interface.
#[derive(Debug, Clone)]
pub struct Signal<'a> {
    /// The name of the signal.
    pub name: &'a str,
    /// The arguments carried by the signal.
    pub arguments: Box<[Argument<'a>]>,
    /// Annotations applied to the signal.
    pub annotations: Box<[Annotation<'a>]>,
    /// Documentation associated with the signal.
    pub doc: Doc<'a>,
}

/// Whether a property can be read, written, or both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// The property can only be read.
    Read,
    /// The property can only be written.
    Write,
    /// The property can be both read and written.
    ReadWrite,
}

impl Access {
    /// Test if the property can be read.
    pub fn is_readable(self) -> bool {
        matches!(self, Access::Read | Access::ReadWrite)
    }

    /// Test if the property can be written.
    pub fn is_writable(self) -> bool {
        matches!(self, Access::Write | Access::ReadWrite)
    }
}

/// A property on an interface.
#[derive(Debug, Clone)]
pub struct Property<'a> {
    /// The name of the property.
    pub name: &'a str,
    /// The type of the property.
    pub ty: &'a Signature,
    /// Whether the property can be read, written, or both.
    pub access: Access,
    /// Annotations applied to the property.
    pub annotations: Box<[Annotation<'a>]>,
    /// Documentation associated with the property.
    pub doc: Doc<'a>,
}

/// An annotation, which is a free form name and value pair attached to an
/// element.
#[derive(Debug, Clone, Copy)]
pub struct Annotation<'a> {
    /// The name of the annotation, such as `org.freedesktop.DBus.Deprecated`.
    pub name: &'a str,
    /// The value of the annotation.
    pub value: &'a str,
}

/// Documentation associated with an element.
#[derive(Debug, Default, Clone)]
pub struct Doc<'a> {
    /// Documentation summary.
    pub summary: Option<&'a str>,
    /// Description.
    pub description: Description<'a>,
}

impl<'a> Doc<'a> {
    /// Iterate over the lines of documentation, if there are any.
    ///
    /// Leading and trailing whitespace is trimmed from each line, since
    /// documentation in an interface file is indented to match the surrounding
    /// XML rather than the text.
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        self.summary
            .into_iter()
            .chain(self.description.paragraph)
            .flat_map(|text| text.lines())
            .map(str::trim)
            .filter(|line| !line.is_empty())
    }

    /// Test if there is no documentation at all.
    pub fn is_empty(&self) -> bool {
        self.summary.is_none() && self.description.paragraph.is_none()
    }
}

/// The description of an element.
#[derive(Debug, Default, Clone)]
pub struct Description<'a> {
    /// Paragraph describing an element.
    pub paragraph: Option<&'a str>,
}
