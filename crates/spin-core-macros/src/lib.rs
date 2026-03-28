use proc_macro::TokenStream;

mod spin_core_impl;

/// Marks a Rust type as corresponding to a spin-core-net.spin type.
/// Generates a compile-time assertion that the Rust type's structure
/// matches the .spin definition.
#[proc_macro_attribute]
pub fn spin_core(attr: TokenStream, item: TokenStream) -> TokenStream {
    spin_core_impl::spin_core_impl(attr.into(), item.into()).into()
}
