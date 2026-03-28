use spin_up::ast::{Attribute, ChoiceDef, Expr, Item, PrimitiveType, RecordDef, TypeExpr, Variant};
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
        Item::ResourceDef(r) => match &r.fields[0].ty {
            TypeExpr::Generic { name, args } => {
                assert_eq!(name, "Option");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], TypeExpr::SelfPath(n) if n == "Tls"));
            }
            other => panic!("expected Generic, got {other:?}"),
        },
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

// --- AST structural tests (Task 4) ---

#[test]
fn test_ast_attribute_construction() {
    let attr = Attribute {
        name: "lang-item".to_string(),
        span: 0..11,
    };
    assert_eq!(attr.name, "lang-item");
    assert_eq!(attr.span, 0..11);
}

#[test]
fn test_ast_attribute_equality() {
    let a = Attribute {
        name: "lang-item".to_string(),
        span: 0..11,
    };
    let b = Attribute {
        name: "lang-item".to_string(),
        span: 0..11,
    };
    assert_eq!(a, b);
}

#[test]
fn test_ast_record_def_construction() {
    let record = RecordDef {
        name: "Tls".to_string(),
        attributes: vec![Attribute {
            name: "lang-item".to_string(),
            span: 0..11,
        }],
        fields: vec![],
        span: 0..20,
    };
    assert_eq!(record.name, "Tls");
    assert_eq!(record.attributes.len(), 1);
    assert!(record.fields.is_empty());
}

#[test]
fn test_ast_choice_def_construction() {
    let choice = ChoiceDef {
        name: "IpAddr".to_string(),
        attributes: vec![],
        variants: vec![
            Variant {
                name: "V4".to_string(),
                fields: vec![TypeExpr::Named("IpAddrV4".to_string())],
                span: 20..35,
            },
            Variant {
                name: "V6".to_string(),
                fields: vec![],
                span: 37..39,
            },
        ],
        span: 0..40,
    };
    assert_eq!(choice.name, "IpAddr");
    assert_eq!(choice.variants.len(), 2);
    assert_eq!(choice.variants[0].name, "V4");
    assert_eq!(choice.variants[0].fields.len(), 1);
    assert!(choice.variants[1].fields.is_empty());
}

#[test]
fn test_ast_variant_construction() {
    let variant = Variant {
        name: "Some".to_string(),
        fields: vec![TypeExpr::Primitive(PrimitiveType::U32)],
        span: 0..10,
    };
    assert_eq!(variant.name, "Some");
    assert_eq!(variant.fields.len(), 1);
}

#[test]
fn test_ast_primitive_type_equality() {
    assert_eq!(PrimitiveType::Bool, PrimitiveType::Bool);
    assert_eq!(PrimitiveType::U8, PrimitiveType::U8);
    assert_eq!(PrimitiveType::U16, PrimitiveType::U16);
    assert_eq!(PrimitiveType::U32, PrimitiveType::U32);
    assert_eq!(PrimitiveType::U64, PrimitiveType::U64);
    assert_eq!(PrimitiveType::U128, PrimitiveType::U128);
    assert_eq!(PrimitiveType::I8, PrimitiveType::I8);
    assert_eq!(PrimitiveType::I16, PrimitiveType::I16);
    assert_eq!(PrimitiveType::I32, PrimitiveType::I32);
    assert_eq!(PrimitiveType::I64, PrimitiveType::I64);
    assert_eq!(PrimitiveType::I128, PrimitiveType::I128);
    assert_eq!(PrimitiveType::F32, PrimitiveType::F32);
    assert_eq!(PrimitiveType::F64, PrimitiveType::F64);
    assert_eq!(PrimitiveType::Str, PrimitiveType::Str);
    assert_ne!(PrimitiveType::U8, PrimitiveType::I8);
}

#[test]
fn test_ast_type_expr_primitive() {
    let ty = TypeExpr::Primitive(PrimitiveType::U32);
    assert!(matches!(ty, TypeExpr::Primitive(PrimitiveType::U32)));
}

#[test]
fn test_ast_type_expr_array() {
    let ty = TypeExpr::Array {
        element: Box::new(TypeExpr::Primitive(PrimitiveType::U8)),
        size: 4,
    };
    match ty {
        TypeExpr::Array { element, size } => {
            assert!(matches!(*element, TypeExpr::Primitive(PrimitiveType::U8)));
            assert_eq!(size, 4);
        }
        other => panic!("expected Array, got {other:?}"),
    }
}

#[test]
fn test_ast_type_expr_slice() {
    let ty = TypeExpr::Slice(Box::new(TypeExpr::Primitive(PrimitiveType::U8)));
    match ty {
        TypeExpr::Slice(inner) => {
            assert!(matches!(*inner, TypeExpr::Primitive(PrimitiveType::U8)));
        }
        other => panic!("expected Slice, got {other:?}"),
    }
}

#[test]
fn test_ast_type_expr_tuple() {
    let ty = TypeExpr::Tuple(vec![
        TypeExpr::Primitive(PrimitiveType::U32),
        TypeExpr::Primitive(PrimitiveType::Str),
    ]);
    match ty {
        TypeExpr::Tuple(elems) => {
            assert_eq!(elems.len(), 2);
        }
        other => panic!("expected Tuple, got {other:?}"),
    }
}

#[test]
fn test_ast_type_expr_unit() {
    let ty = TypeExpr::Unit;
    assert!(matches!(ty, TypeExpr::Unit));
}

#[test]
fn test_ast_item_record_def_variant() {
    let item = Item::RecordDef(RecordDef {
        name: "Tls".to_string(),
        attributes: vec![],
        fields: vec![],
        span: 0..10,
    });
    assert!(matches!(item, Item::RecordDef(_)));
}

#[test]
fn test_ast_item_choice_def_variant() {
    let item = Item::ChoiceDef(ChoiceDef {
        name: "IpAddr".to_string(),
        attributes: vec![],
        variants: vec![],
        span: 0..10,
    });
    assert!(matches!(item, Item::ChoiceDef(_)));
}

#[test]
fn test_resource_def_has_attributes_field() {
    // Verify that existing parser still works and ResourceDef has attributes field
    let module = parse("resource Postgres {}").unwrap();
    match &module.items[0] {
        Item::ResourceDef(r) => {
            assert!(r.attributes.is_empty());
        }
        other => panic!("expected ResourceDef, got {other:?}"),
    }
}
