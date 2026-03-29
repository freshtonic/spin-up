# Phase 2: Spin-Core Primitives — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use cipherpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend the lexer, AST, and parser to support primitive types, records, choices, and attributes. Create the `spin-core-net.spin` embedded module, a proc-macro crate for compile-time verification, and corresponding Rust types.

**Architecture:** Primitive type names become lexer keywords. Records (product types) and choices (sum types) are new top-level items. Attributes (`#[lang-item]`) attach to items. The `spin-core-macros` proc-macro crate provides `#[spin_core]` which generates a spin AST from a Rust type and compares it to the embedded `.spin` source at compile time.

**Tech Stack:** Rust 2024 edition, `syn`/`quote`/`proc-macro2` (proc-macro crate). Workspace layout with `spin-core-macros` as a member.

---

### Task 1: Lexer — Primitive Type Keywords and Record/Choice

**Files:**
- Modify: `src/lexer.rs`
- Modify: `tests/lexer.rs`

**Step 1: Write the failing tests**

Append to `tests/lexer.rs`:

```rust
#[test]
fn test_lex_primitive_type_keywords() {
    let input = "bool u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64 str";
    let tokens = lex(input).unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        &[
            &Token::Bool,
            &Token::U8,
            &Token::U16,
            &Token::U32,
            &Token::U64,
            &Token::U128,
            &Token::I8,
            &Token::I16,
            &Token::I32,
            &Token::I64,
            &Token::I128,
            &Token::F32,
            &Token::F64,
            &Token::Str,
        ]
    );
}

#[test]
fn test_lex_type_keyword() {
    let tokens = lex("type").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(kinds, &[&Token::Type]);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test lexer`
Expected: FAIL — `Token::Bool`, `Token::Type`, etc. don't exist

**Step 3: Add keyword variants to Token enum**

Add to the `Token` enum in `src/lexer.rs`:

```rust
    // Primitive type keywords
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

    // Type definition keywords
    Type,
```

Add to the keyword match in the `lex` function:

```rust
                    "bool" => Token::Bool,
                    "u8" => Token::U8,
                    "u16" => Token::U16,
                    "u32" => Token::U32,
                    "u64" => Token::U64,
                    "u128" => Token::U128,
                    "i8" => Token::I8,
                    "i16" => Token::I16,
                    "i32" => Token::I32,
                    "i64" => Token::I64,
                    "i128" => Token::I128,
                    "f32" => Token::F32,
                    "f64" => Token::F64,
                    "str" => Token::Str,
                    "type" => Token::Type,
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test lexer`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/lexer.rs tests/lexer.rs
git commit -m "feat: add primitive type and type keyword to lexer"
```

---

### Task 2: Lexer — Rust-Style Numeric Literals

**Files:**
- Modify: `src/lexer.rs`
- Modify: `tests/lexer.rs`

**Step 1: Write the failing tests**

Append to `tests/lexer.rs`:

```rust
#[test]
fn test_lex_hex_literal() {
    let tokens = lex("0xff 0xFF 0x1A2B").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("0xff".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("0xFF".to_string()));
    assert_eq!(tokens[2].kind, Token::Number("0x1A2B".to_string()));
}

#[test]
fn test_lex_binary_literal() {
    let tokens = lex("0b1010 0b0000_1111").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("0b1010".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("0b0000_1111".to_string()));
}

#[test]
fn test_lex_octal_literal() {
    let tokens = lex("0o77 0o755").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("0o77".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("0o755".to_string()));
}

#[test]
fn test_lex_underscore_separators() {
    let tokens = lex("1_000_000 0xff_ff").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("1_000_000".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("0xff_ff".to_string()));
}

#[test]
fn test_lex_type_suffix() {
    let tokens = lex("42u32 3.14f64 0xffu8").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("42u32".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("3.14f64".to_string()));
    assert_eq!(tokens[2].kind, Token::Number("0xffu8".to_string()));
}

