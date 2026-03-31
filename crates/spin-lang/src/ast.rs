use std::ops::Range;

/// A wrapper that pairs an AST node with its source span.
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub kind: T,
    pub span: Range<usize>,
}

/// Type alias for a spanned expression.
pub type SpannedExpr = Spanned<Expr>;

/// Type alias for a spanned type expression.
pub type SpannedTypeExpr = Spanned<TypeExpr>;

impl<T> Spanned<T> {
    /// Create a new spanned node.
    pub fn new(kind: T, span: Range<usize>) -> Self {
        Self { kind, span }
    }
}

/// A complete .spin module
#[derive(Debug, Clone)]
pub struct Module {
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
}

/// An import statement: `import postgres`
#[derive(Debug, Clone)]
pub struct Import {
    pub module_name: String,
    pub span: Range<usize>,
}

/// An attribute: `#[lang-item]` or `#[delegate(PostgresEndpoint)]`
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub args: Option<String>,
    pub span: Range<usize>,
}

/// A top-level item in a module
#[derive(Debug, Clone)]
pub enum Item {
    RecordDef(RecordDef),
    ChoiceDef(ChoiceDef),
    InterfaceDef(InterfaceDef),
    ImplBlock(ImplBlock),
    LetBinding(LetBinding),
}

/// A record definition (product type): `type Tls = port: u16, key: str;`
#[derive(Debug, Clone)]
pub struct RecordDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub attributes: Vec<Attribute>,
    pub fields: Vec<Field>,
    pub span: Range<usize>,
}

/// A choice definition (sum type): `type IpAddr = V4(IpAddrV4) | V6(IpAddrV6);`
#[derive(Debug, Clone)]
pub struct ChoiceDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub attributes: Vec<Attribute>,
    pub variants: Vec<Variant>,
    pub span: Range<usize>,
}

/// A variant of a choice type
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<SpannedTypeExpr>,
    pub span: Range<usize>,
}

/// A field in a type definition: `port: spin-core::TcpPort`
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: SpannedTypeExpr,
    pub attributes: Vec<Attribute>,
    pub span: Range<usize>,
}

/// A type expression
#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// A simple named type, e.g. `MyType`
    Named(String),
    /// A primitive type, e.g. `number`, `bool`, `string`
    Primitive(PrimitiveType),
    /// A qualified path, e.g. `spin-core::TcpPort`
    Path { module: String, name: String },
    /// A generic type, e.g. `Option<number>`
    Generic {
        name: String,
        args: Vec<SpannedTypeExpr>,
    },
    /// Self-qualified type, e.g. `Self::Tls`
    SelfPath(String),
    /// List type: `[T]`
    List(Box<SpannedTypeExpr>),
    /// HashMap type: `{K: V}`
    HashMap {
        key: Box<SpannedTypeExpr>,
        value: Box<SpannedTypeExpr>,
    },
}

/// A segment of an interpolated string
#[derive(Debug, Clone)]
pub enum StringPart {
    /// Literal text segment
    Literal(String),
    /// Interpolated expression: `${expr}`
    Expr(SpannedExpr),
}

/// An expression
#[derive(Debug, Clone)]
pub enum Expr {
    /// String literal: `"hello"`
    StringLit(String),
    /// String with interpolated expressions: `"hello ${name}"`
    StringInterpolation(Vec<StringPart>),
    /// Numeric literal: `42`, `3.14`, `0xff`
    Number(String),
    /// Boolean literal: `true`, `false`
    BoolLit(bool),
    /// Identifier reference: `proxy`, `my_var`
    Ident(String),
    /// Field access: `self.port`, `self.endpoint.user`
    FieldAccess {
        object: Box<SpannedExpr>,
        field: String,
    },
    /// Type construction: `Proxy { field: value, ... }`
    TypeConstruction {
        type_name: String,
        fields: Vec<FieldInit>,
        as_interfaces: Vec<AsInterfaceBlock>,
    },
    /// Variant construction: `SocketAddr::V4(...)` or `Some(x)`
    VariantConstruction {
        type_name: String,
        variant: String,
        args: Vec<SpannedExpr>,
    },
    /// Function/variant call with named args: `SemVer(major: 17)`
    NamedConstruction {
        type_name: String,
        fields: Vec<FieldInit>,
    },
    /// Function call with positional args: `Some(42)`, `my_func(a, b)`
    Call {
        name: String,
        args: Vec<SpannedExpr>,
    },
    /// Binary operation: `it >= 15`, `it < 17`, `a && b`
    BinaryOp {
        left: Box<SpannedExpr>,
        op: BinaryOp,
        right: Box<SpannedExpr>,
    },
    /// Unary operation: `!x`
    UnaryOp {
        op: UnaryOp,
        operand: Box<SpannedExpr>,
    },
    /// The `it` keyword (value being constrained)
    It,
    /// The `self` keyword
    Self_,
    /// `None` literal (sugar for `Option::None`)
    None_,
    /// Regex literal: `r"pattern"`
    RegexLit(String),
    /// List literal: `#[1, 2, 3]`
    ListLit(Vec<SpannedExpr>),
    /// Set literal: `#("a", "b", "c")`
    SetLit(Vec<SpannedExpr>),
    /// HashMap literal: `#{"key": "value", ...}`
    HashMapLit(Vec<(SpannedExpr, SpannedExpr)>),
}

/// A field initializer: `name: expr`
#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub value: SpannedExpr,
    pub span: Range<usize>,
}

/// An `<as Interface> { field: value, ... }` block within a type construction
#[derive(Debug, Clone)]
pub struct AsInterfaceBlock {
    pub interface_name: String,
    pub fields: Vec<FieldInit>,
    pub span: Range<usize>,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    /// `==`
    Eq,
    /// `!=`
    NotEq,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Lte,
    /// `>=`
    Gte,
    /// `&&`
    And,
    /// `||`
    Or,
    /// `=~` regex match
    RegexMatch,
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    /// `!`
    Not,
}

/// An interface definition: `interface Endpoint = host: str, port: u16;`
#[derive(Debug, Clone)]
pub struct InterfaceDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub fields: Vec<InterfaceField>,
    pub span: Range<usize>,
}

/// A field in an interface definition, which can have attributes like `#[default(...)]`
#[derive(Debug, Clone)]
pub struct InterfaceField {
    pub name: String,
    pub ty: SpannedTypeExpr,
    pub attributes: Vec<Attribute>,
    pub span: Range<usize>,
}

/// An impl block: `impl Interface for Type { field: expr, ... }`
#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub interface_name: String,
    pub type_name: String,
    pub mappings: Vec<FieldMapping>,
    pub span: Range<usize>,
}

/// A field mapping in an impl block: `listen_on: self.listen_on`
#[derive(Debug, Clone)]
pub struct FieldMapping {
    pub name: String,
    pub value: SpannedExpr,
    pub span: Range<usize>,
}

/// A let binding: `let proxy = Proxy { ... }`
#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub ty: Option<SpannedTypeExpr>,
    pub value: SpannedExpr,
    pub span: Range<usize>,
}

/// Primitive types built into the language
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    Number,
    String,
}
