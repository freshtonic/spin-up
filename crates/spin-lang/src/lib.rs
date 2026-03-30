pub mod analysis;
pub mod ast;
pub mod ast_normalize;
pub mod builtins;
pub mod diagnostics;
pub mod lexer;
pub mod parser;
pub mod spin_path;

/// Parse inline .spin source code into a [`ast::Module`] AST at runtime.
///
/// This macro stringifies its token input and passes it through `parser::parse`,
/// making it convenient to write .spin code directly in Rust (especially in tests).
///
/// # Limitations
///
/// - Rust's tokenizer processes the input before `stringify!` sees it, so some
///   syntax may be altered. In particular, `#[attr]` attributes inside the macro
///   invocation may conflict with Rust's attribute parsing and are not supported.
///   Use `parser::parse()` directly for .spin code containing attributes.
/// - `${}` string interpolation is not supported inside the macro because Rust
///   treats `$` specially in macro contexts.
///
/// # Example
///
/// ```
/// let module = spin_lang::spin! {
///     type Foo = x: u32, y: str;
/// };
/// assert_eq!(module.items.len(), 1);
/// ```
#[macro_export]
macro_rules! spin {
    ($($tt:tt)*) => {{
        let source = stringify!($($tt)*);
        $crate::parser::parse(source).expect("spin! macro: parse error")
    }};
}