#[test]
fn test_lex_float_exponent() {
    let tokens = lex("1.0e10 2.5E-3 1e5").unwrap();
    assert_eq!(tokens[0].kind, Token::Number("1.0e10".to_string()));
    assert_eq!(tokens[1].kind, Token::Number("2.5E-3".to_string()));
    assert_eq!(tokens[2].kind, Token::Number("1e5".to_string()));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test lexer`
Expected: FAIL — current lexer doesn't handle hex/binary/octal/underscores/suffixes/exponents

**Step 3: Rewrite the number lexing logic**

Replace the `// Numbers` arm in `src/lexer.rs` with a comprehensive Rust-style number lexer. The number arm should handle:

1. Leading `0x`, `0b`, `0o` prefixes
2. Digits appropriate to the base (hex allows a-f/A-F)
3. Underscores as separators anywhere within the digit sequence
4. A single `.` for floats (only in decimal)
5. Exponent `e`/`E` optionally followed by `+`/`-` and digits (only in decimal)
6. Type suffix: consumed as trailing alphanumeric characters (e.g., `u32`, `f64`, `i8`)

The entire number including prefix, digits, underscores, decimal point, exponent, and suffix is captured as a single `Token::Number(String)`. Validation (e.g., that `0b` only has `0`/`1` digits) is left to a later semantic pass.

The implementation approach: after detecting a leading digit, peek for `0x`/`0b`/`0o`. Then consume all valid characters for that base. For decimal, also handle `.`, `e`/`E`, and `+`/`-` after exponent marker. Finally consume any trailing alphanumeric suffix.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test lexer`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/lexer.rs tests/lexer.rs
git commit -m "feat: add Rust-style numeric literal lexing"
```

---

### Task 3: Lexer — Attribute Syntax

**Files:**
- Modify: `src/lexer.rs`
- Modify: `tests/lexer.rs`

**Step 1: Write the failing tests**

Append to `tests/lexer.rs`:

```rust
#[test]
fn test_lex_attribute() {
    let tokens = lex("#[lang-item]").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(kinds, &[&Token::HashBracket, &Token::Ident("lang-item".to_string()), &Token::RBracket]);
}

#[test]
fn test_lex_attribute_before_choice() {
    let tokens = lex("#[lang-item]\ntype IpAddr {}").unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert_eq!(
        kinds,
        &[
            &Token::HashBracket,
            &Token::Ident("lang-item".to_string()),
            &Token::RBracket,
            &Token::Choice,
            &Token::Ident("IpAddr".to_string()),
            &Token::LBrace,
            &Token::RBrace,
        ]
    );
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test lexer`
Expected: FAIL — `Token::HashBracket` doesn't exist, `#` is an unexpected character

**Step 3: Add HashBracket token and lexer rule**

Add `HashBracket` variant to `Token` enum:

```rust
    HashBracket, // #[
```

Add a rule in the lexer for `#` followed by `[`:

```rust
            '#' if matches!(chars.clone().nth(1), Some((_, '['))) => {
                chars.next();
                chars.next();
                tokens.push(Spanned {
                    kind: Token::HashBracket,
                    span: pos..pos + 2,
                });
            }
```

This must be placed before the `_` catch-all in the match.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test lexer`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/lexer.rs tests/lexer.rs
git commit -m "feat: add attribute syntax #[...] to lexer"
```

---

### Task 4: AST — Record, Choice, Attribute, and Extended Type Expressions

**Files:**
- Modify: `src/ast.rs`

**Step 1: Add new AST types**

Add `Attribute` struct:

```rust
/// An attribute: `#[lang-item]`
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub span: Range<usize>,
}
```

Add `RecordDef` and `ChoiceDef` to `Item` enum and define their structs:

```rust
/// A top-level item in a module
#[derive(Debug, Clone)]
pub enum Item {
    ResourceDef(ResourceDef),
    SuppliesDef(SuppliesDef),
    RecordDef(RecordDef),
    ChoiceDef(ChoiceDef),
}

/// A record definition (product type): `type Tls { port: u16, key: str }`
#[derive(Debug, Clone)]
pub struct RecordDef {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub fields: Vec<Field>,
    pub span: Range<usize>,
}

/// A choice definition (sum type): `type IpAddr { V4(IpAddrV4), V6(IpAddrV6) }`
#[derive(Debug, Clone)]
pub struct ChoiceDef {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub variants: Vec<Variant>,
    pub span: Range<usize>,
}

