use spin_up::analysis::let_check::check_let_redefinitions;
use spin_up::analysis::registry::TypeRegistry;
use spin_up::diagnostics::DiagnosticKind;
use spin_up::spin;

#[test]
fn redefinition_same_type_ok() {
    let module = spin! {
        let port: u16 = 5432
        let port: u16 = 8080
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
        let port: u16 = 5432
        let port: str = "hello"
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
        let port: u16 = 5432
        let host: str = "localhost"
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(diags.is_ok());
}

#[test]
fn redefinition_inferred_type_mismatch() {
    // Use string vs bool since numeric literals infer to Unknown
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
fn redefinition_numeric_unknown_is_lenient() {
    // Numeric literals infer to Unknown, so redefinition is allowed
    // since we cannot determine the type mismatch
    let module = spin! {
        let x = 42
        let x = "hello"
    };
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(
        diags.is_ok(),
        "Unknown numeric type should be lenient: {:?}",
        diags.errors()
    );
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
        let x: u16 = 1
        let x: u16 = 2
        let x: str = "bad"
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
        let x: str = "hello"
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
        let x: u16 = 5432
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
