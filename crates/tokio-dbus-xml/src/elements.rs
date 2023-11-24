/// A D-Bus node.
pub struct Node<'a> {
    /// Interfaces in the node.
    pub interfaces: Box<[Interface<'a>]>,
}

/// A single interface.
pub struct Interface<'a> {
    /// The name of the interface.
    pub name: &'a str,
    /// Methods associated with the interface.
    pub methods: Box<[Method<'a>]>,
}

/// The direction of an argument.
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    /// Input argument.
    In,
    /// Output argument.
    Out,
}

/// A method argument.
#[derive(Debug, Clone, Copy)]
pub struct Argument<'a> {
    /// The name of the argument.
    pub name: Option<&'a str>,
    /// The type of the argument.
    pub ty: &'a str,
    /// The direction of an argument.
    pub direction: Direction,
}

/// A single interface.
#[derive(Debug, Clone)]
pub struct Method<'a> {
    /// The name of the interface.
    pub name: &'a str,
    /// Arguments to the method.
    pub arguments: Box<[Argument<'a>]>,
}

/// Documentation associated with an element.
#[derive(Debug, Default)]
pub struct Doc<'a> {
    /// Documentation summary.
    pub summary: Option<&'a str>,
    /// Description.
    pub description: Description<'a>,
}

/// The description of an element.
#[derive(Debug, Default)]
pub struct Description<'a> {
    /// Paragraph describing an element.
    pub paragraph: Option<&'a str>,
}
