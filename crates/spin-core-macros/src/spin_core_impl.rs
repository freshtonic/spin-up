use proc_macro2::TokenStream;
use quote::quote;
use spin_lang::ast;
use spin_lang::ast_normalize::{
    NormalizedChoiceDef, NormalizedField, NormalizedItem, NormalizedRecordDef, NormalizedTypeExpr,
    NormalizedVariant,
};
use syn::{Fields, Item, Type};

/// Parse the `module = "..."` and `resource = "..."` from the attribute arguments.
fn parse_attr_args(attr: &TokenStream) -> Result<(String, String), syn::Error> {
    let mut module = None;
    let mut resource = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("module") {
            let value = meta.value()?;
            let lit: syn::LitStr = value.parse()?;
            module = Some(lit.value());
            Ok(())
        } else if meta.path.is_ident("resource") {
            let value = meta.value()?;
            let lit: syn::LitStr = value.parse()?;
            resource = Some(lit.value());
            Ok(())
        } else {
            Err(meta.error("expected `module` or `resource`"))
        }
    });

    syn::parse::Parser::parse2(parser, attr.clone())?;

    let module = module
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "missing `module`"))?;
    let resource = resource
        .ok_or_else(|| syn::Error::new(proc_macro2::Span::call_site(), "missing `resource`"))?;

    Ok((module, resource))
}

