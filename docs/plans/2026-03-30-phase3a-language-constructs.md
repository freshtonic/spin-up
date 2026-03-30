# Phase 3a: Language Constructs — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use cipherpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend the lexer, AST, and parser to support `interface` definitions, `impl Interface for Type` blocks, `let` bindings with type instantiation and expressions, `#[default(...)]` attribute arguments, `<as Interface>` blocks, `self` keyword, and `it` constraint keyword.

**Architecture:** New keywords (`interface`, `impl`, `for`, `let`, `it`) added to the lexer. AST extended with `InterfaceDef`, `ImplBlock`, `LetBinding`, and an expression system (`Expr`). Parser extended with methods for each new construct. All changes in the `spin-lang` crate.

**Tech Stack:** Rust 2024 edition. All changes in `crates/spin-lang/`.

---

### Task 1: Lexer — New Keywords

**Files:**
- Modify: `crates/spin-lang/src/lexer.rs`
- Modify: `tests/lexer.rs`

**Step 1: Write the failing test**

Append to `tests/lexer.rs`:

```rust
#[test]
fn test_lex_phase3_keywords() {
    let tokens = lex("interface impl for let it").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        &[
            &Token::Interface,
            &Token::Impl,
            &Token::For,
            &Token::Let,
            &Token::It,
        ]
    );
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test lexer`
Expected: FAIL — new token variants don't exist

**Step 3: Add keyword variants**

Add to `Token` enum in `crates/spin-lang/src/lexer.rs`:

```rust
    Interface,
    Impl,
    For,
    Let,
    It,
```

Add keyword matching:

```rust
                    "interface" => Token::Interface,
                    "impl" => Token::Impl,
                    "for" => Token::For,
                    "let" => Token::Let,
                    "it" => Token::It,
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test lexer`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add crates/spin-lang/src/lexer.rs tests/lexer.rs
git commit -m "feat: add interface, impl, for, let, it keywords to lexer"
```

---

### Task 2: Lexer — Additional Operators

**Files:**
- Modify: `crates/spin-lang/src/lexer.rs`
- Modify: `tests/lexer.rs`

The expression system needs `&&`, `||`, and `!` operators for constraint predicates like `it >= 15 && it < 17`.

**Step 1: Write the failing test**

```rust
#[test]
fn test_lex_logical_operators() {
    let tokens = lex("&& || !").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(kinds, &[&Token::And, &Token::Or, &Token::Bang]);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test lexer`
Expected: FAIL

**Step 3: Add token variants and lexer rules**

Add to `Token` enum:

```rust
    And,    // &&
    Or,     // ||
    Bang,   // !
```

Add lexer rules for `&&` (two-char, before any single `&` handling), `||` (two-char, note `|` already produces `Token::Pipe`), and `!` (single char, but `!=` already produces `BangEq` — put `!` after the `!=` check as a fallback for standalone `!`).

**Step 4: Run tests to verify they pass**

Run: `cargo test --test lexer`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add crates/spin-lang/src/lexer.rs tests/lexer.rs
git commit -m "feat: add logical operators &&, ||, ! to lexer"
```

---

### Task 3: AST — Attribute Arguments

**Files:**
- Modify: `crates/spin-lang/src/ast.rs`
- Modify: `crates/spin-lang/src/ast_normalize.rs`

Currently `Attribute` only has a `name`. For `#[default(...)]` and `#[delegate(Foo)]` and `#[target(Foo)]`, attributes need arguments.

**Step 1: Extend Attribute**

Update `Attribute` in `crates/spin-lang/src/ast.rs`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub args: Option<Vec<Expr>>,
    pub span: Range<usize>,
}
```

Update `NormalizedAttribute` in `crates/spin-lang/src/ast_normalize.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedAttribute {
    pub name: String,
    pub args: Option<Vec<NormalizedExpr>>,
}
```

This requires defining `Expr` and `NormalizedExpr` first — see Task 4. For now, use `args: Option<String>` as a raw string capture of the argument content, to be refined in Task 4.

Actually — to keep things simpler, change `Attribute.args` to:

```rust
pub struct Attribute {
    pub name: String,
    pub args: Option<String>,  // Raw argument text, e.g. "SocketAddr::V4(port: 5432)"
    pub span: Range<usize>,
}
```

This captures the argument as raw text for now. Parsing the argument content as an expression will happen in a later task once the expression AST is defined.

**Step 2: Update parser to parse attribute arguments**

In `parse_attributes` in `crates/spin-lang/src/parser.rs`, after consuming `#[name`, if the next token is `LParen`, consume balanced parens and capture the content as raw text. Otherwise `args` is `None`.

