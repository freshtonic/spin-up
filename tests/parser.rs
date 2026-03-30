use spin_up::ast::{Attribute, ChoiceDef, Item, PrimitiveType, RecordDef, TypeExpr, Variant};
use spin_up::parser::parse;

// Note: The `resource` keyword has been removed. Resource definitions now use
// the `type` keyword with record (product type) syntax: `type Foo = field: Type;`

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
fn test_parse_empty_resource_as_record() {
    let module = parse("type Postgres;").unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.name, "Postgres");
            assert!(r.fields.is_empty());
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_resource_with_simple_field() {
    let input = "type Postgres = host: String;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.fields.len(), 1);
            assert_eq!(r.fields[0].name, "host");
            assert!(matches!(&r.fields[0].ty, TypeExpr::Named(n) if n == "String"));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_resource_with_qualified_type() {
    let input = "type Postgres = port: spin-core::TcpPort;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.fields[0].name, "port");
            match &r.fields[0].ty {
                TypeExpr::Path { module, name } => {
                    assert_eq!(module, "spin-core");
                    assert_eq!(name, "TcpPort");
                }
                other => panic!("expected Path, got {other:?}"),
            }
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_resource_with_generic_type() {
    let input = "type Postgres = tls: Option<Self::Tls>;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => match &r.fields[0].ty {
            TypeExpr::Generic { name, args } => {
                assert_eq!(name, "Option");
                assert_eq!(args.len(), 1);
                assert!(matches!(&args[0], TypeExpr::SelfPath(n) if n == "Tls"));
            }
            other => panic!("expected Generic, got {other:?}"),
        },
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_resource_with_self_path() {
    let input = "type Postgres = tls: Self::Tls;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert!(matches!(&r.fields[0].ty, TypeExpr::SelfPath(n) if n == "Tls"));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_resource_trailing_comma_optional() {
    let input = "type Postgres = host: String;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.fields.len(), 1);
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_multiple_fields() {
    let input = "type Postgres = version: spin-core::Semver, host: spin-core::String, port: spin-core::TcpPort;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.fields.len(), 3);
            assert_eq!(r.fields[0].name, "version");
            assert_eq!(r.fields[1].name, "host");
            assert_eq!(r.fields[2].name, "port");
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_import_then_resource() {
    let input = "import spin-core\n\ntype Postgres = port: spin-core::TcpPort;";
    let module = parse(input).unwrap();
    assert_eq!(module.imports.len(), 1);
    assert_eq!(module.imports[0].module_name, "spin-core");
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.name, "Postgres");
        }
        other => panic!("expected RecordDef, got {other:?}"),
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
        type_params: vec![],
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
        type_params: vec![],
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
        type_params: vec![],
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
        type_params: vec![],
        attributes: vec![],
        variants: vec![],
        span: 0..10,
    });
    assert!(matches!(item, Item::ChoiceDef(_)));
}

