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

/// An attribute: `#[lang-item]`
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub span: Range<usize>,
}

/// A top-level item in a module
#[derive(Debug, Clone)]
pub enum Item {
    SuppliesDef(SuppliesDef),
    RecordDef(RecordDef),
    ChoiceDef(ChoiceDef),
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
    pub span: Range<usize>,
}

/// A supplies declaration: `supplies postgres::Postgres { ... }`
#[derive(Debug, Clone)]
pub struct SuppliesDef {
    pub resource_path: QualifiedPath,
    pub field_assignments: Vec<FieldAssignment>,
    pub span: Range<usize>,
}

/// A qualified path: `module::Name`
#[derive(Debug, Clone)]
pub struct QualifiedPath {
    pub module: String,
    pub name: String,
}

/// A field assignment: `host = "localhost"`
#[derive(Debug, Clone)]
pub struct FieldAssignment {
    pub name: String,
    pub value: Expr,
    pub span: Range<usize>,
}

/// An expression (values on the right-hand side of assignments)
#[derive(Debug, Clone)]
pub enum Expr {
    StringLit(String),
    Number(String),
    Bool(bool),
    Ident(String),
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
