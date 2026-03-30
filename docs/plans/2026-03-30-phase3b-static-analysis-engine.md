# Phase 3b: Static Analysis Engine — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use cipherpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the static analysis engine that performs module resolution, type unification, constraint checking, and dependency graph construction — with Rust-quality error reporting via `miette`.

**Architecture:** A new `analysis` module in `spin-lang` with three sub-modules: `resolve` (module/name resolution), `unify` (structural type matching), and `check` (constraint evaluation + DAG verification). Diagnostics collected as `Vec<Diagnostic>` throughout, reported together at the end. A `spin check` CLI subcommand invokes the full pipeline.

**Tech Stack:** Rust 2024 edition, `miette` for diagnostics. All analysis code in `crates/spin-lang/`.

---

### Task 1: Diagnostic Infrastructure

**Files:**
- Create: `crates/spin-lang/src/diagnostics.rs`
- Modify: `crates/spin-lang/src/lib.rs`
- Create: `tests/diagnostics.rs`

**Step 1: Write the failing test**

```rust
use spin_up::diagnostics::{Diagnostic, DiagnosticKind, Diagnostics};

#[test]
fn test_diagnostics_collect_multiple_errors() {
    let mut diags = Diagnostics::new();
    diags.error(DiagnosticKind::TypeMismatch {
        expected: "u32".to_string(),
        found: "str".to_string(),
    }, 0..10, "test.spin");

    diags.error(DiagnosticKind::UnknownType {
        name: "Foo".to_string(),
    }, 20..25, "test.spin");

    assert_eq!(diags.errors().len(), 2);
    assert!(!diags.is_ok());
}

#[test]
fn test_diagnostics_empty_is_ok() {
    let diags = Diagnostics::new();
    assert!(diags.is_ok());
    assert_eq!(diags.errors().len(), 0);
}
```

**Step 2: Implement diagnostics module**

Create `crates/spin-lang/src/diagnostics.rs`:

```rust
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub span: Range<usize>,
    pub source_name: String,
}

#[derive(Debug, Clone)]
pub enum DiagnosticKind {
    // Type unification errors
    TypeMismatch { expected: String, found: String },
    UnknownType { name: String },
    UnknownInterface { name: String },
    MissingField { field: String, interface: String },
    DuplicateField { field: String },
    RedefinitionTypeMismatch { name: String, expected: String, found: String },

    // Impl errors
    InvalidDelegate { reason: String },
    InvalidAsInterface { type_name: String, interface: String },

    // Constraint errors
    ConstraintViolation { description: String },
    InvalidPredicate { description: String },

    // Graph errors
    CyclicDependency { cycle: Vec<String> },

    // Resolution errors
    UnresolvedImport { module: String },
    CircularImport { chain: Vec<String> },
}

#[derive(Debug, Default)]
pub struct Diagnostics {
    errors: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn error(&mut self, kind: DiagnosticKind, span: Range<usize>, source_name: &str) {
        self.errors.push(Diagnostic {
            kind,
            span,
            source_name: source_name.to_string(),
        });
    }

    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn merge(&mut self, other: Diagnostics) {
        self.errors.extend(other.errors);
    }
}
```

Add `pub mod diagnostics;` to `crates/spin-lang/src/lib.rs`.

**Step 3: Run tests, verify pass, commit**

```bash
git commit -m "feat: add diagnostic infrastructure for error collection"
```

---

### Task 2: Symbol Table / Type Registry

**Files:**
- Create: `crates/spin-lang/src/analysis/mod.rs`
- Create: `crates/spin-lang/src/analysis/registry.rs`
- Modify: `crates/spin-lang/src/lib.rs`
- Create: `tests/analysis_registry.rs`

The registry holds all known types, interfaces, and let bindings, indexed by name. It's built from parsed modules before analysis begins.

**Step 1: Write the failing tests**

```rust
use spin_up::analysis::registry::TypeRegistry;
use spin_up::parser;

#[test]
fn test_registry_registers_type() {
    let module = parser::parse("type Foo = x: u32;").unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);
    assert!(registry.lookup_type("Foo").is_some());
}

#[test]
fn test_registry_registers_interface() {
    let module = parser::parse("interface Bar = x: u32;").unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);
    assert!(registry.lookup_interface("Bar").is_some());
}

#[test]
fn test_registry_registers_let_binding() {
    let module = parser::parse("let x = 42").unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);
    assert!(registry.lookup_binding("x").is_some());
}

#[test]
fn test_registry_registers_impl() {
    let module = parser::parse("impl Foo for Bar {\n  x: self.x,\n}").unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);
    let impls = registry.lookup_impls_for_type("Bar");
    assert_eq!(impls.len(), 1);
}

#[test]
fn test_registry_unknown_type_returns_none() {
    let registry = TypeRegistry::new();
    assert!(registry.lookup_type("Unknown").is_none());
}
```

