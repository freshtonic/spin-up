use spin_up::analysis::constraints::check_constraints;
use spin_up::analysis::registry::TypeRegistry;
use spin_up::diagnostics::DiagnosticKind;
use spin_up::spin;

#[test]
fn test_valid_constraint_expression() {
    let module = spin! { let x = SemVer(major: it >= 15 && it < 17) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(
        diags.is_ok(),
        "valid constraint should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn test_simple_constraint_no_it() {
    let module = spin! { let x = SemVer(major: 15) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(
        diags.is_ok(),
        "no constraints should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn test_constraint_with_incompatible_op() {
    // Using a string literal with a comparison where numeric is expected
    let module = spin! { let x = Foo(count: it >= "hello") };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::InvalidPredicate { .. }
    ));
}

#[test]
fn test_constraint_or_operator() {
    let module = spin! { let x = Foo(level: it == 1 || it == 2) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(
        diags.is_ok(),
        "valid || constraint should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn test_constraint_nested_logical_ops() {
    let module = spin! { let x = Foo(val: it >= 1 && it <= 10 || it == 99) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(
        diags.is_ok(),
        "nested logical ops should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn test_constraint_string_comparison_with_string_is_invalid() {
    // Comparing it (in numeric context from named construction) with a string is invalid
    let module = spin! { let x = Bar(name: it >= "abc") };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::InvalidPredicate { .. }
    ));
}

#[test]
fn test_no_bindings_no_errors() {
    let module = spin! { import postgres };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(diags.is_ok());
}

#[test]
fn test_binding_without_named_construction_no_errors() {
    let module = spin! { let x = 42 };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(diags.is_ok());
}

#[test]
fn test_it_with_bool_literal_is_invalid() {
    // Comparing it with a boolean using >= is not valid for constraint expressions
    let module = spin! { let x = Foo(flag: it >= true) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::InvalidPredicate { .. }
    ));
}

// --- Value resolution / satisfiability tests ---

#[test]
fn test_eval_satisfiable_constraint() {
    let module = spin! { let x = Foo(count: it >= 5 && it < 100) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(
        diags.is_ok(),
        "satisfiable constraint should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn test_eval_unsatisfiable_constraint() {
    let module = spin! { let x = Foo(count: it >= 100 && it < 5) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::ConstraintViolation { .. }
    ));
}

#[test]
fn test_eval_equality_constraint() {
    let module = spin! { let x = Foo(count: it == 42) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(diags.is_ok());
}

#[test]
fn test_eval_contradictory_equality() {
    let module = spin! { let x = Foo(count: it == 5 && it == 10) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::ConstraintViolation { .. }
    ));
}

#[test]
fn test_eval_equality_with_compatible_range() {
    // it == 42 && it >= 10 && it < 100 -- 42 satisfies all bounds
    let module = spin! { let x = Foo(count: it == 42 && it >= 10 && it < 100) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(
        diags.is_ok(),
        "equality within range should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn test_eval_equality_outside_range() {
    // it == 200 && it < 100 -- 200 does not satisfy it < 100
    let module = spin! { let x = Foo(count: it == 200 && it < 100) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::ConstraintViolation { .. }
    ));
}

#[test]
fn test_eval_or_constraint_satisfiable() {
    // it == 5 || it == 10 -- at least one branch is satisfiable
    let module = spin! { let x = Foo(count: it == 5 || it == 10) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(
        diags.is_ok(),
        "or constraint should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn test_eval_boundary_equal_lower_inclusive() {
    // it >= 5 && it <= 5 -- exactly 5 satisfies both
    let module = spin! { let x = Foo(count: it >= 5 && it <= 5) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(
        diags.is_ok(),
        "exact boundary constraint should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn test_eval_exclusive_boundary_unsatisfiable() {
    // it > 5 && it < 5 -- no integer satisfies both
    let module = spin! { let x = Foo(count: it > 5 && it < 5) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::ConstraintViolation { .. }
    ));
}

#[test]
fn test_eval_not_equal_with_range() {
    // it != 50 && it >= 1 && it <= 100 -- satisfiable (e.g., 1..50, 51..100)
    let module = spin! { let x = Foo(count: it != 50 && it >= 1 && it <= 100) };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(
        diags.is_ok(),
        "not-equal within range should pass: {:?}",
        diags.errors()
    );
}