/// A variant of a sum type
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<TypeExpr>,
    pub span: Range<usize>,
}
```

Add attributes to `ResourceDef`:

```rust
pub struct ResourceDef {
    pub name: String,
    pub attributes: Vec<Attribute>,
    pub fields: Vec<Field>,
    pub span: Range<usize>,
}
```

Extend `TypeExpr` with primitive, array, slice, tuple, and unit variants:

```rust
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
#[derive(Debug, Clone, PartialEq)]
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
```

**Step 2: Fix compilation errors**

The parser references `Item::ResourceDef` and `Item::SuppliesDef` in match arms — adding new variants to `Item` will cause exhaustiveness warnings or errors. The existing parser tests in `tests/parser.rs` already use wildcard arms for `Item`, so they should still compile. Verify:

Run: `cargo build`
Expected: Compiles (parser match arms for `Item` in `parse_module` only construct specific variants, and test matches use wildcard `other => panic!(...)`)

The `ResourceDef` struct gains a new `attributes` field — update the parser's `parse_resource_def` to pass `attributes: vec![]` for now.

**Step 3: Commit**

```bash
git add src/ast.rs src/parser.rs
git commit -m "feat: add type definition, attribute, and extended type AST nodes"
```

---

### Task 5: Parser — Attributes

**Files:**
- Modify: `src/parser.rs`
- Modify: `tests/parser.rs`

**Step 1: Write the failing tests**

Append to `tests/parser.rs`:

```rust
#[test]
fn test_parse_attribute_on_resource() {
    let input = r#"#[lang-item]
resource Postgres {
  port: u32,
}"#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ResourceDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "lang-item");
        }
        other => panic!("expected ResourceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_multiple_attributes() {
    let input = r#"#[lang-item]
#[deprecated]
resource Postgres {}"#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ResourceDef(r) => {
            assert_eq!(r.attributes.len(), 2);
            assert_eq!(r.attributes[0].name, "lang-item");
            assert_eq!(r.attributes[1].name, "deprecated");
        }
        other => panic!("expected ResourceDef, got {other:?}"),
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test parser`
Expected: FAIL — parser doesn't know how to handle `#[`

**Step 3: Implement attribute parsing**

Add a `parse_attributes` method to `Parser` that collects zero or more `#[name]` sequences. Call it at the start of `parse_module`'s loop — if the next token is `HashBracket`, collect attributes, then dispatch to the item parser, passing attributes along.

Update `parse_resource_def` to accept `attributes: Vec<Attribute>` parameter instead of constructing an empty vec.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test parser`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/parser.rs tests/parser.rs
git commit -m "feat: add attribute parsing for item definitions"
```

---

### Task 6: Parser — Record Definitions

**Files:**
- Modify: `src/parser.rs`
- Modify: `tests/parser.rs`

**Step 1: Write the failing tests**

Append to `tests/parser.rs`:

```rust
#[test]
fn test_parse_record_def() {
    let input = "type Tls {\n  port: u16,\n  key: str,\n}";
    let module = parse(input).unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.name, "Tls");
            assert_eq!(r.fields.len(), 2);
            assert_eq!(r.fields[0].name, "port");
            assert_eq!(r.fields[1].name, "key");
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_record_with_attribute() {
    let input = "#[lang-item]\ntype Tls {\n  port: u16,\n}";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "lang-item");
            assert_eq!(r.fields.len(), 1);
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_empty_record() {
    let module = parse("type Empty {}").unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.name, "Empty");
            assert!(r.fields.is_empty());
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test parser`
Expected: FAIL — parser doesn't handle `Token::Type`

**Step 3: Implement type definition parsing**

In `parse_module`, add a `Token::Type` arm that dispatches to `parse_type_def(attributes)`. The parser peeks at the body after `{` to determine whether it is a product type (record) or sum type (choice). If the first token is `Ident` followed by `Colon`, it is a product type; otherwise a sum type.

The `parse_field` method already exists and handles `name: TypeExpr` — reuse it directly.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test parser`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/parser.rs tests/parser.rs
git commit -m "feat: add type definition parsing (product types)"
```

---

### Task 7: Parser — Choice Definitions

**Files:**
- Modify: `src/parser.rs`
- Modify: `tests/parser.rs`

