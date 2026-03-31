# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/freshtonic/spin-up/releases/tag/spin-lang-v0.1.0) - 2026-03-31

### Added

- add span information to Expr and TypeExpr via Spanned<T> wrapper
- add #[] #() #{} collection literals, built-in functions, =~ regex match
- overhaul type system — 3 primitives, collections, regex, it-expressions
- add spin\! macro for inline .spin code in Rust
- add string interpolation parsing with ${expr} support
- add let binding redefinition type checking
- add as-interface validation for type constructions
- add constraint satisfiability checking with bound analysis
- add delegate/target validation with field-level attributes
- add constraint checking for it-predicate validation
- add type inference for impl RHS expressions
- integrate miette for Rust-quality diagnostic rendering
- wire up spin check to analysis pipeline
- add dependency graph construction and cycle detection
- add type unification engine with impl completeness checking
- add module resolution with import checking
- add type registry for symbol resolution
- add diagnostic infrastructure for error collection
- add <as Interface> block parsing in type constructions
- add type annotation support for let bindings
- add impl block parsing
- add expression parser with operator precedence and let bindings
- add interface definition parsing with field-level attributes
- add expression, interface, impl, and let binding AST types
- add attribute argument parsing
- add logical operators &&, ||, \! to lexer
- add interface, impl, for, let, it keywords to lexer

### Fixed

- numeric type inference returns Number instead of Unknown

### Other

- convert spin\! to proc-macro and migrate all tests
- remove supplies keyword, replaced by impl Interface for Type
- remove resource keyword, use type for all type definitions
- change type syntax to = / | / ; delimiters with generic params
- replace record/choice keywords with unified type keyword
- extract spin-lang crate to break cyclic dependency
