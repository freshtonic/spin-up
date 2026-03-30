# Static Analysis — Design Document

Static analysis runs before any side effects — no resources allocated, no workloads launched. The `spin check` command runs all phases without launching anything.

## Three Phases

### Phase 1: Module Resolution & Import Checking

- Resolve all imports via `SPIN_PATH` and builtins
- Verify no circular module dependencies
- Build a module dependency graph

### Phase 2: Type Unification

Structural field-by-field matching. Answers the question: "Given a type `T` that `impl Interface`, do all the field types match structurally?"

**What it verifies:**

- All `impl Interface for Type` blocks are complete: every interface field is mapped, unless it has `#[default(...)]` or is `Option<T>` (implicitly `None`)
- Field type compatibility through `impl` mappings — the RHS expression's type unifies with the interface field's declared type
- `#[delegate(Interface)]` / `#[target(Interface)]` correctness
- `<as Interface> { ... }` blocks reference valid interface implementations
- `let` binding instantiations: every provided field value's type unifies with the field's declared type
- `let` binding redefinitions: the type must not change
- Generic type parameter propagation (e.g., `Option<SocketAddr>` expects `T = SocketAddr`)

**The algorithm:**

1. For each `impl Interface for Type` block:
   - Collect all fields declared by the interface
   - For each field with a mapping: infer the RHS expression type, unify with the interface field's declared type
   - For each missing field: verify it has `#[default(...)]` or is `Option<T>` — otherwise error

2. For each `let` binding that instantiates a type:
   - Check every provided field value's type unifies with the declared type
   - For `<as Interface> { ... }` blocks, resolve the implementation and check through the mapping

3. For generic types:
   - Unify type parameters by propagation — bind type variables as they're encountered

**Errors are collected, not fatal.** The unifier continues past each error to report all type errors in one pass.

### Phase 3: Constraint Checking

- Evaluate predicate constraints (e.g., `it >= 15 && it < 17`) against resolved values
- Verify `#[default(...)]` values satisfy the field's type
- Verify the resolved dependency graph is a DAG (no cycles)

## Error Reporting

Errors are collected across all phases and reported together. The bar is Rust compiler quality — labelled source spans, "expected X, found Y", suggestions, multi-line underlines, "help:" hints, and chain-of-reasoning notes. `miette` (already a dependency) provides the diagnostic infrastructure.

**Error categories:**

- **Missing field** — `impl` block omits a required field (no default, not `Option`)
- **Type mismatch** — field value type doesn't unify with declared type
- **Unknown interface** — `impl Foo for Bar` where `Foo` isn't a defined interface
- **Unknown type** — reference to a type that doesn't exist in scope
- **Invalid delegate** — `#[delegate(Interface)]` without a matching `#[target(Interface)]` field, or target field doesn't implement the interface
- **Invalid `<as Interface>`** — the type doesn't implement the referenced interface
- **Duplicate field** — same field provided twice in an instantiation
- **Redefinition type mismatch** — `let` binding redefined with a different type
- **Constraint violation** — predicate evaluates to false (Phase 3)
- **Invalid predicate** — predicate uses operations not supported by the type (Phase 3)

## New Language Constructs

This design introduces several constructs not yet implemented in the parser:

**`interface` definitions** — abstract contracts with named typed fields. Fields can have `#[default(...)]` attributes.

```
interface PostgresEndpoint =
  #[default(SocketAddr::V4(port: 5432))]
  listens_on: SocketAddr,

  #[default("postgres")]
  user: str,

  password: str,
;
```

**`impl Interface for Type { ... }`** — maps interface fields to expressions on `self`. `self` is in scope but nothing else. All interface fields must be mapped unless they have `#[default(...)]` or are `Option<T>`.

```
impl PostgresEndpoint for PostgresDevContainer {
  listen_on: self.listen_on,
  user: self.endpoint.user,
  password: self.endpoint.password,
}
```

**`#[delegate(Interface)]` / `#[target(Interface)]`** — sugar for implementing an interface by forwarding to a field.

```
#[delegate(PostgresEndpoint)]
type Proxy =
  #[target(PostgresEndpoint)]
  frontend: PostgresEndpoint,
  backend: PostgresEndpoint,
;
```

**`let` bindings** — lazily evaluated, can be redefined (same type) at the top level. Redefinitions are visible to all prior lazy references.

```
let proxy = Proxy {
  upstream: PostgresDevContainer {
    pg_version: SemVer(major: 17),
  }
}
```

**`<as Interface> { ... }`** — disambiguate fields when constraining through a specific interface implementation.

```
PostgresDevContainer {
  pg_version: SemVer(major: it >= 15),

  <as PostgresEndpoint> {
    listen_on: SocketAddr::V4(port: 5432)
  }
}
```

**Constraint predicates with `it`** — `it` is a keyword representing the value being constrained. Used in boolean expressions on field values.

```
pg_version: SemVer(major: it >= 15 && it < 17)
```

## Dependency Graph

After type unification, spin builds a dependency graph from resolved `let` bindings. Each binding that instantiates a type is a node; field references create edges. The graph must be a DAG — cycles are a static error.

Whether something is a workload (supervised with lifecycle) vs a resource (just data) is determined by whether the type implements the `Lifecycle` interface. No attributes needed.

## Scoping Model

- `let` bindings are module-scoped
- Bindings must be imported (or fully qualified) to be referenced from other modules
- The top-level module (entry point) can redefine any imported binding
- Redefinitions must preserve the type — only values/constraints can change
- Due to lazy evaluation, all references see the final redefined value

## Deferred

- Compile-time literals (`SemVer:"17.0.0"`, `IP:"192.168.1.1"`)
- `#[lifecycle(...)]` attributes (`no_wait`, `optional`, `restart_on_failure`)
- `Set<T>`, `Map<K, V>`
