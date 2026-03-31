use spin_up::analysis::let_check::check_let_redefinitions;
use spin_up::analysis::registry::TypeRegistry;
use spin_up::diagnostics::DiagnosticKind;
use spin_up::spin;

#[test]
fn redefinition_same_type_ok() {
    let module = spin! {
        let port: number = 5432
        let port: number = 8080
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(
        diags.is_ok(),
        "same type redefinition should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn redefinition_type_mismatch() {
    let module = spin! {
        let port: number = 5432
        let port: string = "hello"
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::RedefinitionTypeMismatch { name, .. } if name == "port"
    ));
}

#[test]
fn no_redefinition_passes() {
    let module = spin! {
        let port: number = 5432
        let host: string = "localhost"
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(diags.is_ok());
}

#[test]
fn redefinition_inferred_type_mismatch() {
    // String vs bool is a type mismatch
    let module = spin! {
        let x = "hello"
        let x = true
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(!diags.is_ok());
}

#[test]
fn redefinition_numeric_to_string_is_error() {
    // Numeric literals infer to Number, so redefinition to string is a type mismatch
    let module = spin! {
        let x = 42
        let x = "hello"
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(!diags.is_ok(), "number to string redefinition should be an error");
}

#[test]
fn redefinition_inferred_same_type_ok() {
    let module = spin! {
        let x = "hello"
        let x = "world"
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(
        diags.is_ok(),
        "same inferred type redefinition should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn multiple_redefinitions_first_type_wins() {
    let module = spin! {
        let x: number = 1
        let x: number = 2
        let x: string = "bad"
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::RedefinitionTypeMismatch { name, .. } if name == "x"
    ));
}

#[test]
fn redefinition_explicit_vs_inferred_compatible() {
    let module = spin! {
        let x: string = "hello"
        let x = "world"
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(
        diags.is_ok(),
        "explicit str then inferred str should pass: {:?}",
        diags.errors()
    );
}

#[test]
fn redefinition_explicit_vs_inferred_mismatch() {
    let module = spin! {
        let x: number = 5432
        let x = "hello"
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::RedefinitionTypeMismatch { name, .. } if name == "x"
    ));
}