**Step 1: Write the failing tests**

Append to `tests/parser.rs`:

```rust
#[test]
fn test_parse_choice_def() {
    let input = "type IpAddr {\n  V4(IpAddrV4),\n  V6(IpAddrV6),\n}";
    let module = parse(input).unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::ChoiceDef(c) => {
            assert_eq!(c.name, "IpAddr");
            assert_eq!(c.variants.len(), 2);
            assert_eq!(c.variants[0].name, "V4");
            assert_eq!(c.variants[0].fields.len(), 1);
            assert_eq!(c.variants[1].name, "V6");
            assert_eq!(c.variants[1].fields.len(), 1);
        }
        other => panic!("expected ChoiceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_choice_unit_variant() {
    let input = "type Option {\n  Some(T),\n  None,\n}";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ChoiceDef(c) => {
            assert_eq!(c.variants.len(), 2);
            assert_eq!(c.variants[0].name, "Some");
            assert_eq!(c.variants[0].fields.len(), 1);
            assert_eq!(c.variants[1].name, "None");
            assert!(c.variants[1].fields.is_empty());
        }
        other => panic!("expected ChoiceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_choice_with_attribute() {
    let input = "#[lang-item]\ntype IpAddr {\n  V4(IpAddrV4),\n  V6(IpAddrV6),\n}";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ChoiceDef(c) => {
            assert_eq!(c.attributes.len(), 1);
            assert_eq!(c.attributes[0].name, "lang-item");
        }
        other => panic!("expected ChoiceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_choice_multi_field_variant() {
    let input = "type Pair {\n  Both(u32, str),\n  Neither,\n}";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ChoiceDef(c) => {
            assert_eq!(c.variants[0].name, "Both");
            assert_eq!(c.variants[0].fields.len(), 2);
        }
        other => panic!("expected ChoiceDef, got {other:?}"),
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test parser`
Expected: FAIL — parser doesn't handle sum type variant of `Token::Type`

**Step 3: Implement sum type parsing**

The `parse_type_def` method already handles both product and sum types via body peeking. When the body contains variant syntax (identifiers followed by `(` rather than `:`), it parses as a sum type (ChoiceDef).

Add `parse_variant` method:

```rust
fn parse_variant(&mut self) -> Result<Variant, ParseError> {
    let (name, name_span) = self.expect_ident()?;
    let mut fields = Vec::new();

    if self.check(&Token::LParen) {
        self.advance();
        loop {
            if self.check(&Token::RParen) {
                break;
            }
            fields.push(self.parse_type_expr()?);
            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.expect_token(Token::RParen)?;
    }

    let end = self.previous_span_end();
    Ok(Variant {
        name,
        fields,
        span: name_span.start..end,
    })
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test --test parser`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/parser.rs tests/parser.rs
git commit -m "feat: add sum type definition parsing with variants"
```

---

### Task 8: Parser — Primitive and Compound Type Expressions

**Files:**
- Modify: `src/parser.rs`
- Modify: `tests/parser.rs`

**Step 1: Write the failing tests**

Append to `tests/parser.rs`:

```rust
use spin_up::ast::PrimitiveType;

