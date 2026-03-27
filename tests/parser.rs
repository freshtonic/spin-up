use spin_up::ast::{Expr, Item, TypeExpr};
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
        other => panic!("expected ResourceDef, got {other:?}"),
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
        other => panic!("expected ResourceDef, got {other:?}"),
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
        other => panic!("expected ResourceDef, got {other:?}"),
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
        other => panic!("expected ResourceDef, got {other:?}"),
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
        other => panic!("expected ResourceDef, got {other:?}"),
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
        other => panic!("expected ResourceDef, got {other:?}"),
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
        other => panic!("expected ResourceDef, got {other:?}"),
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
        other => panic!("expected ResourceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_supplies_declaration() {
    let input = r#"import postgres

supplies postgres::Postgres {
  host = "localhost",
  port = 5432,
}"#;
    let module = parse(input).unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::SuppliesDef(s) => {
            assert_eq!(s.resource_path.module, "postgres");
            assert_eq!(s.resource_path.name, "Postgres");
            assert_eq!(s.field_assignments.len(), 2);
            assert_eq!(s.field_assignments[0].name, "host");
            assert_eq!(s.field_assignments[1].name, "port");
        }
        other => panic!("expected SuppliesDef, got {other:?}"),
    }
}

#[test]
fn test_parse_supplies_string_value() {
    let input = r#"supplies postgres::Postgres {
  host = "localhost",
}"#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::SuppliesDef(s) => match &s.field_assignments[0].value {
            Expr::StringLit(v) => assert_eq!(v, "localhost"),
            other => panic!("expected StringLit, got {other:?}"),
        },
        other => panic!("expected SuppliesDef, got {other:?}"),
    }
}

#[test]
fn test_parse_supplies_number_value() {
    let input = r#"supplies postgres::Postgres {
  port = 5432,
}"#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::SuppliesDef(s) => match &s.field_assignments[0].value {
            Expr::Number(v) => assert_eq!(v, "5432"),
            other => panic!("expected Number, got {other:?}"),
        },
        other => panic!("expected SuppliesDef, got {other:?}"),
    }
}
