use spin_up::parser::parse;

#[test]
fn test_parse_single_import() {
    let module = parse("import postgres").unwrap();
    assert_eq!(module.imports.len(), 1);
    assert_eq!(module.imports[0].module_name, "postgres");
}

#[test]
fn test_parse_multiple_imports() {
    let module = parse("import postgres\nimport redis").unwrap();
    assert_eq!(module.imports.len(), 2);
    assert_eq!(module.imports[0].module_name, "postgres");
    assert_eq!(module.imports[1].module_name, "redis");
}

#[test]
fn test_parse_import_with_hyphen() {
    let module = parse("import spin-core").unwrap();
    assert_eq!(module.imports[0].module_name, "spin-core");
}

#[test]
fn test_parse_empty_input() {
    let module = parse("").unwrap();
    assert!(module.imports.is_empty());
    assert!(module.items.is_empty());
}

#[test]
fn test_parse_import_missing_name() {
    let result = parse("import");
    assert!(result.is_err());
}
