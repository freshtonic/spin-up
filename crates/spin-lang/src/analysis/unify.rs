use crate::analysis::infer::{
    TypeInfo, infer_expr_type, spanned_type_expr_to_type_info, types_compatible,
};
use crate::analysis::registry::TypeRegistry;
use crate::ast::TypeExpr;
use crate::diagnostics::{DiagnosticKind, Diagnostics};

/// Check impl completeness: every required interface field must be mapped
/// (or have a default / be `Option<T>`).
///
/// Also checks that mapped expression types are compatible with the
/// interface field's declared type.
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
            let mapping = impl_block.mappings.iter().find(|m| m.name == field.name);

            if let Some(mapping) = mapping {
                // Field is mapped -- check type compatibility
                let expected = spanned_type_expr_to_type_info(&field.ty);
                let actual = infer_expr_type(&mapping.value, &impl_block.type_name, registry);

                if !matches!(actual, TypeInfo::Unknown) && !types_compatible(&expected, &actual) {
                    diags.error(
                        DiagnosticKind::TypeMismatch {
                            expected: expected.to_string(),
                            found: actual.to_string(),
                        },
                        mapping.span.clone(),
                        "unify",
                    );
                }
                continue;
            }

            let has_default = field.attributes.iter().any(|a| a.name == "default");
            if has_default {
                continue;
            }

            let is_option =
                matches!(&field.ty.kind, TypeExpr::Generic { name, .. } if name == "Option");
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
