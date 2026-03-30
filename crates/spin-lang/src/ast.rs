use std::ops::Range;

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
    pub fields: Vec<TypeExpr>,
    pub span: Range<usize>,
}

/// A field in a type definition: `port: spin-core::TcpPort`
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeExpr,
    pub attributes: Vec<Attribute>,
    pub span: Range<usize>,
}

/// A type expression
#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// A simple named type, e.g. `MyType`
    Named(String),
    /// A primitive type, e.g. `u32`, `bool`, `str`
    Primitive(PrimitiveType),
    /// A qualified path, e.g. `spin-core::TcpPort`
    Path { module: String, name: String },
    /// A generic type, e.g. `Option<u32>`
    Generic { name: String, args: Vec<TypeExpr> },
    /// Self-qualified type, e.g. `Self::Tls`
    SelfPath(String),
    /// Fixed-size array, e.g. `[u8; 4]`
    Array { element: Box<TypeExpr>, size: usize },
    /// Slice (size unknown), e.g. `[u8]`
    Slice(Box<TypeExpr>),
    /// Tuple, e.g. `(u32, str)`
    Tuple(Vec<TypeExpr>),
    /// Unit type `()`
    Unit,
}

/// A segment of an interpolated string
#[derive(Debug, Clone)]
pub enum StringPart {
    /// Literal text segment
    Literal(String),
    /// Interpolated expression: `${expr}`
    Expr(Expr),
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
    FieldAccess { object: Box<Expr>, field: String },
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
        args: Vec<Expr>,
    },
    /// Function/variant call with named args: `SemVer(major: 17)`
    NamedConstruction {
        type_name: String,
        fields: Vec<FieldInit>,
    },
    /// Function call with positional args: `Some(42)`, `my_func(a, b)`
    Call { name: String, args: Vec<Expr> },
    /// Binary operation: `it >= 15`, `it < 17`, `a && b`
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    /// Unary operation: `!x`
    UnaryOp { op: UnaryOp, operand: Box<Expr> },
    /// The `it` keyword (value being constrained)
    It,
    /// The `self` keyword
    Self_,
    /// `None` literal (sugar for `Option::None`)
    None_,
}

/// A field initializer: `name: expr`
#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub value: Expr,
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
    pub ty: TypeExpr,
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
    pub value: Expr,
    pub span: Range<usize>,
}

/// A let binding: `let proxy = Proxy { ... }`
#[derive(Debug, Clone)]
pub struct LetBinding {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Range<usize>,
}

/// Primitive types built into the language
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Str,
}
