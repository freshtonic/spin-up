use spin_up::ast::{Item, TypeExpr};
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

#[test]
fn test_parse_empty_resource() {
    let module = parse("resource Postgres {}").unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::ResourceDef(r) => {
            assert_eq!(r.name, "Postgres");
            assert!(r.fields.is_empty());
        }
    }
}

#[test]
fn test_parse_resource_with_simple_field() {
    let input = "resource Postgres {\n  host: String,\n}";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ResourceDef(r) => {
            assert_eq!(r.fields.len(), 1);
            assert_eq!(r.fields[0].name, "host");
            assert!(matches!(&r.fields[0].ty, TypeExpr::Named(n) if n == "String"));
        }
    }
}

#[test]
fn test_parse_resource_with_qualified_type() {
    let input = "resource Postgres {\n  port: spin-core::TcpPort,\n}";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ResourceDef(r) => {
            assert_eq!(r.fields[0].name, "port");
            match &r.fields[0].ty {
                TypeExpr::Path { module, name } => {
                    assert_eq!(module, "spin-core");
                    assert_eq!(name, "TcpPort");
                }
                other => panic!("expected Path, got {other:?}"),
            }
        }
    }
}

#[test]
fn test_parse_resource_with_generic_type() {
    let input = "resource Postgres {\n  tls: Option<Self::Tls>,\n}";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ResourceDef(r) => {
            match &r.fields[0].ty {
                TypeExpr::Generic { name, args } => {
                    assert_eq!(name, "Option");
                    assert_eq!(args.len(), 1);
                    assert!(matches!(&args[0], TypeExpr::SelfPath(n) if n == "Tls"));
                }
                other => panic!("expected Generic, got {other:?}"),
            }
        }
    }
}

#[test]
fn test_parse_resource_with_self_path() {
    let input = "resource Postgres {\n  tls: Self::Tls,\n}";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ResourceDef(r) => {
            assert!(matches!(&r.fields[0].ty, TypeExpr::SelfPath(n) if n == "Tls"));
        }
    }
}

#[test]
fn test_parse_resource_trailing_comma_optional() {
    let input = "resource Postgres {\n  host: String\n}";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ResourceDef(r) => {
            assert_eq!(r.fields.len(), 1);
        }
    }
}

#[test]
fn test_parse_multiple_fields() {
    let input = r#"resource Postgres {
  version: spin-core::Semver,
  host: spin-core::String,
  port: spin-core::TcpPort,
}"#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ResourceDef(r) => {
            assert_eq!(r.fields.len(), 3);
            assert_eq!(r.fields[0].name, "version");
            assert_eq!(r.fields[1].name, "host");
            assert_eq!(r.fields[2].name, "port");
        }
    }
}

#[test]
fn test_parse_import_then_resource() {
    let input = r#"import spin-core

resource Postgres {
  port: spin-core::TcpPort,
}"#;
    let module = parse(input).unwrap();
    assert_eq!(module.imports.len(), 1);
    assert_eq!(module.imports[0].module_name, "spin-core");
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::ResourceDef(r) => {
            assert_eq!(r.name, "Postgres");
        }
    }
}
