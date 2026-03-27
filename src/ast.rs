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

/// A top-level item in a module
#[derive(Debug, Clone)]
pub enum Item {
    ResourceDef(ResourceDef),
}

/// A resource definition: `resource Postgres { ... }`
#[derive(Debug, Clone)]
pub struct ResourceDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Range<usize>,
}

/// A field in a resource definition: `port: spin-core::TcpPort`
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Range<usize>,
}

/// A type expression
#[derive(Debug, Clone)]
pub enum TypeExpr {
    /// A simple named type, e.g. `String`
    Named(String),
    /// A qualified path, e.g. `spin-core::TcpPort`
    Path { module: String, name: String },
    /// A generic type, e.g. `Option<spin-core::TcpPort>`
    Generic { name: String, args: Vec<TypeExpr> },
    /// Self-qualified type, e.g. `Self::Tls`
    SelfPath(String),
}
