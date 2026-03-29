# Spin Core Primitives — Design Document

The spin type system has three layers: primitives (fundamental, no module), `spin-core` intrinsics (auto-imported), and `spin-core-*.spin` modules (embedded in the binary with compile-time verification against Rust types).

## Primitives

Primitives are fundamental types not defined in any `.spin` module. They are always available without importing.

**Full list:**
- `bool`
- Integers: `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128`
- Floats: `f32`, `f64`
- `str`
- `[T; N]` — fixed-size array
- `[T]` — slice (size unknown statically)
- `()` — unit type
- `(T1, T2, ...)` — tuple types

Primitive type names are keywords in the lexer — they cannot be shadowed by user definitions.

**Numeric literal syntax is identical to Rust's:** decimal (`42`, `1_000`), hex (`0xff`), binary (`0b1010`), octal (`0o77`), float (`3.14`, `1.0e10`), with type suffixes (`42u32`, `3.14f64`).

Primitives map directly to Rust equivalents. Since they are Rust builtins, the `#[spin-core]` proc-macro cannot be applied — structural equivalence is verified via manual tests.

## `spin-core` — Language Intrinsics

`spin-core` is an intrinsic module. Its types are auto-imported and always available. Fully-qualified names use the `spin-core::<name>` form but users never need to write that.

### Structural Building Blocks

- **Record** — product type (struct-like). Fields are named and typed.
- **Choice** — sum type (enum-like). Variants can carry data.

### Built-in Types

- `Option<T>` — sum type with `Some(T)` / `None` variants.
- `Result<T>` — sum type with `Ok(T)` / `Err(Error)` variants. Unlike Rust, the error type is always `spin-core::Error` — not parameterized.
- `Error` — opaque to users. They can receive and propagate it but not construct or inspect it.
- `Set<T>` — unordered collection of unique values.
- `Map<K, V>` — key-value mapping.

`Set` and `Map` are intrinsic types whose internals are not expressed in `.spin` source. No structural equivalence checking is performed for them.

`Option` and `Result` correspond to Rust's `Option<T>` and `Result<T, SpinError>`. Since they are std types, the `#[spin-core]` proc-macro cannot be applied — structural equivalence is verified via manual tests.

### Core Traits

All types — including user-defined resources — implicitly implement all core traits. No explicit `impl` or `derives` needed.

- `Eq`, `Ord`, `Display`, `Hash` — same semantics as Rust.
- `Mappable` — Functor (`fmap`). No direct Rust equivalent.
- `Applicable` — Applicative. No direct Rust equivalent.

`Mappable` and `Applicable` require Rank-2 type checking, with implications for the type unification system in Phase 3 (static analysis).

## `spin-core-net.spin` — Networking Types

The first `spin-core-*` module. Written in `.spin` source, embedded in the binary via `include_str!`, with corresponding Rust types verified at compile time.

### Types

- `IpAddrV4` — IPv4 address
- `IpAddrV6` — IPv6 address
- `IpAddr` — sum type: `V4(IpAddrV4)` / `V6(IpAddrV6)`
- `SocketAddrV4` — IPv4 address + port (`IpAddrV4`, `u16`)
- `SocketAddrV6` — IPv6 address + port (`IpAddrV6`, `u16`)
- `SocketAddr` — sum type: `V4(SocketAddrV4)` / `V6(SocketAddrV6)`

These align with Rust's `std::net` equivalents.

### IP Literal Syntax

- Infallible: `IP:"192.168.1.1"` — parsed and validated statically at compile time. Must be a literal string, no interpolation. Produces `IpAddr::V4(...)` or `IpAddr::V6(...)`.
- Fallible: `IP!:"${some_string}"` — allows interpolation, returns `Result<IpAddr>`. Parsed at runtime.

## The `#[spin-core]` / `#[lang-item]` Proc-Macro System

This applies only to `spin-core-*.spin` modules (currently `spin-core-net.spin`). Intrinsics and primitives do not use it.

### `.spin` side

Types are annotated with `#[lang-item]`, indicating they have a Rust-native implementation:

```
#[lang-item]
type IpAddr = V4(IpAddrV4) | V6(IpAddrV6);
```

### Rust side

The corresponding type is annotated with `#[spin_core]`:

```rust
#[spin_core(module = "spin-core-net", resource = "IpAddr")]
enum IpAddr {
    V4(IpAddrV4),
    V6(IpAddrV6),
}
```

### What the proc-macro does at compile time

1. Generates a spin AST fragment from the Rust type definition.
2. Parses the corresponding `.spin` source (found via `include_str!` of `spin-core-net.spin`).
3. Locates the matching `#[lang-item]` definition by name.
4. Normalizes both ASTs (strips spans, whitespace).
5. Asserts structural equality — **build fails if they diverge**.

### What needs manual tests instead

- Primitives — Rust builtins, cannot annotate.
- `Option<T>`, `Result<T>`, `Set<T>`, `Map<K, V>` — std types, cannot annotate.
- `Error` — opaque, no user-visible structure to verify.

## Lexer Changes

The lexer needs updates to support the full primitive type system and new syntax.

### Numeric literals (Rust-style)

- Hex: `0xff`, `0xFF`
- Binary: `0b1010`
- Octal: `0o77`
- Underscores as separators: `1_000_000`, `0xff_ff`
- Type suffixes: `42u32`, `3.14f64`
- Float exponents: `1.0e10`, `2.5E-3`

### New keywords

- Primitive type names: `bool`, `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128`, `f32`, `f64`, `str`
- `type` for product and sum type definitions

### New tokens

- `#[` and `]` for attribute syntax
- `IP:` and `IP!:` for IP literal syntax

## Future Work

- Filesystem types (`TempDir`, `FilePath`, etc.) — to be designed later.
