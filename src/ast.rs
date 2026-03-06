#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub nodes: Vec<Node>,
}

impl Document {
    pub fn new(nodes: Vec<Node>) -> Self {
        Self { nodes }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub element: ElementNode,
}

impl Node {
    pub fn new(element: ElementNode) -> Self {
        Self { element }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementNode {
    pub name: String,
    pub properties: Vec<Property>,
    pub children: Vec<Node>,
    pub span: Span,
}

impl ElementNode {
    pub fn new(
        name: impl Into<String>,
        properties: Vec<Property>,
        children: Vec<Node>,
        span: Span,
    ) -> Self {
        Self {
            name: name.into(),
            properties,
            children,
            span,
        }
    }

    pub fn line(&self) -> usize {
        self.span.line
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub name: String,
    pub value: Value,
    pub span: Span,
}

impl Property {
    pub fn new(name: impl Into<String>, value: Value, span: Span) -> Self {
        Self {
            name: name.into(),
            value,
            span,
        }
    }

    pub fn line(&self) -> usize {
        self.span.line
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    String(String),
    Number(String),
    Ident(String),
    Raw(String),
}

impl Value {
    pub fn as_str(&self) -> &str {
        match self {
            Value::String(value) => value,
            Value::Number(value) => value,
            Value::Ident(value) => value,
            Value::Raw(value) => value,
        }
    }

    pub fn into_string(self) -> String {
        match self {
            Value::String(value) => value,
            Value::Number(value) => value,
            Value::Ident(value) => value,
            Value::Raw(value) => value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}