**Step 2: Implement the registry**

Create `crates/spin-lang/src/analysis/mod.rs`:

```rust
pub mod registry;
```

Create `crates/spin-lang/src/analysis/registry.rs` with a `TypeRegistry` that stores:
- `types: HashMap<String, &RecordDef or &ChoiceDef>` — all type definitions
- `interfaces: HashMap<String, &InterfaceDef>` — all interface definitions
- `bindings: HashMap<String, &LetBinding>` — all let bindings
- `impls: Vec<&ImplBlock>` — all impl blocks (looked up by type name or interface name)

Use owned clones rather than references for simplicity (clone the AST nodes during registration).

Add `pub mod analysis;` to `crates/spin-lang/src/lib.rs`.

**Step 3: Run tests, verify pass, commit**

```bash
git commit -m "feat: add type registry for symbol resolution"
```

---

### Task 3: Module Resolution

**Files:**
- Create: `crates/spin-lang/src/analysis/resolve.rs`
- Create: `tests/analysis_resolve.rs`

**Step 1: Write the failing tests**

```rust
use spin_up::analysis::resolve::resolve_modules;
use spin_up::diagnostics::DiagnosticKind;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_resolve_single_module() {
    let tmp = TempDir::new().unwrap();
    let spin_file = tmp.path().join("main.spin");
    fs::write(&spin_file, "type Foo = x: u32;").unwrap();

    let result = resolve_modules(&spin_file, &[tmp.path().to_path_buf()]);
    assert!(result.diagnostics.is_ok());
    assert!(result.registry.lookup_type("Foo").is_some());
}

#[test]
fn test_resolve_with_import() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("types.spin"), "type Bar = y: str;").unwrap();
    fs::write(tmp.path().join("main.spin"), "import types\ntype Foo = x: Bar;").unwrap();

    let main_file = tmp.path().join("main.spin");
    let result = resolve_modules(&main_file, &[tmp.path().to_path_buf()]);
    assert!(result.diagnostics.is_ok());
    assert!(result.registry.lookup_type("Bar").is_some());
    assert!(result.registry.lookup_type("Foo").is_some());
}

#[test]
fn test_resolve_unresolved_import() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("main.spin"), "import nonexistent").unwrap();

    let main_file = tmp.path().join("main.spin");
    let result = resolve_modules(&main_file, &[tmp.path().to_path_buf()]);
    assert!(!result.diagnostics.is_ok());
    assert!(matches!(
        &result.diagnostics.errors()[0].kind,
        DiagnosticKind::UnresolvedImport { .. }
    ));
}
```

**Step 2: Implement module resolution**

Create `crates/spin-lang/src/analysis/resolve.rs`:

The `resolve_modules` function:
1. Takes an entry point `.spin` file path and SPIN_PATH directories
2. Parses the entry point module
3. Recursively resolves imports (using `SpinPath::resolve_source` or `builtins::get_module_source`)
4. Checks for circular imports
5. Registers all items from all resolved modules into a `TypeRegistry`
6. Returns `ResolveResult { registry: TypeRegistry, diagnostics: Diagnostics }`

**Step 3: Run tests, verify pass, commit**

```bash
git commit -m "feat: add module resolution with import checking"
```

---

### Task 4: Type Unification — Core Engine

**Files:**
- Create: `crates/spin-lang/src/analysis/unify.rs`
- Create: `tests/analysis_unify.rs`

**Step 1: Write the failing tests**

```rust
use spin_up::analysis::registry::TypeRegistry;
use spin_up::analysis::unify::unify;
use spin_up::parser;

#[test]
fn test_unify_complete_impl() {
    let source = r#"
interface Endpoint = host: str, port: u16;
type Server = hostname: str, port_num: u16;
impl Endpoint for Server {
  host: self.hostname,
  port: self.port_num,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(diags.is_ok(), "expected no errors, got: {:?}", diags.errors());
}

#[test]
fn test_unify_missing_required_field() {
    let source = r#"
interface Endpoint = host: str, port: u16;
type Server = hostname: str;
impl Endpoint for Server {
  host: self.hostname,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::MissingField { field, .. } if field == "port"
    ));
}

#[test]
fn test_unify_optional_field_can_be_omitted() {
    let source = r#"
interface Endpoint = host: str, tls: Option<TlsConfig>;
type Server = hostname: str;
impl Endpoint for Server {
  host: self.hostname,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(diags.is_ok(), "Option fields should be omittable: {:?}", diags.errors());
}

#[test]
fn test_unify_default_field_can_be_omitted() {
    let source = r#"
interface Endpoint =
  host: str,
  #[default("localhost")]
  fallback: str,
;
type Server = hostname: str;
impl Endpoint for Server {
  host: self.hostname,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(diags.is_ok(), "Default fields should be omittable: {:?}", diags.errors());
}

#[test]
fn test_unify_unknown_interface_error() {
    let source = r#"
type Server = hostname: str;
impl NonExistent for Server {
  host: self.hostname,
}
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = unify(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::UnknownInterface { name } if name == "NonExistent"
    ));
}
```

