use std::collections::HashMap;

use spin_up::diagnostics::{DiagnosticKind, Diagnostics, format_diagnostic};

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

// --- format_diagnostic tests ---

#[test]
fn test_format_diagnostic_type_mismatch() {
    let kind = DiagnosticKind::TypeMismatch {
        expected: "u32".to_string(),
        found: "str".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "type mismatch");
    assert_eq!(label, "expected u32, found str");
    assert!(help.is_none());
}

#[test]
fn test_format_diagnostic_unknown_type() {
    let kind = DiagnosticKind::UnknownType {
        name: "Foo".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "unknown type `Foo`");
    assert_eq!(label, "not found in scope");
    assert!(help.is_some());
    assert_eq!(help.unwrap(), "did you forget an import?");
}

#[test]
fn test_format_diagnostic_unknown_interface() {
    let kind = DiagnosticKind::UnknownInterface {
        name: "Bar".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "unknown interface `Bar`");
    assert_eq!(label, "not found in scope");
    assert!(help.is_some());
}

#[test]
fn test_format_diagnostic_missing_field() {
    let kind = DiagnosticKind::MissingField {
        field: "port".to_string(),
        interface: "HttpServer".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "missing field `port` in impl for `HttpServer`");
    assert_eq!(label, "required by interface");
    assert!(help.is_some());
    assert_eq!(
        help.unwrap(),
        "add a mapping for this field, or add #[default(...)] to the interface"
    );
}

#[test]
fn test_format_diagnostic_cyclic_dependency() {
    let kind = DiagnosticKind::CyclicDependency {
        cycle: vec!["a".to_string(), "b".to_string(), "c".to_string()],
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "cyclic dependency detected");
    assert_eq!(label, "cycle involves: a, b, c");
    assert!(help.is_none());
}

#[test]
fn test_format_diagnostic_unresolved_import() {
    let kind = DiagnosticKind::UnresolvedImport {
        module: "missing_mod".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "unresolved import `missing_mod`");
    assert_eq!(label, "module not found");
    assert!(help.is_some());
    assert_eq!(help.unwrap(), "check your SPIN_PATH");
}

#[test]
fn test_format_diagnostic_circular_import() {
    let kind = DiagnosticKind::CircularImport {
        chain: vec!["a".to_string(), "b".to_string()],
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "circular import detected");
    assert_eq!(label, "import chain: a, b");
    assert!(help.is_none());
}

#[test]
fn test_format_diagnostic_duplicate_field() {
    let kind = DiagnosticKind::DuplicateField {
        field: "name".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "duplicate field `name`");
    assert_eq!(label, "already defined");
    assert!(help.is_none());
}

#[test]
fn test_format_diagnostic_redefinition_type_mismatch() {
    let kind = DiagnosticKind::RedefinitionTypeMismatch {
        name: "x".to_string(),
        expected: "u32".to_string(),
        found: "str".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "type mismatch in redefinition of `x`");
    assert_eq!(label, "expected u32, found str");
    assert!(help.is_none());
}

#[test]
fn test_format_diagnostic_invalid_delegate() {
    let kind = DiagnosticKind::InvalidDelegate {
        reason: "not delegatable".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "invalid delegate");
    assert_eq!(label, "not delegatable");
    assert!(help.is_none());
}

#[test]
fn test_format_diagnostic_invalid_as_interface() {
    let kind = DiagnosticKind::InvalidAsInterface {
        type_name: "Foo".to_string(),
        interface: "Bar".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "`Foo` does not implement interface `Bar`");
    assert_eq!(label, "interface not satisfied");
    assert!(help.is_none());
}

#[test]
fn test_format_diagnostic_constraint_violation() {
    let kind = DiagnosticKind::ConstraintViolation {
        description: "port must be > 0".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "constraint violation");
    assert_eq!(label, "port must be > 0");
    assert!(help.is_none());
}

#[test]
fn test_format_diagnostic_invalid_predicate() {
    let kind = DiagnosticKind::InvalidPredicate {
        description: "unknown predicate".to_string(),
    };
    let (message, label, help) = format_diagnostic(&kind);
    assert_eq!(message, "invalid predicate");
    assert_eq!(label, "unknown predicate");
    assert!(help.is_none());
}

// --- into_reports tests ---

#[test]
fn test_into_reports_converts_diagnostics_to_spin_errors() {
    let mut diags = Diagnostics::new();
    diags.error(
        DiagnosticKind::UnknownType {
            name: "Foo".to_string(),
        },
        5..8,
        "test.spin",
    );

    let mut sources = HashMap::new();
    sources.insert("test.spin".to_string(), "type Foo = x: u32;".to_string());

    let reports = diags.into_reports(&sources);
    assert_eq!(reports.len(), 1);

    let report = &reports[0];
    let rendered = format!("{:?}", miette::Report::new(report.clone()));
    assert!(rendered.contains("unknown type `Foo`"));
    assert!(rendered.contains("test.spin"));
}

#[test]
fn test_into_reports_handles_missing_source() {
    let mut diags = Diagnostics::new();
    diags.error(
        DiagnosticKind::UnresolvedImport {
            module: "missing".to_string(),
        },
        0..6,
        "nonexistent.spin",
    );

    let sources = HashMap::new();

    let reports = diags.into_reports(&sources);
    assert_eq!(reports.len(), 1);
    // Should not panic even without source text
}

#[test]
fn test_into_reports_multiple_diagnostics() {
    let mut diags = Diagnostics::new();
    diags.error(
        DiagnosticKind::UnknownType {
            name: "A".to_string(),
        },
        0..1,
        "a.spin",
    );
    diags.error(
        DiagnosticKind::UnknownType {
            name: "B".to_string(),
        },
        0..1,
        "b.spin",
    );

    let mut sources = HashMap::new();
    sources.insert("a.spin".to_string(), "A".to_string());
    sources.insert("b.spin".to_string(), "B".to_string());

    let reports = diags.into_reports(&sources);
    assert_eq!(reports.len(), 2);
}