**Step 3: Update NormalizedAttribute**

```rust
pub struct NormalizedAttribute {
    pub name: String,
    pub args: Option<String>,
}
```

Update normalization to propagate args.

**Step 4: Update tests**

Update tests that construct `Attribute` to include `args: None`. Add new test:

```rust
#[test]
fn test_parse_attribute_with_args() {
    let input = r#"#[default("postgres")]
type Foo = name: str;"#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            // The default attribute is on the field, not the type.
            // Actually, in our current syntax, attributes are on items, not fields.
            // We need field-level attributes first. See Task 5.
        }
        _ => panic!("expected RecordDef"),
    }
}
```

Wait — field-level attributes aren't supported yet. The `#[default(...)]` goes on interface fields. Let's first add attribute arg parsing for item-level attributes, then handle field-level attributes in a separate task.

Test for item-level attribute with args:

```rust
#[test]
fn test_parse_attribute_with_args() {
    let input = "#[delegate(PostgresEndpoint)]\ntype Proxy = frontend: str;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "delegate");
            assert_eq!(r.attributes[0].args.as_deref(), Some("PostgresEndpoint"));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}
```

**Step 5: Verify and commit**

Run: `cargo test --workspace`
Expected: All tests PASS

```bash
git commit -m "feat: add attribute argument parsing"
```

---

### Task 4: AST — Expression Types

**Files:**
- Modify: `crates/spin-lang/src/ast.rs`

Define the expression AST needed for `let` bindings, `impl` bodies, and constraint predicates.

**Step 1: Add Expr enum to ast.rs**

```rust
/// An expression
#[derive(Debug, Clone)]
pub enum Expr {
    /// String literal: `"hello"`
    StringLit(String),
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
    /// `<as Interface> { field: value, ... }` block within a construction
    AsInterface {
        interface_name: String,
        fields: Vec<FieldInit>,
    },
}

/// A field initializer: `name: expr`
#[derive(Debug, Clone)]
pub struct FieldInit {
    pub name: String,
    pub value: Expr,
    pub span: Range<usize>,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Eq,      // ==
    NotEq,   // !=
    Lt,      // <
    Gt,      // >
    Lte,     // <=
    Gte,     // >=
    And,     // &&
    Or,      // ||
}

/// Unary operators
#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,     // !
}
```

**Step 2: Verify it compiles**

Run: `cargo build --workspace`
Expected: Compiles (no references to Expr yet from parser)

**Step 3: Commit**

```bash
git add crates/spin-lang/src/ast.rs
git commit -m "feat: add expression AST types"
```

---

### Task 5: AST — Interface, Impl, Let, and Field Attributes

**Files:**
- Modify: `crates/spin-lang/src/ast.rs`

**Step 1: Add new top-level items**

Add to `Item` enum:

```rust
pub enum Item {
    RecordDef(RecordDef),
    ChoiceDef(ChoiceDef),
    InterfaceDef(InterfaceDef),
    ImplBlock(ImplBlock),
    LetBinding(LetBinding),
}
```

Define new structs:

```rust
/// An interface definition: `interface PostgresEndpoint = ...;`
#[derive(Debug, Clone)]
pub struct InterfaceDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub fields: Vec<InterfaceField>,
    pub span: Range<usize>,
}

/// A field in an interface definition, which can have attributes like #[default(...)]
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
```

**Step 2: Verify it compiles**

Run: `cargo build --workspace`
Expected: Compiles (might need to handle new Item variants in ast_normalize — add panics for now)

**Step 3: Commit**

```bash
git add crates/spin-lang/src/ast.rs crates/spin-lang/src/ast_normalize.rs
git commit -m "feat: add interface, impl, and let binding AST types"
```

---

### Task 6: Parser — Interface Definitions

