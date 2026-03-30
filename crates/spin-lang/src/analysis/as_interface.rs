use crate::analysis::registry::TypeRegistry;
use crate::ast::Expr;
use crate::diagnostics::{DiagnosticKind, Diagnostics};

/// Validate all `<as Interface> { ... }` blocks in type constructions.
///
/// For every let binding whose value is a `TypeConstruction` containing
/// `as_interfaces` entries, this check verifies:
/// 1. The referenced interface exists in the registry.
/// 2. The constructing type has an `impl` block for that interface.
pub fn check_as_interfaces(registry: &TypeRegistry) -> Diagnostics {
    let mut diags = Diagnostics::new();

    for binding in registry.all_bindings().values() {
        check_expr(registry, &binding.value, &mut diags);
    }

    diags
}

fn check_expr(registry: &TypeRegistry, expr: &Expr, diags: &mut Diagnostics) {
    let Expr::TypeConstruction {
        type_name,
        as_interfaces,
        fields,
    } = expr
    else {
        return;
    };

    // Recurse into field value expressions
    for field in fields {
        check_expr(registry, &field.value, diags);
    }

    for block in as_interfaces {
        // Check the interface exists
        if registry.lookup_interface(&block.interface_name).is_none() {
            diags.error(
                DiagnosticKind::UnknownInterface {
                    name: block.interface_name.clone(),
                },
                block.span.clone(),
                "check",
            );
            continue;
        }

        // Check the constructing type implements the interface
        let has_impl = registry
            .lookup_impls_for_type(type_name)
            .iter()
            .any(|imp| imp.interface_name == block.interface_name);

        if !has_impl {
            diags.error(
                DiagnosticKind::InvalidAsInterface {
                    type_name: type_name.clone(),
                    interface: block.interface_name.clone(),
                },
                block.span.clone(),
                "check",
            );
        }
    }
}