#[test]
fn test_parse_primitive_type_in_field() {
    let input = "type Foo { x: u32, y: bool, z: str }";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert!(matches!(&r.fields[0].ty, TypeExpr::Primitive(PrimitiveType::U32)));
            assert!(matches!(&r.fields[1].ty, TypeExpr::Primitive(PrimitiveType::Bool)));
            assert!(matches!(&r.fields[2].ty, TypeExpr::Primitive(PrimitiveType::Str)));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_array_type() {
    let input = "type Foo { data: [u8; 4] }";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            match &r.fields[0].ty {
                TypeExpr::Array { element, size } => {
                    assert!(matches!(element.as_ref(), TypeExpr::Primitive(PrimitiveType::U8)));
                    assert_eq!(*size, 4);
                }
                other => panic!("expected Array, got {other:?}"),
            }
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_slice_type() {
    let input = "type Foo { data: [u8] }";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            match &r.fields[0].ty {
                TypeExpr::Slice(element) => {
                    assert!(matches!(element.as_ref(), TypeExpr::Primitive(PrimitiveType::U8)));
                }
                other => panic!("expected Slice, got {other:?}"),
            }
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_tuple_type() {
    let input = "type Foo { pair: (u32, str) }";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            match &r.fields[0].ty {
                TypeExpr::Tuple(elements) => {
                    assert_eq!(elements.len(), 2);
                    assert!(matches!(&elements[0], TypeExpr::Primitive(PrimitiveType::U32)));
                    assert!(matches!(&elements[1], TypeExpr::Primitive(PrimitiveType::Str)));
                }
                other => panic!("expected Tuple, got {other:?}"),
            }
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_unit_type() {
    let input = "type Foo { nothing: () }";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert!(matches!(&r.fields[0].ty, TypeExpr::Unit));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test parser`
Expected: FAIL — parser doesn't produce `TypeExpr::Primitive`, `TypeExpr::Array`, etc.

**Step 3: Extend `parse_type_expr`**

Update `parse_type_expr` in `src/parser.rs`:

1. **Primitive types:** Check if the next token is a primitive keyword (`Bool`, `U8`, ..., `Str`). If so, consume it and return `TypeExpr::Primitive(PrimitiveType::...)`.

2. **Array/Slice types:** If next token is `LBracket`, consume it, parse inner type expression. If followed by `;` then a number then `]` → `TypeExpr::Array`. If followed by `]` → `TypeExpr::Slice`. (Note: `;` is not yet a token — add `Semicolon` to the lexer and lex `;` as `Token::Semicolon`.)

3. **Tuple/Unit types:** If next token is `LParen`, consume it. If immediately followed by `RParen` → `TypeExpr::Unit`. Otherwise parse comma-separated type expressions until `RParen` → `TypeExpr::Tuple`.

Add `Semicolon` to `Token` enum and lex `;` in the lexer.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test parser`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/lexer.rs src/parser.rs tests/parser.rs
git commit -m "feat: add primitive, array, slice, tuple, and unit type parsing"
```

---

### Task 9: Embedded `.spin` Sources and Module Loader Integration

**Files:**
- Create: `spin-core-modules/spin-core-net.spin`
- Create: `src/builtins.rs`
- Modify: `src/lib.rs`
- Modify: `src/spin_path.rs`
- Create: `tests/builtins.rs`

**Step 1: Write the `.spin` source**

Create `spin-core-modules/spin-core-net.spin`:

```
#[lang-item]
type IpAddrV4 {
  octets: [u8; 4],
}

#[lang-item]
type IpAddrV6 {
  octets: [u8; 16],
}

#[lang-item]
type IpAddr {
  V4(IpAddrV4),
  V6(IpAddrV6),
}

#[lang-item]
type SocketAddrV4 {
  ip: IpAddrV4,
  port: u16,
}

#[lang-item]
type SocketAddrV6 {
  ip: IpAddrV6,
  port: u16,
}

#[lang-item]
type SocketAddr {
  V4(SocketAddrV4),
  V6(SocketAddrV6),
}
```

**Step 2: Write the failing tests**

Create `tests/builtins.rs`:

```rust
use spin_up::builtins;
use spin_up::parser;

#[test]
fn test_builtin_module_exists() {
    let source = builtins::get_module_source("spin-core-net");
    assert!(source.is_some());
}

#[test]
fn test_builtin_module_not_found() {
    let source = builtins::get_module_source("spin-core-nonexistent");
    assert!(source.is_none());
}

#[test]
fn test_builtin_spin_core_net_parses() {
    let source = builtins::get_module_source("spin-core-net").unwrap();
    let module = parser::parse(source).unwrap();
    // Should have 6 items: IpAddrV4, IpAddrV6, IpAddr, SocketAddrV4, SocketAddrV6, SocketAddr
    assert_eq!(module.items.len(), 6);
}

#[test]
fn test_builtin_module_names_list() {
    let names = builtins::builtin_module_names();
    assert!(names.contains(&"spin-core-net"));
}
```

**Step 3: Run tests to verify they fail**

Run: `cargo test --test builtins`
Expected: FAIL — `spin_up::builtins` doesn't exist

**Step 4: Implement builtins module**

Add to `src/lib.rs`:

```rust
pub mod builtins;
```

Create `src/builtins.rs`:

```rust
const SPIN_CORE_NET: &str = include_str!("../spin-core-modules/spin-core-net.spin");

pub fn get_module_source(name: &str) -> Option<&'static str> {
    match name {
        "spin-core-net" => Some(SPIN_CORE_NET),
        _ => None,
    }
}

pub fn builtin_module_names() -> &'static [&'static str] {
    &["spin-core-net"]
}
```

**Step 5: Update SpinPath to check builtins**

Update `resolve` in `src/spin_path.rs`: before searching `SPIN_PATH` directories, check if the module name starts with `spin-core` and if `builtins::get_module_source` returns `Some`. If so, return a new variant or a special result indicating it's a builtin. For now, add a new error or return type — or simply: change the prefix check so that `spin-core-*` names don't error with `ReservedPrefix` but instead return a new `SpinPathError::BuiltinModule(String)` indicating callers should use `builtins::get_module_source` instead.

Actually, a cleaner approach: add a method `resolve_source` to `SpinPath` that returns the source string (reading from disk or from builtins):

```rust
pub fn resolve_source(&self, module_name: &str) -> Result<String, SpinPathError> {
    // Check builtins first
    if let Some(source) = crate::builtins::get_module_source(module_name) {
        return Ok(source.to_string());
    }

    // Then check SPIN_PATH on disk
    let path = self.resolve(module_name)?;
    std::fs::read_to_string(&path)
        .map_err(|e| SpinPathError::ReadError { path, source: e })
}
```

Add `ReadError` variant to `SpinPathError`:

```rust
    #[error("failed to read {path}: {source}")]
    ReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
```

**Step 6: Run tests to verify they pass**

Run: `cargo test`
Expected: All tests PASS

**Step 7: Commit**

```bash
git add spin-core-modules/ src/builtins.rs src/lib.rs src/spin_path.rs tests/builtins.rs
git commit -m "feat: embed spin-core-net.spin and integrate with module loader"
```

---

### Task 10: Workspace Setup — Proc-Macro Crate

**Files:**
- Modify: `Cargo.toml` (make it a workspace root)
- Create: `crates/spin-core-macros/Cargo.toml`
- Create: `crates/spin-core-macros/src/lib.rs`

**Step 1: Create workspace structure**

Update root `Cargo.toml` to add workspace:

```toml
[workspace]
members = [".", "crates/spin-core-macros"]
```

Create `crates/spin-core-macros/Cargo.toml`:

```toml
[package]
name = "spin-core-macros"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true

[dependencies]
syn = { version = "2", features = ["full"] }
quote = "1"
proc-macro2 = "1"
spin-up = { path = "../.." }
```

Create `crates/spin-core-macros/src/lib.rs`:

```rust
use proc_macro::TokenStream;

/// Marks a Rust type as corresponding to a spin-core-net.spin type.
/// Generates a compile-time assertion that the Rust type's structure
/// matches the .spin definition.
#[proc_macro_attribute]
pub fn spin_core(attr: TokenStream, item: TokenStream) -> TokenStream {
    // Placeholder — passes through the item unchanged
    let _ = attr;
    item
}
```

**Step 2: Verify it compiles**

Run: `cargo build --workspace`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add Cargo.toml crates/ Cargo.lock
git commit -m "build: add spin-core-macros proc-macro crate to workspace"
```

---

### Task 11: AST Normalization for Comparison

**Files:**
- Create: `src/ast_normalize.rs`
- Modify: `src/lib.rs`
- Create: `tests/ast_normalize.rs`

**Step 1: Write the failing tests**

Create `tests/ast_normalize.rs`:

```rust
use spin_up::ast::{
    Attribute, ChoiceDef, Field, Item, PrimitiveType, RecordDef, TypeExpr, Variant,
};
use spin_up::ast_normalize::normalize_item;

#[test]
fn test_normalize_strips_spans() {
    let item1 = Item::RecordDef(RecordDef {
        name: "Foo".to_string(),
        attributes: vec![],
        fields: vec![Field {
            name: "x".to_string(),
            ty: TypeExpr::Primitive(PrimitiveType::U32),
            span: 0..10,
        }],
        span: 0..20,
    });

    let item2 = Item::RecordDef(RecordDef {
        name: "Foo".to_string(),
        attributes: vec![],
        fields: vec![Field {
            name: "x".to_string(),
            ty: TypeExpr::Primitive(PrimitiveType::U32),
            span: 50..60,
        }],
        span: 50..100,
    });

    assert_eq!(normalize_item(&item1), normalize_item(&item2));
}

#[test]
fn test_normalize_different_items_not_equal() {
    let item1 = Item::RecordDef(RecordDef {
        name: "Foo".to_string(),
        attributes: vec![],
        fields: vec![Field {
            name: "x".to_string(),
            ty: TypeExpr::Primitive(PrimitiveType::U32),
            span: 0..10,
        }],
        span: 0..20,
    });

    let item2 = Item::RecordDef(RecordDef {
        name: "Bar".to_string(),
        attributes: vec![],
        fields: vec![Field {
            name: "x".to_string(),
            ty: TypeExpr::Primitive(PrimitiveType::U32),
            span: 0..10,
        }],
        span: 0..20,
    });

    assert_ne!(normalize_item(&item1), normalize_item(&item2));
}

#[test]
fn test_normalize_choice() {
    let item = Item::ChoiceDef(ChoiceDef {
        name: "IpAddr".to_string(),
        attributes: vec![Attribute {
            name: "lang-item".to_string(),
            span: 0..11,
        }],
        variants: vec![
            Variant {
                name: "V4".to_string(),
                fields: vec![TypeExpr::Named("IpAddrV4".to_string())],
                span: 0..15,
            },
            Variant {
                name: "V6".to_string(),
                fields: vec![TypeExpr::Named("IpAddrV6".to_string())],
                span: 16..30,
            },
        ],
        span: 0..50,
    });

    let normalized = normalize_item(&item);
    // Verify it's deterministic
    assert_eq!(normalized, normalize_item(&item));
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --test ast_normalize`
Expected: FAIL — `spin_up::ast_normalize` doesn't exist

**Step 3: Implement AST normalization**

Add to `src/lib.rs`:

```rust
pub mod ast_normalize;
```

Create `src/ast_normalize.rs`. The normalization produces a `NormalizedItem` enum (and supporting types) that is identical in structure to the AST types but with all `span` fields removed, and derives `PartialEq, Eq, Debug`. This is a straightforward mechanical transformation:

```rust
/// Normalized AST types with spans stripped for structural comparison.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizedItem {
    RecordDef(NormalizedRecordDef),
    ChoiceDef(NormalizedChoiceDef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedRecordDef {
    pub name: String,
    pub attributes: Vec<NormalizedAttribute>,
    pub fields: Vec<NormalizedField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedChoiceDef {
    pub name: String,
    pub attributes: Vec<NormalizedAttribute>,
    pub variants: Vec<NormalizedVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedAttribute {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedField {
    pub name: String,
    pub ty: NormalizedTypeExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedVariant {
    pub name: String,
    pub fields: Vec<NormalizedTypeExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizedTypeExpr {
    Named(String),
    Primitive(crate::ast::PrimitiveType),
    Path { module: String, name: String },
    Generic { name: String, args: Vec<NormalizedTypeExpr> },
    SelfPath(String),
    Array { element: Box<NormalizedTypeExpr>, size: usize },
    Slice(Box<NormalizedTypeExpr>),
    Tuple(Vec<NormalizedTypeExpr>),
    Unit,
}
```

Implement `pub fn normalize_item(item: &Item) -> NormalizedItem` which recursively converts AST nodes to their normalized equivalents, dropping all span information. Only handle `RecordDef` and `ChoiceDef` variants (those are the only ones that appear in `.spin` module definitions used by the proc-macro). For `ResourceDef` or `SuppliesDef`, panic with a message — they shouldn't be normalized by this path.

**Step 4: Run tests to verify they pass**

Run: `cargo test --test ast_normalize`
Expected: All tests PASS

**Step 5: Commit**

```bash
git add src/ast_normalize.rs src/lib.rs tests/ast_normalize.rs
git commit -m "feat: add AST normalization for span-independent comparison"
```

---

### Task 12: Proc-Macro — `#[spin_core]` Implementation

**Files:**
- Modify: `crates/spin-core-macros/src/lib.rs`
- Modify: `crates/spin-core-macros/Cargo.toml`

**Step 1: Implement the proc-macro**

The `#[spin_core(module = "spin-core-net", resource = "IpAddr")]` attribute macro:

1. Parses the attribute arguments to extract `module` and `resource` names.
2. Gets the embedded `.spin` source for the module via `spin_up::builtins::get_module_source`.
3. Parses the `.spin` source via `spin_up::parser::parse`.
4. Finds the item with a matching `#[lang-item]` attribute and matching name.
5. Generates a spin AST fragment from the Rust `syn` item (struct → RecordDef, enum → ChoiceDef).
6. Normalizes both ASTs via `spin_up::ast_normalize::normalize_item`.
7. Compares them. If they differ, emits a `compile_error!` with a descriptive message.
8. Passes through the original Rust item unchanged.

The proc-macro must be able to map Rust types to spin types. The mapping for fields:
- Rust `u8`, `u16`, etc. → `TypeExpr::Primitive(PrimitiveType::U8)`, etc.
- Rust struct name reference (e.g., `IpAddrV4`) → `TypeExpr::Named("IpAddrV4")`
- Rust `[u8; N]` → `TypeExpr::Array { element, size: N }`

Use `syn` to parse the Rust item and `quote` to emit the output. The heavy lifting is in the Rust-to-spin-AST conversion function.

**Step 2: Verify it compiles**

Run: `cargo build --workspace`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add crates/spin-core-macros/
git commit -m "feat: implement #[spin_core] proc-macro for compile-time verification"
```

---

### Task 13: Rust Types for `spin-core-net` with `#[spin_core]`

**Files:**
- Create: `src/core_net.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml` (add `spin-core-macros` dependency)

**Step 1: Add dependency on proc-macro crate**

Add to root `Cargo.toml` under `[dependencies]`:

```toml
spin-core-macros = { path = "crates/spin-core-macros" }
```

**Step 2: Write the Rust types with `#[spin_core]`**

Add to `src/lib.rs`:

```rust
pub mod core_net;
```

Create `src/core_net.rs`:

```rust
use spin_core_macros::spin_core;

#[spin_core(module = "spin-core-net", resource = "IpAddrV4")]
pub struct IpAddrV4 {
    pub octets: [u8; 4],
}

#[spin_core(module = "spin-core-net", resource = "IpAddrV6")]
pub struct IpAddrV6 {
    pub octets: [u8; 16],
}

#[spin_core(module = "spin-core-net", resource = "IpAddr")]
pub enum IpAddr {
    V4(IpAddrV4),
    V6(IpAddrV6),
}

#[spin_core(module = "spin-core-net", resource = "SocketAddrV4")]
pub struct SocketAddrV4 {
    pub ip: IpAddrV4,
    pub port: u16,
}

#[spin_core(module = "spin-core-net", resource = "SocketAddrV6")]
pub struct SocketAddrV6 {
    pub ip: IpAddrV6,
    pub port: u16,
}

#[spin_core(module = "spin-core-net", resource = "SocketAddr")]
pub enum SocketAddr {
    V4(SocketAddrV4),
    V6(SocketAddrV6),
}
```

**Step 3: Verify compile-time verification works**

Run: `cargo build`
Expected: Compiles successfully — the proc-macro verifies all 6 types match their `.spin` definitions.

**Step 4: Test that drift is caught**

Temporarily add a field to `IpAddrV4` in `core_net.rs` (e.g., `pub extra: u8`). Run `cargo build`. Expected: compile error from the proc-macro. Revert the change.

**Step 5: Commit**

```bash
git add src/core_net.rs src/lib.rs Cargo.toml Cargo.lock
git commit -m "feat: add spin-core-net Rust types with compile-time verification"
```

---

## What's Next (Future Phases)

**Phase 3: Static Analysis** — dependency graph construction, type unification (requires brainstorming session), consumer/provider matching.

**Phase 4: Runtime** — supervision tree, Unix socket IPC, process lifecycle.

**Phase 5: Language Completions** — string interpolation, conditionals, functions, map/filter.