**Files:**
- Modify: `crates/spin-lang/src/parser.rs`
- Modify: `tests/parser.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_parse_interface_def() {
    let input = "interface Endpoint = host: str, port: u16;";
    let module = parse(input).unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::InterfaceDef(i) => {
            assert_eq!(i.name, "Endpoint");
            assert_eq!(i.fields.len(), 2);
            assert_eq!(i.fields[0].name, "host");
            assert_eq!(i.fields[1].name, "port");
        }
        other => panic!("expected InterfaceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_interface_with_field_attributes() {
    let input = r#"interface Endpoint =
  #[default("localhost")]
  host: str,
  port: u16,
;"#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::InterfaceDef(i) => {
            assert_eq!(i.fields[0].attributes.len(), 1);
            assert_eq!(i.fields[0].attributes[0].name, "default");
            assert!(i.fields[0].attributes[0].args.is_some());
            assert_eq!(i.fields[1].attributes.len(), 0);
        }
        other => panic!("expected InterfaceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_interface_with_generic() {
    let input = "interface Container<T> = items: T;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::InterfaceDef(i) => {
            assert_eq!(i.type_params, vec!["T"]);
        }
        other => panic!("expected InterfaceDef, got {other:?}"),
    }
}
```

**Step 2: Run tests to verify they fail**

**Step 3: Implement interface parsing**

Add `Token::Interface` arm in `parse_module`. Implement `parse_interface_def`:
- Consume `interface`, expect ident, optionally parse `<T>` generic params, expect `=`
- Parse fields until `;`. Each field can have attributes (call `parse_attributes` before each field), then `name: TypeExpr`, separated by `,`.
- Fields use `InterfaceField` (which includes attributes) rather than `Field`.

**Step 4: Run tests, verify pass, commit**

```bash
git commit -m "feat: add interface definition parsing"
```

---

### Task 7: Parser — Expression Parsing

**Files:**
- Modify: `crates/spin-lang/src/parser.rs`
- Modify: `tests/parser.rs`

This is the core expression parser needed by `impl` blocks and `let` bindings. Implements a Pratt parser for operator precedence.

**Step 1: Write the failing tests**

```rust
#[test]
fn test_parse_expr_string_lit() {
    // We'll test expressions via let bindings once available.
    // For now, test the expression parser directly via a helper.
}
```

Actually — expressions are only reachable through `let` bindings and `impl` blocks. We should implement the expression parser as an internal method and test it through those constructs. Skip standalone expression tests; test via Task 8 and Task 9.

**Step 2: Implement `parse_expr` method**

Add `parse_expr` to `Parser`. It should handle (in order of precedence, lowest to highest):

1. `||` (logical or)
2. `&&` (logical and)
3. `==`, `!=` (equality)
4. `<`, `>`, `<=`, `>=` (comparison)
5. Unary `!`
6. Primary: literals (`"string"`, `42`, `true`, `false`), `it`, `self`, `None`, identifiers, type/variant construction, field access (`.`), `<as Interface> { ... }`

Use a simple Pratt parser or recursive descent with precedence levels.

For type construction: if we see `Ident LBrace` or `Ident LParen`, parse as construction. For field access: after a primary, if we see `.`, consume and parse ident as field name, chain for `self.endpoint.user`.

For `<as Interface> { ... }`: if we see `Lt` at the start of a position where an expression is expected, and the pattern is `< as Ident > { ... }`, parse as `AsInterface`.

**Step 3: Commit**

```bash
git commit -m "feat: add expression parser with operator precedence"
```

---

### Task 8: Parser — Impl Blocks

**Files:**
- Modify: `crates/spin-lang/src/parser.rs`
- Modify: `tests/parser.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_parse_impl_block() {
    let input = "impl Endpoint for MyServer {\n  host: self.hostname,\n  port: self.config.port,\n}";
    let module = parse(input).unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::ImplBlock(i) => {
            assert_eq!(i.interface_name, "Endpoint");
            assert_eq!(i.type_name, "MyServer");
            assert_eq!(i.mappings.len(), 2);
            assert_eq!(i.mappings[0].name, "host");
            assert_eq!(i.mappings[1].name, "port");
        }
        other => panic!("expected ImplBlock, got {other:?}"),
    }
}
```

**Step 2: Run tests to verify they fail**

**Step 3: Implement impl block parsing**

