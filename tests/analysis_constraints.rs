use spin_up::analysis::constraints::check_constraints;
use spin_up::analysis::registry::TypeRegistry;
use spin_up::diagnostics::DiagnosticKind;
use spin_up::parser;

#[test]
fn test_valid_constraint_expression() {
    let source = r#"let x = SemVer(major: it >= 15 && it < 17)"#;
    let module = parser::parse(source).unwrap();
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
    let source = r#"let x = SemVer(major: 15)"#;
    let module = parser::parse(source).unwrap();
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
    let source = r#"let x = Foo(count: it >= "hello")"#;
    let module = parser::parse(source).unwrap();
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
    let source = r#"let x = Foo(level: it == 1 || it == 2)"#;
    let module = parser::parse(source).unwrap();
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
    let source = r#"let x = Foo(val: it >= 1 && it <= 10 || it == 99)"#;
    let module = parser::parse(source).unwrap();
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
    let source = r#"let x = Bar(name: it >= "abc")"#;
    let module = parser::parse(source).unwrap();
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
    let source = r#"import postgres"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(diags.is_ok());
}

#[test]
fn test_binding_without_named_construction_no_errors() {
    let source = r#"let x = 42"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(diags.is_ok());
}

#[test]
fn test_it_with_bool_literal_is_invalid() {
    // Comparing it with a boolean using >= is not valid for constraint expressions
    let source = r#"let x = Foo(flag: it >= true)"#;
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_constraints(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::InvalidPredicate { .. }
    ));
}
