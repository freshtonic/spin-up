use crate::analysis::registry::TypeRegistry;
use crate::ast::{BinaryOp, Expr, FieldInit};
use crate::diagnostics::{DiagnosticKind, Diagnostics};

/// Check constraint expressions in all let bindings.
///
/// Walks all let-binding expressions in the registry, finds `BinaryOp`
/// expressions involving `It`, and validates that the predicates are
/// structurally valid (correct operators for the types involved).
/// Also checks that compound constraints are satisfiable (e.g.,
/// `it >= 100 && it < 5` is unsatisfiable and will be flagged).
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
                    check_constraint_satisfiability(&field.value, field, source_name, diags);
                }
            }
        }
        Expr::TypeConstruction { fields, .. } => {
            for field in fields {
                if contains_it(&field.value) {
                    validate_constraint_expr(&field.value, field, source_name, diags);
                    check_constraint_satisfiability(&field.value, field, source_name, diags);
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
            BinaryOp::RegexMatch
            | BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div => {
                // Arithmetic and regex ops are valid in constraint position
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

// --- Constraint satisfiability checking ---

/// A constant value extracted from a constraint expression.
#[derive(Debug, Clone, PartialEq)]
enum ConstValue {
    Integer(i128),
    Float(f64),
}

/// Try to parse a numeric literal string into a `ConstValue`.
fn parse_numeric(s: &str) -> Option<ConstValue> {
    // Try integer first (including hex, octal, binary)
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return i128::from_str_radix(hex, 16).ok().map(ConstValue::Integer);
    }
    if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        return i128::from_str_radix(oct, 8).ok().map(ConstValue::Integer);
    }
    if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        return i128::from_str_radix(bin, 2).ok().map(ConstValue::Integer);
    }
    if s.contains('.') {
        return s.parse::<f64>().ok().map(ConstValue::Float);
    }
    s.parse::<i128>().ok().map(ConstValue::Integer)
}

/// Extract a constant value from an expression.
fn eval_constant(expr: &Expr) -> Option<ConstValue> {
    match expr {
        Expr::Number(s) => parse_numeric(s),
        _ => None,
    }
}

/// Represents the collected bounds from a constraint expression.
/// Used to check satisfiability of `&&`-combined comparisons.
#[derive(Debug, Default)]
struct BoundSet {
    /// Lower bound: the value and whether it is inclusive (>=) or exclusive (>).
    lower: Option<(ConstValue, bool)>,
    /// Upper bound: the value and whether it is inclusive (<=) or exclusive (<).
    upper: Option<(ConstValue, bool)>,
    /// Equality constraints: `it == value`.
    equalities: Vec<ConstValue>,
    /// Not-equal constraints: `it != value`.
    not_equals: Vec<ConstValue>,
}

/// Collect all bounds from an `&&`-chained constraint expression.
/// Returns `None` if the expression contains `||` (we handle disjunctions separately)
/// or if it contains non-constant operands.
fn collect_bounds(expr: &Expr) -> Option<BoundSet> {
    let mut bounds = BoundSet::default();
    if collect_bounds_inner(expr, &mut bounds) {
        Some(bounds)
    } else {
        None
    }
}

/// Recursively collect bounds. Returns false if the expression cannot be analyzed
/// (e.g., contains `||` or non-constant operands).
fn collect_bounds_inner(expr: &Expr, bounds: &mut BoundSet) -> bool {
    match expr {
        Expr::BinaryOp { left, op, right } => match op {
            BinaryOp::And => {
                collect_bounds_inner(left, bounds) && collect_bounds_inner(right, bounds)
            }
            BinaryOp::Or => {
                // Disjunctions are handled at a higher level
                false
            }
            BinaryOp::Gte
            | BinaryOp::Lte
            | BinaryOp::Gt
            | BinaryOp::Lt
            | BinaryOp::Eq
            | BinaryOp::NotEq => {
                // Determine which side is `it` and which is the literal
                let (normalized_op, value) = if is_it(left) {
                    // it <op> value
                    let Some(v) = eval_constant(right) else {
                        return false;
                    };
                    (op.clone(), v)
                } else if is_it(right) {
                    // value <op> it → flip the operator
                    let Some(v) = eval_constant(left) else {
                        return false;
                    };
                    let Some(flipped) = flip_comparison(op) else {
                        return false;
                    };
                    (flipped, v)
                } else {
                    return false;
                };

                add_bound(bounds, &normalized_op, value);
                true
            }
            BinaryOp::RegexMatch
            | BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div => false,
        },
        Expr::It => true,
        _ => false,
    }
}

/// Flip a comparison operator (for `value <op> it` → `it <flipped> value`).
fn flip_comparison(op: &BinaryOp) -> Option<BinaryOp> {
    Some(match op {
        BinaryOp::Gte => BinaryOp::Lte,
        BinaryOp::Lte => BinaryOp::Gte,
        BinaryOp::Gt => BinaryOp::Lt,
        BinaryOp::Lt => BinaryOp::Gt,
        BinaryOp::Eq => BinaryOp::Eq,
        BinaryOp::NotEq => BinaryOp::NotEq,
        _ => return None,
    })
}

/// Add a bound to the set based on the operator.
fn add_bound(bounds: &mut BoundSet, op: &BinaryOp, value: ConstValue) {
    match op {
        BinaryOp::Gte => {
            // it >= value → lower bound (inclusive)
            update_lower_bound(bounds, value, true);
        }
        BinaryOp::Gt => {
            // it > value → lower bound (exclusive)
            update_lower_bound(bounds, value, false);
        }
        BinaryOp::Lte => {
            // it <= value → upper bound (inclusive)
            update_upper_bound(bounds, value, true);
        }
        BinaryOp::Lt => {
            // it < value → upper bound (exclusive)
            update_upper_bound(bounds, value, false);
        }
        BinaryOp::Eq => {
            bounds.equalities.push(value);
        }
        BinaryOp::NotEq => {
            bounds.not_equals.push(value);
        }
        _ => {}
    }
}

/// Update the lower bound, keeping the tightest (highest) one.
fn update_lower_bound(bounds: &mut BoundSet, value: ConstValue, inclusive: bool) {
    match &bounds.lower {
        Some((existing, existing_inclusive)) => {
            if const_value_gt(&value, existing)
                || (const_value_eq(&value, existing) && !inclusive && *existing_inclusive)
            {
                bounds.lower = Some((value, inclusive));
            }
        }
        None => {
            bounds.lower = Some((value, inclusive));
        }
    }
}

/// Update the upper bound, keeping the tightest (lowest) one.
fn update_upper_bound(bounds: &mut BoundSet, value: ConstValue, inclusive: bool) {
    match &bounds.upper {
        Some((existing, existing_inclusive)) => {
            if const_value_lt(&value, existing)
                || (const_value_eq(&value, existing) && !inclusive && *existing_inclusive)
            {
                bounds.upper = Some((value, inclusive));
            }
        }
        None => {
            bounds.upper = Some((value, inclusive));
        }
    }
}

fn const_value_as_f64(v: &ConstValue) -> f64 {
    match v {
        ConstValue::Integer(i) => *i as f64,
        ConstValue::Float(f) => *f,
    }
}

fn const_value_gt(a: &ConstValue, b: &ConstValue) -> bool {
    const_value_as_f64(a) > const_value_as_f64(b)
}

fn const_value_lt(a: &ConstValue, b: &ConstValue) -> bool {
    const_value_as_f64(a) < const_value_as_f64(b)
}

fn const_value_eq(a: &ConstValue, b: &ConstValue) -> bool {
    const_value_as_f64(a) == const_value_as_f64(b)
}

fn const_value_gte(a: &ConstValue, b: &ConstValue) -> bool {
    const_value_as_f64(a) >= const_value_as_f64(b)
}

fn const_value_lte(a: &ConstValue, b: &ConstValue) -> bool {
    const_value_as_f64(a) <= const_value_as_f64(b)
}

/// Check if a bound set is satisfiable.
/// Returns a description of why it is unsatisfiable, or `None` if satisfiable.
fn check_satisfiability(bounds: &BoundSet) -> Option<String> {
    // Check equality constraints are mutually consistent
    if bounds.equalities.len() > 1 {
        let first = &bounds.equalities[0];
        for eq in &bounds.equalities[1..] {
            if !const_value_eq(first, eq) {
                return Some(format!(
                    "contradictory equality constraints: it == {} and it == {}",
                    format_const_value(first),
                    format_const_value(eq)
                ));
            }
        }
    }

    // If we have an equality, check it against range bounds
    if let Some(eq_val) = bounds.equalities.first() {
        if let Some((lower, lower_inclusive)) = &bounds.lower {
            let ok = if *lower_inclusive {
                const_value_gte(eq_val, lower)
            } else {
                const_value_gt(eq_val, lower)
            };
            if !ok {
                return Some(format!(
                    "equality it == {} violates lower bound it {} {}",
                    format_const_value(eq_val),
                    if *lower_inclusive { ">=" } else { ">" },
                    format_const_value(lower)
                ));
            }
        }
        if let Some((upper, upper_inclusive)) = &bounds.upper {
            let ok = if *upper_inclusive {
                const_value_lte(eq_val, upper)
            } else {
                const_value_lt(eq_val, upper)
            };
            if !ok {
                return Some(format!(
                    "equality it == {} violates upper bound it {} {}",
                    format_const_value(eq_val),
                    if *upper_inclusive { "<=" } else { "<" },
                    format_const_value(upper)
                ));
            }
        }
    }

    // Check range bounds are satisfiable
    if let (Some((lower, lower_inclusive)), Some((upper, upper_inclusive))) =
        (&bounds.lower, &bounds.upper)
    {
        if const_value_gt(lower, upper) {
            return Some(format!(
                "unsatisfiable range: it {} {} and it {} {}",
                if *lower_inclusive { ">=" } else { ">" },
                format_const_value(lower),
                if *upper_inclusive { "<=" } else { "<" },
                format_const_value(upper)
            ));
        }
        if const_value_eq(lower, upper) {
            // Equal bounds: only satisfiable if both inclusive
            if !lower_inclusive || !upper_inclusive {
                return Some(format!(
                    "unsatisfiable range: it {} {} and it {} {}",
                    if *lower_inclusive { ">=" } else { ">" },
                    format_const_value(lower),
                    if *upper_inclusive { "<=" } else { "<" },
                    format_const_value(upper)
                ));
            }
        }
    }

    None
}

fn format_const_value(v: &ConstValue) -> String {
    match v {
        ConstValue::Integer(i) => i.to_string(),
        ConstValue::Float(f) => f.to_string(),
    }
}

/// Check whether a constraint expression is satisfiable and emit a
/// `ConstraintViolation` if it is not.
fn check_constraint_satisfiability(
    expr: &Expr,
    field: &FieldInit,
    source_name: &str,
    diags: &mut Diagnostics,
) {
    // For `||` (disjunction) at the top level, at least one branch must be satisfiable.
    // For `&&` (conjunction), we collect all bounds and check satisfiability.
    match expr {
        Expr::BinaryOp {
            left,
            op: BinaryOp::Or,
            right,
        } => {
            // For an OR, we check each branch independently.
            // If either branch is satisfiable, the whole thing is satisfiable.
            let left_ok = is_branch_satisfiable(left);
            let right_ok = is_branch_satisfiable(right);
            if !left_ok && !right_ok {
                diags.error(
                    DiagnosticKind::ConstraintViolation {
                        description: format!(
                            "no branch of disjunction is satisfiable for field `{}`",
                            field.name
                        ),
                    },
                    field.span.clone(),
                    source_name,
                );
            }
        }
        _ => {
            // Try to collect bounds from the entire expression
            if let Some(bounds) = collect_bounds(expr)
                && let Some(reason) = check_satisfiability(&bounds)
            {
                diags.error(
                    DiagnosticKind::ConstraintViolation {
                        description: format!(
                            "constraint on field `{}` is unsatisfiable: {}",
                            field.name, reason
                        ),
                    },
                    field.span.clone(),
                    source_name,
                );
            }
        }
    }
}

/// Check if an expression branch is satisfiable. Returns true if satisfiable
/// or if we cannot determine satisfiability (giving the benefit of the doubt).
fn is_branch_satisfiable(expr: &Expr) -> bool {
    match collect_bounds(expr) {
        Some(bounds) => check_satisfiability(&bounds).is_none(),
        None => true, // Cannot analyze → assume satisfiable
    }
}
