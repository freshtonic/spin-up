use crate::analysis::infer::{
    TypeInfo, infer_expr_type, spanned_type_expr_to_type_info, types_compatible,
};
use crate::analysis::registry::TypeRegistry;
use crate::ast::LetBinding;
use crate::diagnostics::{DiagnosticKind, Diagnostics};

/// Check that redefined let-bindings do not change type.
///
/// For each binding name that appears more than once, compare each
/// subsequent definition's type against the first definition's type.
/// Types can come from explicit annotations or be inferred from the
/// value expression.
pub fn check_let_redefinitions(registry: &TypeRegistry) -> Diagnostics {
    let mut diagnostics = Diagnostics::new();

    for (name, bindings) in registry.all_bindings_by_name() {
        if bindings.len() < 2 {
            continue;
        }

        let first = &bindings[0];
        let first_type = resolve_binding_type(first, registry);

        for subsequent in &bindings[1..] {
            let subsequent_type = resolve_binding_type(subsequent, registry);

            if !types_compatible(&first_type, &subsequent_type) {
                diagnostics.error(
                    DiagnosticKind::RedefinitionTypeMismatch {
                        name: name.clone(),
                        expected: first_type.to_string(),
                        found: subsequent_type.to_string(),
                    },
                    subsequent.span.clone(),
                    "test",
                );
            }
        }
    }

    diagnostics
}

/// Determine the type of a binding, preferring the explicit annotation
/// and falling back to inference from the value expression.
fn resolve_binding_type(binding: &LetBinding, registry: &TypeRegistry) -> TypeInfo {
    if let Some(ty) = &binding.ty {
        spanned_type_expr_to_type_info(ty)
    } else {
        infer_expr_type(&binding.value, "", registry)
    }
}
