use crate::analysis::registry::TypeRegistry;
use crate::ast::TypeExpr;
use crate::diagnostics::{DiagnosticKind, Diagnostics};

/// Check impl completeness: every required interface field must be mapped
/// (or have a default / be `Option<T>`).
///
/// Returns collected diagnostics (non-fatal).
pub fn unify(registry: &TypeRegistry) -> Diagnostics {
    let mut diags = Diagnostics::new();

    for impl_block in registry.all_impls() {
        // Look up the interface
        let Some(interface) = registry.lookup_interface(&impl_block.interface_name) else {
            diags.error(
                DiagnosticKind::UnknownInterface {
                    name: impl_block.interface_name.clone(),
                },
                impl_block.span.clone(),
                "unify",
            );
            continue;
        };

        // Look up the implementing type
        if registry.lookup_type(&impl_block.type_name).is_none() {
            diags.error(
                DiagnosticKind::UnknownType {
                    name: impl_block.type_name.clone(),
                },
                impl_block.span.clone(),
                "unify",
            );
            continue;
        }

        // Check each interface field is covered
        for field in &interface.fields {
            let has_mapping = impl_block.mappings.iter().any(|m| m.name == field.name);
            if has_mapping {
                continue;
            }

            let has_default = field.attributes.iter().any(|a| a.name == "default");
            if has_default {
                continue;
            }

            let is_option = matches!(&field.ty, TypeExpr::Generic { name, .. } if name == "Option");
            if is_option {
                continue;
            }

            diags.error(
                DiagnosticKind::MissingField {
                    field: field.name.clone(),
                    interface: impl_block.interface_name.clone(),
                },
                impl_block.span.clone(),
                "unify",
            );
        }
    }

    diags
}