#[test]
fn test_record_def_has_attributes_field() {
    let module = parse("type Postgres;").unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert!(r.attributes.is_empty());
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_attribute_on_record() {
    let input = "#[lang-item]\ntype Postgres = port: u32;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "lang-item");
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_multiple_attributes() {
    let input = "#[lang-item]\n#[deprecated]\ntype Postgres;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 2);
            assert_eq!(r.attributes[0].name, "lang-item");
            assert_eq!(r.attributes[1].name, "deprecated");
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

// --- Record definition parsing (Task 6) ---

#[test]
fn test_parse_record_def() {
    let input = "type Tls =\n  port: u16,\n  key: str,\n;";
    let module = parse(input).unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.name, "Tls");
            assert_eq!(r.fields.len(), 2);
            assert_eq!(r.fields[0].name, "port");
            assert_eq!(r.fields[1].name, "key");
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_record_with_attribute() {
    let input = "#[lang-item]\ntype Tls =\n  port: u16,\n;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "lang-item");
            assert_eq!(r.fields.len(), 1);
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_empty_record() {
    let module = parse("type Empty = ;").unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.name, "Empty");
            assert!(r.fields.is_empty());
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

// --- Choice definition parsing (Task 7) ---

#[test]
fn test_parse_choice_def() {
    let input = "type IpAddr = V4(IpAddrV4) | V6(IpAddrV6);";
    let module = parse(input).unwrap();
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::ChoiceDef(c) => {
            assert_eq!(c.name, "IpAddr");
            assert_eq!(c.variants.len(), 2);
            assert_eq!(c.variants[0].name, "V4");
            assert_eq!(c.variants[0].fields.len(), 1);
            assert_eq!(c.variants[1].name, "V6");
            assert_eq!(c.variants[1].fields.len(), 1);
        }
        other => panic!("expected ChoiceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_choice_unit_variant() {
    let input = "type Option = Some(T) | None;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ChoiceDef(c) => {
            assert_eq!(c.variants.len(), 2);
            assert_eq!(c.variants[0].name, "Some");
            assert_eq!(c.variants[0].fields.len(), 1);
            assert_eq!(c.variants[1].name, "None");
            assert!(c.variants[1].fields.is_empty());
        }
        other => panic!("expected ChoiceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_choice_with_attribute() {
    let input = "#[lang-item]\ntype IpAddr = V4(IpAddrV4) | V6(IpAddrV6);";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ChoiceDef(c) => {
            assert_eq!(c.attributes.len(), 1);
            assert_eq!(c.attributes[0].name, "lang-item");
        }
        other => panic!("expected ChoiceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_choice_multi_field_variant() {
    let input = "type Pair = Both(u32, str) | Neither;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::ChoiceDef(c) => {
            assert_eq!(c.variants[0].name, "Both");
            assert_eq!(c.variants[0].fields.len(), 2);
        }
        other => panic!("expected ChoiceDef, got {other:?}"),
    }
}

// --- Primitive and compound type expression parsing (Task 8) ---

#[test]
fn test_parse_primitive_type_in_field() {
    let input = "type Foo = x: u32, y: bool, z: str;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert!(matches!(
                &r.fields[0].ty,
                TypeExpr::Primitive(PrimitiveType::U32)
            ));
            assert!(matches!(
                &r.fields[1].ty,
                TypeExpr::Primitive(PrimitiveType::Bool)
            ));
            assert!(matches!(
                &r.fields[2].ty,
                TypeExpr::Primitive(PrimitiveType::Str)
            ));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_array_type() {
    let input = "type Foo = data: [u8; 4];";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => match &r.fields[0].ty {
            TypeExpr::Array { element, size } => {
                assert!(matches!(
                    element.as_ref(),
                    TypeExpr::Primitive(PrimitiveType::U8)
                ));
                assert_eq!(*size, 4);
            }
            other => panic!("expected Array, got {other:?}"),
        },
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_slice_type() {
    let input = "type Foo = data: [u8];";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => match &r.fields[0].ty {
            TypeExpr::Slice(element) => {
                assert!(matches!(
                    element.as_ref(),
                    TypeExpr::Primitive(PrimitiveType::U8)
                ));
            }
            other => panic!("expected Slice, got {other:?}"),
        },
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_tuple_type() {
    let input = "type Foo = pair: (u32, str);";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => match &r.fields[0].ty {
            TypeExpr::Tuple(elements) => {
                assert_eq!(elements.len(), 2);
                assert!(matches!(
                    &elements[0],
                    TypeExpr::Primitive(PrimitiveType::U32)
                ));
                assert!(matches!(
                    &elements[1],
                    TypeExpr::Primitive(PrimitiveType::Str)
                ));
            }
            other => panic!("expected Tuple, got {other:?}"),
        },
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_unit_type() {
    let input = "type Foo = nothing: ();";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert!(matches!(&r.fields[0].ty, TypeExpr::Unit));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}