/// Convert a Rust `syn::Type` to a spin `NormalizedTypeExpr`.
fn rust_type_to_normalized(ty: &Type) -> Result<NormalizedTypeExpr, String> {
    match ty {
        Type::Path(type_path) => {
            let seg = type_path
                .path
                .segments
                .last()
                .ok_or_else(|| "empty type path".to_string())?;
            let name = seg.ident.to_string();
            match name.as_str() {
                "bool" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::Bool)),
                "u8" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::U8)),
                "u16" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::U16)),
                "u32" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::U32)),
                "u64" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::U64)),
                "u128" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::U128)),
                "i8" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::I8)),
                "i16" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::I16)),
                "i32" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::I32)),
                "i64" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::I64)),
                "i128" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::I128)),
                "f32" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::F32)),
                "f64" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::F64)),
                "str" => Ok(NormalizedTypeExpr::Primitive(ast::PrimitiveType::Str)),
                other => Ok(NormalizedTypeExpr::Named(other.to_string())),
            }
        }
        Type::Array(array) => {
            let element = rust_type_to_normalized(&array.elem)?;
            let size = match &array.len {
                syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                    syn::Lit::Int(lit_int) => lit_int
                        .base10_parse::<usize>()
                        .map_err(|e| format!("invalid array size: {e}"))?,
                    _ => return Err("array size must be an integer literal".to_string()),
                },
                _ => return Err("array size must be a literal expression".to_string()),
            };
            Ok(NormalizedTypeExpr::Array {
                element: Box::new(element),
                size,
            })
        }
        _ => Err(format!("unsupported Rust type: {}", quote!(#ty))),
    }
}

/// Convert a Rust struct (syn::ItemStruct) to a NormalizedItem::RecordDef.
fn struct_to_normalized(item: &syn::ItemStruct) -> Result<NormalizedItem, String> {
    let name = item.ident.to_string();
    let fields = match &item.fields {
        Fields::Named(named) => named
            .named
            .iter()
            .map(|f| {
                let field_name = f
                    .ident
                    .as_ref()
                    .ok_or_else(|| "struct field missing name".to_string())?
                    .to_string();
                let ty = rust_type_to_normalized(&f.ty)?;
                Ok(NormalizedField {
                    name: field_name,
                    ty,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        _ => return Err("only named fields are supported for record mapping".to_string()),
    };

    Ok(NormalizedItem::RecordDef(NormalizedRecordDef {
        name,
        type_params: vec![],
        attributes: vec![],
        fields,
    }))
}

/// Convert a Rust enum (syn::ItemEnum) to a NormalizedItem::ChoiceDef.
fn enum_to_normalized(item: &syn::ItemEnum) -> Result<NormalizedItem, String> {
    let name = item.ident.to_string();
    let variants = item
        .variants
        .iter()
        .map(|v| {
            let variant_name = v.ident.to_string();
            let fields = match &v.fields {
                Fields::Unnamed(unnamed) => unnamed
                    .unnamed
                    .iter()
                    .map(|f| rust_type_to_normalized(&f.ty))
                    .collect::<Result<Vec<_>, String>>()?,
                Fields::Unit => vec![],
                Fields::Named(_) => {
                    return Err("named fields in enum variants are not supported".to_string());
                }
            };
            Ok(NormalizedVariant {
                name: variant_name,
                fields,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    Ok(NormalizedItem::ChoiceDef(NormalizedChoiceDef {
        name,
        type_params: vec![],
        attributes: vec![],
        variants,
    }))
}

/// Strip attributes from a NormalizedItem for comparison.
fn strip_attributes(item: &NormalizedItem) -> NormalizedItem {
    match item {
        NormalizedItem::RecordDef(record) => NormalizedItem::RecordDef(NormalizedRecordDef {
            name: record.name.clone(),
            type_params: record.type_params.clone(),
            attributes: vec![],
            fields: record.fields.clone(),
        }),
        NormalizedItem::ChoiceDef(choice) => NormalizedItem::ChoiceDef(NormalizedChoiceDef {
            name: choice.name.clone(),
            type_params: choice.type_params.clone(),
            attributes: vec![],
            variants: choice.variants.clone(),
        }),
    }
}

/// Find an item by name in a parsed .spin module.
fn find_item_by_name<'a>(items: &'a [ast::Item], name: &str) -> Option<&'a ast::Item> {
    items.iter().find(|item| match item {
        ast::Item::RecordDef(r) => r.name == name,
        ast::Item::ChoiceDef(c) => c.name == name,
    })
}

pub fn spin_core_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let (module_name, resource_name) = match parse_attr_args(&attr) {
        Ok(args) => args,
        Err(e) => return e.to_compile_error(),
    };

    // Parse the Rust item
    let syn_item: Item = match syn::parse2(item.clone()) {
        Ok(item) => item,
        Err(e) => return e.to_compile_error(),
    };

    // Get the .spin source
    let source = match spin_lang::builtins::get_module_source(&module_name) {
        Some(s) => s,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("unknown spin module: {module_name}"),
            )
            .to_compile_error();
        }
    };

    // Parse the .spin source
    let module = match spin_lang::parser::parse(source) {
        Ok(m) => m,
        Err(e) => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("failed to parse spin module '{module_name}': {e}"),
            )
            .to_compile_error();
        }
    };

    // Find the matching item in the .spin module
    let spin_item = match find_item_by_name(&module.items, &resource_name) {
        Some(item) => item,
        None => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                format!("item '{resource_name}' not found in spin module '{module_name}'"),
            )
            .to_compile_error();
        }
    };

    // Normalize the .spin item
    let spin_normalized = spin_lang::ast_normalize::normalize_item(spin_item);

    // Convert the Rust item to a NormalizedItem
    let rust_normalized = match &syn_item {
        Item::Struct(s) => match struct_to_normalized(s) {
            Ok(n) => n,
            Err(e) => {
                return syn::Error::new(proc_macro2::Span::call_site(), e).to_compile_error();
            }
        },
        Item::Enum(e) => match enum_to_normalized(e) {
            Ok(n) => n,
            Err(err) => {
                return syn::Error::new(proc_macro2::Span::call_site(), err).to_compile_error();
            }
        },
        _ => {
            return syn::Error::new(
                proc_macro2::Span::call_site(),
                "#[spin_core] can only be applied to structs or enums",
            )
            .to_compile_error();
        }
    };

    // Strip attributes from both sides before comparing
    let spin_stripped = strip_attributes(&spin_normalized);
    let rust_stripped = strip_attributes(&rust_normalized);

    if spin_stripped != rust_stripped {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            format!(
                "Rust type does not match spin definition for '{resource_name}' in module '{module_name}'.\n\
                 Spin definition: {spin_stripped:?}\n\
                 Rust definition: {rust_stripped:?}"
            ),
        )
        .to_compile_error();
    }

    // Pass through the original item unchanged
    item
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use spin_lang::ast_normalize::NormalizedAttribute;

    // --- Tests for parse_attr_args ---

    #[test]
    fn parse_attr_args_extracts_module_and_resource() {
        let attr = quote! { module = "spin-core-net", resource = "IpAddr" };
        let (module, resource) = parse_attr_args(&attr).unwrap();
        assert_eq!(module, "spin-core-net");
        assert_eq!(resource, "IpAddr");
    }

    #[test]
    fn parse_attr_args_errors_on_missing_module() {
        let attr = quote! { resource = "IpAddr" };
        let result = parse_attr_args(&attr);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("module"),
            "expected error about module, got: {err}"
        );
    }

    #[test]
    fn parse_attr_args_errors_on_missing_resource() {
        let attr = quote! { module = "spin-core-net" };
        let result = parse_attr_args(&attr);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("resource"),
            "expected error about resource, got: {err}"
        );
    }

    #[test]
    fn parse_attr_args_errors_on_unknown_key() {
        let attr = quote! { module = "spin-core-net", resource = "IpAddr", extra = "bad" };
        let result = parse_attr_args(&attr);
        assert!(result.is_err());
    }

    // --- Tests for rust_type_to_normalized ---

    #[test]
    fn rust_type_converts_u8() {
        let ty: Type = syn::parse2(quote! { u8 }).unwrap();
        let result = rust_type_to_normalized(&ty).unwrap();
        assert_eq!(
            result,
            NormalizedTypeExpr::Primitive(ast::PrimitiveType::U8)
        );
    }

    #[test]
    fn rust_type_converts_u16() {
        let ty: Type = syn::parse2(quote! { u16 }).unwrap();
        let result = rust_type_to_normalized(&ty).unwrap();
        assert_eq!(
            result,
            NormalizedTypeExpr::Primitive(ast::PrimitiveType::U16)
        );
    }

    #[test]
    fn rust_type_converts_u32() {
        let ty: Type = syn::parse2(quote! { u32 }).unwrap();
        let result = rust_type_to_normalized(&ty).unwrap();
        assert_eq!(
            result,
            NormalizedTypeExpr::Primitive(ast::PrimitiveType::U32)
        );
    }

    #[test]
    fn rust_type_converts_bool() {
        let ty: Type = syn::parse2(quote! { bool }).unwrap();
        let result = rust_type_to_normalized(&ty).unwrap();
        assert_eq!(
            result,
            NormalizedTypeExpr::Primitive(ast::PrimitiveType::Bool)
        );
    }

    #[test]
    fn rust_type_converts_named_type() {
        let ty: Type = syn::parse2(quote! { IpAddrV4 }).unwrap();
        let result = rust_type_to_normalized(&ty).unwrap();
        assert_eq!(result, NormalizedTypeExpr::Named("IpAddrV4".to_string()));
    }

    #[test]
    fn rust_type_converts_array() {
        let ty: Type = syn::parse2(quote! { [u8; 4] }).unwrap();
        let result = rust_type_to_normalized(&ty).unwrap();
        assert_eq!(
            result,
            NormalizedTypeExpr::Array {
                element: Box::new(NormalizedTypeExpr::Primitive(ast::PrimitiveType::U8)),
                size: 4,
            }
        );
    }

    #[test]
    fn rust_type_converts_array_size_16() {
        let ty: Type = syn::parse2(quote! { [u8; 16] }).unwrap();
        let result = rust_type_to_normalized(&ty).unwrap();
        assert_eq!(
            result,
            NormalizedTypeExpr::Array {
                element: Box::new(NormalizedTypeExpr::Primitive(ast::PrimitiveType::U8)),
                size: 16,
            }
        );
    }

    // --- Tests for struct_to_normalized ---

    #[test]
    fn struct_to_normalized_simple_record() {
        let item: syn::ItemStruct = syn::parse2(quote! {
            pub struct IpAddrV4 {
                pub octets: [u8; 4],
            }
        })
        .unwrap();
        let result = struct_to_normalized(&item).unwrap();
        assert_eq!(
            result,
            NormalizedItem::RecordDef(NormalizedRecordDef {
                name: "IpAddrV4".to_string(),
                type_params: vec![],
                attributes: vec![],
                fields: vec![NormalizedField {
                    name: "octets".to_string(),
                    ty: NormalizedTypeExpr::Array {
                        element: Box::new(NormalizedTypeExpr::Primitive(ast::PrimitiveType::U8)),
                        size: 4,
                    },
                }],
            })
        );
    }

    #[test]
    fn struct_to_normalized_multi_field_record() {
        let item: syn::ItemStruct = syn::parse2(quote! {
            pub struct SocketAddrV4 {
                pub ip: IpAddrV4,
                pub port: u16,
            }
        })
        .unwrap();
        let result = struct_to_normalized(&item).unwrap();
        assert_eq!(
            result,
            NormalizedItem::RecordDef(NormalizedRecordDef {
                name: "SocketAddrV4".to_string(),
                type_params: vec![],
                attributes: vec![],
                fields: vec![
                    NormalizedField {
                        name: "ip".to_string(),
                        ty: NormalizedTypeExpr::Named("IpAddrV4".to_string()),
                    },
                    NormalizedField {
                        name: "port".to_string(),
                        ty: NormalizedTypeExpr::Primitive(ast::PrimitiveType::U16),
                    },
                ],
            })
        );
    }

    // --- Tests for enum_to_normalized ---

    #[test]
    fn enum_to_normalized_choice() {
        let item: syn::ItemEnum = syn::parse2(quote! {
            pub enum IpAddr {
                V4(IpAddrV4),
                V6(IpAddrV6),
            }
        })
        .unwrap();
        let result = enum_to_normalized(&item).unwrap();
        assert_eq!(
            result,
            NormalizedItem::ChoiceDef(NormalizedChoiceDef {
                name: "IpAddr".to_string(),
                type_params: vec![],
                attributes: vec![],
                variants: vec![
                    NormalizedVariant {
                        name: "V4".to_string(),
                        fields: vec![NormalizedTypeExpr::Named("IpAddrV4".to_string())],
                    },
                    NormalizedVariant {
                        name: "V6".to_string(),
                        fields: vec![NormalizedTypeExpr::Named("IpAddrV6".to_string())],
                    },
                ],
            })
        );
    }

    // --- Tests for strip_attributes ---

    #[test]
    fn strip_attributes_removes_record_attributes() {
        let item = NormalizedItem::RecordDef(NormalizedRecordDef {
            name: "IpAddrV4".to_string(),
            type_params: vec![],
            attributes: vec![NormalizedAttribute {
                name: "lang-item".to_string(),
                args: None,
            }],
            fields: vec![],
        });
        let stripped = strip_attributes(&item);
        assert_eq!(
            stripped,
            NormalizedItem::RecordDef(NormalizedRecordDef {
                name: "IpAddrV4".to_string(),
                type_params: vec![],
                attributes: vec![],
                fields: vec![],
            })
        );
    }

    #[test]
    fn strip_attributes_removes_choice_attributes() {
        let item = NormalizedItem::ChoiceDef(NormalizedChoiceDef {
            name: "IpAddr".to_string(),
            type_params: vec![],
            attributes: vec![NormalizedAttribute {
                name: "lang-item".to_string(),
                args: None,
            }],
            variants: vec![],
        });
        let stripped = strip_attributes(&item);
        assert_eq!(
            stripped,
            NormalizedItem::ChoiceDef(NormalizedChoiceDef {
                name: "IpAddr".to_string(),
                type_params: vec![],
                attributes: vec![],
                variants: vec![],
            })
        );
    }

    // --- Tests for find_item_by_name ---

    #[test]
    fn find_item_by_name_finds_record() {
        let source = spin_lang::builtins::get_module_source("spin-core-net").unwrap();
        let module = spin_lang::parser::parse(source).unwrap();
        let item = find_item_by_name(&module.items, "IpAddrV4");
        assert!(item.is_some());
        assert!(matches!(item.unwrap(), ast::Item::RecordDef(r) if r.name == "IpAddrV4"));
    }

    #[test]
    fn find_item_by_name_finds_choice() {
        let source = spin_lang::builtins::get_module_source("spin-core-net").unwrap();
        let module = spin_lang::parser::parse(source).unwrap();
        let item = find_item_by_name(&module.items, "IpAddr");
        assert!(item.is_some());
        assert!(matches!(item.unwrap(), ast::Item::ChoiceDef(c) if c.name == "IpAddr"));
    }

    #[test]
    fn find_item_by_name_returns_none_for_missing() {
        let source = spin_lang::builtins::get_module_source("spin-core-net").unwrap();
        let module = spin_lang::parser::parse(source).unwrap();
        let item = find_item_by_name(&module.items, "NonExistent");
        assert!(item.is_none());
    }

    // --- Integration test: full comparison matches ---

    #[test]
    fn rust_struct_matches_spin_record_ip_addr_v4() {
        // Build the Rust side
        let item: syn::ItemStruct = syn::parse2(quote! {
            pub struct IpAddrV4 {
                pub octets: [u8; 4],
            }
        })
        .unwrap();
        let rust_normalized = struct_to_normalized(&item).unwrap();

        // Build the spin side
        let source = spin_lang::builtins::get_module_source("spin-core-net").unwrap();
        let module = spin_lang::parser::parse(source).unwrap();
        let spin_item = find_item_by_name(&module.items, "IpAddrV4").unwrap();
        let spin_normalized = spin_lang::ast_normalize::normalize_item(spin_item);

        // Strip attributes and compare
        let rust_stripped = strip_attributes(&rust_normalized);
        let spin_stripped = strip_attributes(&spin_normalized);
        assert_eq!(rust_stripped, spin_stripped);
    }

    #[test]
    fn rust_enum_matches_spin_choice_ip_addr() {
        let item: syn::ItemEnum = syn::parse2(quote! {
            pub enum IpAddr {
                V4(IpAddrV4),
                V6(IpAddrV6),
            }
        })
        .unwrap();
        let rust_normalized = enum_to_normalized(&item).unwrap();

        let source = spin_lang::builtins::get_module_source("spin-core-net").unwrap();
        let module = spin_lang::parser::parse(source).unwrap();
        let spin_item = find_item_by_name(&module.items, "IpAddr").unwrap();
        let spin_normalized = spin_lang::ast_normalize::normalize_item(spin_item);

        let rust_stripped = strip_attributes(&rust_normalized);
        let spin_stripped = strip_attributes(&spin_normalized);
        assert_eq!(rust_stripped, spin_stripped);
    }

    #[test]
    fn mismatched_struct_is_detected() {
        // Add an extra field — should NOT match
        let item: syn::ItemStruct = syn::parse2(quote! {
            pub struct IpAddrV4 {
                pub octets: [u8; 4],
                pub extra: u8,
            }
        })
        .unwrap();
        let rust_normalized = struct_to_normalized(&item).unwrap();

        let source = spin_lang::builtins::get_module_source("spin-core-net").unwrap();
        let module = spin_lang::parser::parse(source).unwrap();
        let spin_item = find_item_by_name(&module.items, "IpAddrV4").unwrap();
        let spin_normalized = spin_lang::ast_normalize::normalize_item(spin_item);

        let rust_stripped = strip_attributes(&rust_normalized);
        let spin_stripped = strip_attributes(&spin_normalized);
        assert_ne!(rust_stripped, spin_stripped);
    }

    // --- Integration: spin_core_impl produces pass-through for matching types ---

    #[test]
    fn spin_core_impl_passes_through_matching_struct() {
        let attr = quote! { module = "spin-core-net", resource = "IpAddrV4" };
        let item = quote! {
            pub struct IpAddrV4 {
                pub octets: [u8; 4],
            }
        };
        let output = super::spin_core_impl(attr, item.clone());
        // Should pass through unchanged — no compile_error
        let output_str = output.to_string();
        assert!(
            !output_str.contains("compile_error"),
            "expected no compile_error, got: {output_str}"
        );
    }

    #[test]
    fn spin_core_impl_errors_on_mismatched_struct() {
        let attr = quote! { module = "spin-core-net", resource = "IpAddrV4" };
        let item = quote! {
            pub struct IpAddrV4 {
                pub octets: [u8; 4],
                pub extra: u8,
            }
        };
        let output = super::spin_core_impl(attr, item);
        let output_str = output.to_string();
        assert!(
            output_str.contains("compile_error"),
            "expected compile_error for mismatched struct, got: {output_str}"
        );
    }

    #[test]
    fn spin_core_impl_errors_on_unknown_module() {
        let attr = quote! { module = "nonexistent", resource = "Foo" };
        let item = quote! {
            pub struct Foo {}
        };
        let output = super::spin_core_impl(attr, item);
        let output_str = output.to_string();
        assert!(
            output_str.contains("compile_error"),
            "expected compile_error for unknown module, got: {output_str}"
        );
    }

    #[test]
    fn spin_core_impl_errors_on_unknown_resource() {
        let attr = quote! { module = "spin-core-net", resource = "NonExistent" };
        let item = quote! {
            pub struct NonExistent {}
        };
        let output = super::spin_core_impl(attr, item);
        let output_str = output.to_string();
        assert!(
            output_str.contains("compile_error"),
            "expected compile_error for unknown resource, got: {output_str}"
        );
    }

    #[test]
    fn spin_core_impl_passes_through_matching_enum() {
        let attr = quote! { module = "spin-core-net", resource = "IpAddr" };
        let item = quote! {
            pub enum IpAddr {
                V4(IpAddrV4),
                V6(IpAddrV6),
            }
        };
        let output = super::spin_core_impl(attr, item.clone());
        let output_str = output.to_string();
        assert!(
            !output_str.contains("compile_error"),
            "expected no compile_error, got: {output_str}"
        );
    }
}
