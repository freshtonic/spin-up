use proc_macro::TokenStream;

mod spin_core_impl;
mod spin_macro_impl;

/// Marks a Rust type as corresponding to a spin-core-net.spin type.
/// Generates a compile-time assertion that the Rust type's structure
/// matches the .spin definition.
#[proc_macro_attribute]
pub fn spin_core(attr: TokenStream, item: TokenStream) -> TokenStream {
    spin_core_impl::spin_core_impl(attr.into(), item.into()).into()
}

/// Parse inline .spin source code into a [`spin_lang::ast::Module`] AST at runtime.
///
/// Unlike the previous `macro_rules!` version, this proc-macro faithfully preserves:
/// - `#[attr]` attributes (which conflicted with Rust's attribute parsing)
/// - `${expr}` string interpolation (which broke because Rust treats `$` specially)
///
/// # Example
///
/// ```ignore
/// use spin_core_macros::spin;
///
/// let module = spin! {
///     #[lang-item]
///     type Foo = x: u32;
/// };
/// ```
#[proc_macro]
pub fn spin(input: TokenStream) -> TokenStream {
    let source = spin_macro_impl::tokens_to_spin_source(input.into());
    let source_lit = proc_macro2::Literal::string(&source);
    let output: proc_macro2::TokenStream = quote::quote! {
        spin_lang::parser::parse(#source_lit).expect("spin! macro: parse error")
    };
    output.into()
}