Add `Token::Impl` arm in `parse_module`. Implement `parse_impl_block`:
- Consume `impl`, expect ident (interface name), expect `for` (Token::For), expect ident (type name), expect `{`
- Parse field mappings until `}`: each is `name: expr` separated by `,` (trailing comma allowed)
- Returns `ImplBlock`

The `expr` in each mapping is parsed by `parse_expr` from Task 7. Expressions like `self.hostname` or `self.config.port` are `FieldAccess` chains.

**Step 4: Run tests, verify pass, commit**

```bash
git commit -m "feat: add impl block parsing"
```

---

### Task 9: Parser — Let Bindings

**Files:**
- Modify: `crates/spin-lang/src/parser.rs`
- Modify: `tests/parser.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_parse_let_binding_simple() {
    let input = r#"let name = "hello""#;
    let module = parse(input).unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::LetBinding(l) => {
            assert_eq!(l.name, "name");
        }
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

#[test]
fn test_parse_let_binding_with_type_annotation() {
    let input = "let port: u16 = 5432";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::LetBinding(l) => {
            assert_eq!(l.name, "port");
            assert!(l.ty.is_some());
        }
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

#[test]
fn test_parse_let_binding_with_construction() {
    let input = "let server = MyServer {\n  host: \"localhost\",\n  port: 8080,\n}";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::LetBinding(l) => {
            assert_eq!(l.name, "server");
        }
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

#[test]
fn test_parse_let_with_constraint() {
    let input = "let version = SemVer(major: it >= 15)";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::LetBinding(l) => {
            assert_eq!(l.name, "version");
        }
        other => panic!("expected LetBinding, got {other:?}"),
    }
}
```

**Step 2: Run tests to verify they fail**

**Step 3: Implement let binding parsing**

Add `Token::Let` arm in `parse_module`. Implement `parse_let_binding`:
- Consume `let`, expect ident (name)
- Optionally: if next is `:`, consume and parse type expression (type annotation)
- Expect `=`
- Parse expression (the value)
- Returns `LetBinding`

Note: `let` bindings do NOT end with `;` — they end when the expression is complete (no trailing semicolon needed since expressions are self-terminating: string/number literals end naturally, type constructions end at `}`).

**Step 4: Run tests, verify pass, commit**

```bash
git commit -m "feat: add let binding parsing"
```

---

### Task 10: Parser — As-Interface Blocks in Expressions

**Files:**
- Modify: `crates/spin-lang/src/parser.rs`
- Modify: `tests/parser.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_parse_as_interface_in_construction() {
    let input = r#"let x = MyType {
  name: "foo",
  <as Endpoint> {
    port: 8080,
  }
}"#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::LetBinding(l) => {
            // The value should be a TypeConstruction with an AsInterface embedded
            match &l.value {
                Expr::TypeConstruction { type_name, fields } => {
                    assert_eq!(type_name, "MyType");
                    // One of the fields or a separate AsInterface entry should exist
                }
                other => panic!("expected TypeConstruction, got {other:?}"),
            }
        }
        other => panic!("expected LetBinding, got {other:?}"),
    }
}
```

**Step 2: Implement `<as Interface>` parsing**

Within `parse_type_construction` (inside the `{ ... }` body), check for `<` followed by `as`. If found:
- Consume `<`, `as`, expect ident (interface name), expect `>`
- Expect `{`, parse field inits until `}`, expect `}`
- Add as an `AsInterface` entry

The `TypeConstruction` struct should include `as_interfaces: Vec<AsInterfaceBlock>` or the `Expr::AsInterface` variant can be mixed into the fields. Simplest: add a new field to `TypeConstruction`:

```rust
TypeConstruction {
    type_name: String,
    fields: Vec<FieldInit>,
    as_interfaces: Vec<AsInterfaceBlock>,
}
```

With:

```rust
pub struct AsInterfaceBlock {
    pub interface_name: String,
    pub fields: Vec<FieldInit>,
    pub span: Range<usize>,
}
```

**Step 3: Run tests, verify pass, commit**

```bash
git commit -m "feat: add <as Interface> block parsing in type constructions"
```

---

## What's Next

**Phase 3b: Static Analysis Engine** — to be planned after Phase 3a is complete. Covers:
- Module resolution (checking imports, building module graph)
- Type unification engine (structural matching, error collection)
- Constraint checking (predicate evaluation)
- Dependency graph construction (DAG verification)
- Error reporting with miette diagnostics
