use spin_up::analysis::resolve::resolve_modules;
use spin_up::diagnostics::DiagnosticKind;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_resolve_single_module() {
    let tmp = TempDir::new().unwrap();
    let spin_file = tmp.path().join("main.spin");
    fs::write(&spin_file, "type Foo = x: number;").unwrap();

    let result = resolve_modules(&spin_file, &[tmp.path().to_path_buf()]);
    assert!(result.diagnostics.is_ok());
    assert!(result.registry.lookup_type("Foo").is_some());
}

#[test]
fn test_resolve_with_import() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("types.spin"), "type Bar = y: string;").unwrap();
    fs::write(
        tmp.path().join("main.spin"),
        "import types\ntype Foo = x: Bar;",
    )
    .unwrap();

    let main_file = tmp.path().join("main.spin");
    let result = resolve_modules(&main_file, &[tmp.path().to_path_buf()]);
    assert!(result.diagnostics.is_ok());
    assert!(result.registry.lookup_type("Bar").is_some());
    assert!(result.registry.lookup_type("Foo").is_some());
}

#[test]
fn test_resolve_unresolved_import() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("main.spin"), "import nonexistent").unwrap();

    let main_file = tmp.path().join("main.spin");
    let result = resolve_modules(&main_file, &[tmp.path().to_path_buf()]);
    assert!(!result.diagnostics.is_ok());
    assert!(matches!(
        &result.diagnostics.errors()[0].kind,
        DiagnosticKind::UnresolvedImport { .. }
    ));
}
