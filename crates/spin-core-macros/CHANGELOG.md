# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/freshtonic/spin-up/releases/tag/spin-core-macros-v0.1.0) - 2026-03-31

### Added

- add span information to Expr and TypeExpr via Spanned<T> wrapper
- add #[] #() #{} collection literals, built-in functions, =~ regex match
- overhaul type system — 3 primitives, collections, regex, it-expressions
- add expression, interface, impl, and let binding AST types
- add attribute argument parsing
- implement #[spin_core] proc-macro for compile-time verification

### Other

- convert spin\! to proc-macro and migrate all tests
- remove supplies keyword, replaced by impl Interface for Type
- remove resource keyword, use type for all type definitions
- change type syntax to = / | / ; delimiters with generic params
- extract spin-lang crate to break cyclic dependency
- add spin-core-macros proc-macro crate to workspace
