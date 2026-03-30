use spin_up::analysis::let_check::check_let_redefinitions;
use spin_up::analysis::registry::TypeRegistry;
use spin_up::diagnostics::DiagnosticKind;
use spin_up::parser;

#[test]
fn redefinition_same_type_ok() {
    let source = "let port: u16 = 5432\nlet port: u16 = 8080";
    let module = parser::parse(source).unwrap();
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
    let source = "let port: u16 = 5432\nlet port: str = \"hello\"";
    let module = parser::parse(source).unwrap();
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
    let source = "let port: u16 = 5432\nlet host: str = \"localhost\"";
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(diags.is_ok());
}

#[test]
fn redefinition_inferred_type_mismatch() {
    // Use string vs bool since numeric literals infer to Unknown
    let source = "let x = \"hello\"\nlet x = true";
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(!diags.is_ok());
}

#[test]
fn redefinition_numeric_unknown_is_lenient() {
    // Numeric literals infer to Unknown, so redefinition is allowed
    // since we cannot determine the type mismatch
    let source = "let x = 42\nlet x = \"hello\"";
    let module = parser::parse(source).unwrap();
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
    let source = "let x = \"hello\"\nlet x = \"world\"";
    let module = parser::parse(source).unwrap();
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
    let source = "let x: u16 = 1\nlet x: u16 = 2\nlet x: str = \"bad\"";
    let module = parser::parse(source).unwrap();
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
    let source = "let x: str = \"hello\"\nlet x = \"world\"";
    let module = parser::parse(source).unwrap();
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
    let source = "let x: u16 = 5432\nlet x = \"hello\"";
    let module = parser::parse(source).unwrap();
    let mut registry = TypeRegistry::new();
    registry.register_module("test", &module);

    let diags = check_let_redefinitions(&registry);
    assert!(!diags.is_ok());
    assert!(matches!(
        &diags.errors()[0].kind,
        DiagnosticKind::RedefinitionTypeMismatch { name, .. } if name == "x"
    ));
}
