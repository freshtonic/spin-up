use spin_up::diagnostics::{DiagnosticKind, Diagnostics};

#[test]
fn test_diagnostics_collect_multiple_errors() {
    let mut diags = Diagnostics::new();
    diags.error(
        DiagnosticKind::TypeMismatch {
            expected: "u32".to_string(),
            found: "str".to_string(),
        },
        0..10,
        "test.spin",
    );

    diags.error(
        DiagnosticKind::UnknownType {
            name: "Foo".to_string(),
        },
        20..25,
        "test.spin",
    );

    assert_eq!(diags.errors().len(), 2);
    assert!(!diags.is_ok());
}

#[test]
fn test_diagnostics_empty_is_ok() {
    let diags = Diagnostics::new();
    assert!(diags.is_ok());
    assert_eq!(diags.errors().len(), 0);
}