**Step 2: Implement type unification**

Create `crates/spin-lang/src/analysis/unify.rs`:

The `unify` function takes a `&TypeRegistry` and returns `Diagnostics`. It:

1. For each `ImplBlock` in the registry:
   - Look up the interface by name — if not found, emit `UnknownInterface` error
   - Look up the implementing type by name — if not found, emit `UnknownType` error
   - For each interface field:
     - If the impl block has a mapping for this field → valid (type checking of the RHS expression is deferred to a later refinement)
     - If no mapping: check if the field has `#[default(...)]` attribute → valid
     - If no mapping: check if the field type is `Option<...>` (TypeExpr::Generic with name "Option") → valid
     - Otherwise → emit `MissingField` error

This is the structural check only — actual type inference of RHS expressions is complex and deferred. The immediate value is verifying impl completeness.

**Step 3: Run tests, verify pass, commit**

```bash
git commit -m "feat: add type unification engine with impl completeness checking"
```

---

### Task 5: Dependency Graph and DAG Verification

**Files:**
- Create: `crates/spin-lang/src/analysis/graph.rs`
- Create: `tests/analysis_graph.rs`

**Step 1: Write the failing tests**

```rust
use spin_up::analysis::graph::{build_dependency_graph, DependencyGraph};
use spin_up::analysis::registry::TypeRegistry;
use spin_up::parser;

#[test]
fn test_graph_simple_dependency() {
    let source = r#"
type Database = port: u16;
let db = Database { port: 5432 }
let app = MyApp { database: db }
type MyApp = database: Database;
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let graph = build_dependency_graph(&registry);
    assert!(graph.diagnostics.is_ok());
    let order = graph.topological_order();
    // db should come before app
    let db_pos = order.iter().position(|n| n == "db").unwrap();
    let app_pos = order.iter().position(|n| n == "app").unwrap();
    assert!(db_pos < app_pos);
}

#[test]
fn test_graph_detects_cycle() {
    let source = r#"
let a = Foo { dep: b }
let b = Bar { dep: a }
type Foo = dep: Bar;
type Bar = dep: Foo;
"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let graph = build_dependency_graph(&registry);
    assert!(!graph.diagnostics.is_ok());
    assert!(matches!(
        &graph.diagnostics.errors()[0].kind,
        DiagnosticKind::CyclicDependency { .. }
    ));
}
```

**Step 2: Implement dependency graph**

Create `crates/spin-lang/src/analysis/graph.rs`:

Build a directed graph from `let` bindings. Walk each binding's expression to find references to other bindings (Expr::Ident that matches a known binding name). Add edges. Run topological sort — if it fails, report a cycle.

Use a simple adjacency list and Kahn's algorithm or DFS-based topological sort.

**Step 3: Run tests, verify pass, commit**

```bash
git commit -m "feat: add dependency graph construction and cycle detection"
```

---

### Task 6: `spin check` CLI Subcommand

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Modify: `tests/cli.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn test_spin_check_subcommand_exists() {
    Command::cargo_bin("spin")
        .unwrap()
        .arg("check")
        .assert()
        .success();
}
```

**Step 2: Add Check variant to CLI**

Add `Check` to the `Command` enum in `src/cli.rs`. In `src/main.rs`, add a match arm that prints "spin check: not yet implemented" (the actual analysis pipeline integration is a follow-up).

**Step 3: Run tests, verify pass, commit**

```bash
git commit -m "feat: add spin check CLI subcommand"
```

---

## What's Next

After Phase 3b:
- **Miette integration** — render diagnostics with labelled source spans, colours, help text
- **Type inference for impl RHS expressions** — verify that `self.hostname` actually has type `str` when the interface field expects `str`
- **Constraint checking** (Phase 3 of static analysis) — evaluate predicates
- **Phase 4: Runtime** — supervision tree, Unix socket IPC
