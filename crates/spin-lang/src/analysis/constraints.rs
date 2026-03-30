use crate::analysis::registry::TypeRegistry;
use crate::ast::{BinaryOp, Expr, FieldInit};
use crate::diagnostics::{DiagnosticKind, Diagnostics};

/// Check constraint expressions in all let bindings.
///
/// Walks all let-binding expressions in the registry, finds `BinaryOp`
/// expressions involving `It`, and validates that the predicates are
/// structurally valid (correct operators for the types involved).
pub fn check_constraints(registry: &TypeRegistry) -> Diagnostics {
    let mut diags = Diagnostics::new();

    for binding in registry.all_bindings().values() {
        check_expr_for_constraints(&binding.value, &binding.name, &mut diags);
    }

    diags
}

/// Check an expression for constraint sub-expressions.
///
/// Only field inits inside `NamedConstruction` are valid constraint contexts.
/// Any `It` usage elsewhere would be flagged, but for now we only walk
/// into named constructions.
fn check_expr_for_constraints(expr: &Expr, source_name: &str, diags: &mut Diagnostics) {
    match expr {
        Expr::NamedConstruction { fields, .. } => {
            for field in fields {
                if contains_it(&field.value) {
                    validate_constraint_expr(&field.value, field, source_name, diags);
                }
            }
        }
        Expr::TypeConstruction { fields, .. } => {
            for field in fields {
                if contains_it(&field.value) {
                    validate_constraint_expr(&field.value, field, source_name, diags);
                }
            }
        }
        _ => {}
    }
}

/// Returns true if the expression tree contains an `Expr::It`.
fn contains_it(expr: &Expr) -> bool {
    match expr {
        Expr::It => true,
        Expr::BinaryOp { left, right, .. } => contains_it(left) || contains_it(right),
        Expr::UnaryOp { operand, .. } => contains_it(operand),
        _ => false,
    }
}

/// Validate that a constraint expression is structurally valid.
///
/// A valid constraint expression is one of:
/// - A comparison (`>=`, `<=`, `>`, `<`, `==`, `!=`) where one side is `it`
///   and the other is a numeric literal
/// - A logical combination (`&&`, `||`) of valid constraint expressions
fn validate_constraint_expr(
    expr: &Expr,
    field: &FieldInit,
    source_name: &str,
    diags: &mut Diagnostics,
) {
    match expr {
        Expr::BinaryOp { left, op, right } => match op {
            BinaryOp::And | BinaryOp::Or => {
                // Both sides must be valid constraint expressions
                validate_constraint_expr(left, field, source_name, diags);
                validate_constraint_expr(right, field, source_name, diags);
            }
            BinaryOp::Gte
            | BinaryOp::Lte
            | BinaryOp::Gt
            | BinaryOp::Lt
            | BinaryOp::Eq
            | BinaryOp::NotEq => {
                validate_comparison_operands(left, right, field, source_name, diags);
            }
        },
        // A bare `it` inside a constraint context is valid on its own
        // (though unusual, it doesn't violate structural validity)
        Expr::It => {}
        _ => {}
    }
}

/// Validate that a comparison has `it` on one side and a compatible literal
/// on the other.
fn validate_comparison_operands(
    left: &Expr,
    right: &Expr,
    field: &FieldInit,
    source_name: &str,
    diags: &mut Diagnostics,
) {
    // Determine which side is `it` and which is the literal
    let (it_side, other_side) = if is_it(left) {
        (left, right)
    } else if is_it(right) {
        (right, left)
    } else {
        // Neither side is `it` — not a constraint we need to check
        return;
    };

    // Suppress unused variable warning; `it_side` is used for pattern matching
    let _ = it_side;

    // The other side must be a numeric literal for comparisons to be valid
    // in a constraint context within a NamedConstruction field
    if !is_numeric_literal(other_side) {
        diags.error(
            DiagnosticKind::InvalidPredicate {
                description: format!(
                    "comparison operand must be a numeric literal in constraint for field `{}`",
                    field.name
                ),
            },
            field.span.clone(),
            source_name,
        );
    }
}

/// Returns true if the expression is `Expr::It`.
fn is_it(expr: &Expr) -> bool {
    matches!(expr, Expr::It)
}

/// Returns true if the expression is a numeric literal.
fn is_numeric_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Number(_))
}
