use spin_up::ast::{
    AsInterfaceBlock, Attribute, BinaryOp, ChoiceDef, Expr, FieldInit, FieldMapping, ImplBlock,
    InterfaceDef, InterfaceField, Item, LetBinding, PrimitiveType, RecordDef, StringPart, TypeExpr,
    UnaryOp, Variant,
};
use spin_up::parser::parse;
use spin_up::spin;

// Note: The `resource` keyword has been removed. Resource definitions now use
// the `type` keyword with record (product type) syntax: `type Foo = field: Type;`

#[test]
fn test_parse_single_import() {
    let module = spin! { import postgres };
    assert_eq!(module.imports.len(), 1);
    assert_eq!(module.imports[0].module_name, "postgres");
}

#[test]
fn test_parse_multiple_imports() {
    let module = spin! {
        import postgres
        import redis
    };
    assert_eq!(module.imports.len(), 2);
    assert_eq!(module.imports[0].module_name, "postgres");
    assert_eq!(module.imports[1].module_name, "redis");
}

#[test]
fn test_parse_import_with_hyphen() {
    let module = spin! { import spin-core };
    assert_eq!(module.imports[0].module_name, "spin-core");
}

#[test]
fn test_parse_empty_input() {
    let module = spin! {};
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
    let module = spin! { type Postgres; };
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
    let module = spin! { type Postgres = host: String; };
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
    let module = spin! { type Postgres = port: spin-core::TcpPort; };
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
    let module = spin! { type Postgres = tls: Option<Self::Tls>; };
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
    let module = spin! { type Postgres = tls: Self::Tls; };
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert!(matches!(&r.fields[0].ty, TypeExpr::SelfPath(n) if n == "Tls"));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_resource_trailing_comma_optional() {
    let module = spin! { type Postgres = host: String; };
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.fields.len(), 1);
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_multiple_fields() {
    let module = spin! {
        type Postgres = version: spin-core::Semver, host: spin-core::String, port: spin-core::TcpPort;
    };
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
    let module = spin! {
        import spin-core
        type Postgres = port: spin-core::TcpPort;
    };
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
        args: None,
        span: 0..11,
    };
    assert_eq!(attr.name, "lang-item");
    assert_eq!(attr.span, 0..11);
}

#[test]
fn test_ast_attribute_equality() {
    let a = Attribute {
        name: "lang-item".to_string(),
        args: None,
        span: 0..11,
    };
    let b = Attribute {
        name: "lang-item".to_string(),
        args: None,
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
            args: None,
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

// --- Attribute arguments (Phase 3a Task 3) ---

#[test]
fn test_parse_attribute_with_args() {
    let input = "#[delegate(PostgresEndpoint)]\ntype Proxy = frontend: str;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "delegate");
            assert_eq!(r.attributes[0].args.as_deref(), Some("PostgresEndpoint"));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_attribute_with_string_args() {
    let input = "#[default(\"postgres\")]\ntype Foo = name: str;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "default");
            assert_eq!(r.attributes[0].args.as_deref(), Some("\"postgres\""));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_attribute_without_args() {
    let input = "#[lang-item]\ntype Foo = name: str;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "lang-item");
            assert!(r.attributes[0].args.is_none());
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

#[test]
fn test_parse_attribute_with_nested_parens() {
    let input = "#[target(SocketAddr::V4(port: 5432))]\ntype Foo = name: str;";
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert_eq!(r.attributes.len(), 1);
            assert_eq!(r.attributes[0].name, "target");
            assert_eq!(
                r.attributes[0].args.as_deref(),
                Some("SocketAddr :: V4 ( port : 5432 )")
            );
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
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
    let module = spin! { type Postgres; };
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
    let module = spin! {
        type Tls =
            port: u16,
            key: str,
        ;
    };
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
    let module = spin! { type Empty = ; };
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
    let module = spin! { type IpAddr = V4(IpAddrV4) | V6(IpAddrV6); };
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
    let module = spin! { type Option = Some(T) | None; };
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
    let module = spin! { type Pair = Both(u32, str) | Neither; };
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
    let module = spin! { type Foo = x: u32, y: bool, z: str; };
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
    let module = spin! { type Foo = data: [u8; 4]; };
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
    let module = spin! { type Foo = data: [u8]; };
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
    let module = spin! { type Foo = pair: (u32, str); };
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
    let module = spin! { type Foo = nothing: (); };
    match &module.items[0] {
        Item::RecordDef(r) => {
            assert!(matches!(&r.fields[0].ty, TypeExpr::Unit));
        }
        other => panic!("expected RecordDef, got {other:?}"),
    }
}

// --- Phase 3a Task 4: Expression AST types ---

#[test]
fn test_expr_string_lit_construction() {
    let expr = Expr::StringLit("hello".to_string());
    assert!(matches!(expr, Expr::StringLit(s) if s == "hello"));
}

#[test]
fn test_expr_number_construction() {
    let expr = Expr::Number("42".to_string());
    assert!(matches!(expr, Expr::Number(n) if n == "42"));
}

#[test]
fn test_expr_bool_lit_construction() {
    let expr_true = Expr::BoolLit(true);
    let expr_false = Expr::BoolLit(false);
    assert!(matches!(expr_true, Expr::BoolLit(true)));
    assert!(matches!(expr_false, Expr::BoolLit(false)));
}

#[test]
fn test_expr_ident_construction() {
    let expr = Expr::Ident("my_var".to_string());
    assert!(matches!(expr, Expr::Ident(name) if name == "my_var"));
}

#[test]
fn test_expr_field_access_construction() {
    let expr = Expr::FieldAccess {
        object: Box::new(Expr::Self_),
        field: "port".to_string(),
    };
    match expr {
        Expr::FieldAccess { object, field } => {
            assert!(matches!(*object, Expr::Self_));
            assert_eq!(field, "port");
        }
        other => panic!("expected FieldAccess, got {other:?}"),
    }
}

#[test]
fn test_expr_type_construction() {
    let expr = Expr::TypeConstruction {
        type_name: "Proxy".to_string(),
        fields: vec![FieldInit {
            name: "host".to_string(),
            value: Expr::StringLit("localhost".to_string()),
            span: 0..10,
        }],
        as_interfaces: vec![],
    };
    match expr {
        Expr::TypeConstruction {
            type_name, fields, ..
        } => {
            assert_eq!(type_name, "Proxy");
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].name, "host");
        }
        other => panic!("expected TypeConstruction, got {other:?}"),
    }
}

#[test]
fn test_expr_type_construction_with_as_interface() {
    let expr = Expr::TypeConstruction {
        type_name: "MyServer".to_string(),
        fields: vec![],
        as_interfaces: vec![AsInterfaceBlock {
            interface_name: "Endpoint".to_string(),
            fields: vec![FieldInit {
                name: "port".to_string(),
                value: Expr::Number("8080".to_string()),
                span: 0..10,
            }],
            span: 0..30,
        }],
    };
    match expr {
        Expr::TypeConstruction { as_interfaces, .. } => {
            assert_eq!(as_interfaces.len(), 1);
            assert_eq!(as_interfaces[0].interface_name, "Endpoint");
            assert_eq!(as_interfaces[0].fields.len(), 1);
        }
        other => panic!("expected TypeConstruction, got {other:?}"),
    }
}

#[test]
fn test_expr_variant_construction() {
    let expr = Expr::VariantConstruction {
        type_name: "SocketAddr".to_string(),
        variant: "V4".to_string(),
        args: vec![Expr::Ident("addr".to_string())],
    };
    match expr {
        Expr::VariantConstruction {
            type_name,
            variant,
            args,
        } => {
            assert_eq!(type_name, "SocketAddr");
            assert_eq!(variant, "V4");
            assert_eq!(args.len(), 1);
        }
        other => panic!("expected VariantConstruction, got {other:?}"),
    }
}

#[test]
fn test_expr_named_construction() {
    let expr = Expr::NamedConstruction {
        type_name: "SemVer".to_string(),
        fields: vec![FieldInit {
            name: "major".to_string(),
            value: Expr::Number("17".to_string()),
            span: 0..10,
        }],
    };
    match expr {
        Expr::NamedConstruction { type_name, fields } => {
            assert_eq!(type_name, "SemVer");
            assert_eq!(fields.len(), 1);
        }
        other => panic!("expected NamedConstruction, got {other:?}"),
    }
}

#[test]
fn test_expr_binary_op_construction() {
    let expr = Expr::BinaryOp {
        left: Box::new(Expr::It),
        op: BinaryOp::Gte,
        right: Box::new(Expr::Number("15".to_string())),
    };
    match expr {
        Expr::BinaryOp { left, op, right } => {
            assert!(matches!(*left, Expr::It));
            assert_eq!(op, BinaryOp::Gte);
            assert!(matches!(*right, Expr::Number(n) if n == "15"));
        }
        other => panic!("expected BinaryOp, got {other:?}"),
    }
}

#[test]
fn test_expr_unary_op_construction() {
    let expr = Expr::UnaryOp {
        op: UnaryOp::Not,
        operand: Box::new(Expr::BoolLit(true)),
    };
    match expr {
        Expr::UnaryOp { op, operand } => {
            assert_eq!(op, UnaryOp::Not);
            assert!(matches!(*operand, Expr::BoolLit(true)));
        }
        other => panic!("expected UnaryOp, got {other:?}"),
    }
}

#[test]
fn test_expr_it_construction() {
    let expr = Expr::It;
    assert!(matches!(expr, Expr::It));
}

#[test]
fn test_expr_self_construction() {
    let expr = Expr::Self_;
    assert!(matches!(expr, Expr::Self_));
}

#[test]
fn test_expr_none_construction() {
    let expr = Expr::None_;
    assert!(matches!(expr, Expr::None_));
}

#[test]
fn test_binary_op_equality() {
    assert_eq!(BinaryOp::Eq, BinaryOp::Eq);
    assert_eq!(BinaryOp::NotEq, BinaryOp::NotEq);
    assert_eq!(BinaryOp::Lt, BinaryOp::Lt);
    assert_eq!(BinaryOp::Gt, BinaryOp::Gt);
    assert_eq!(BinaryOp::Lte, BinaryOp::Lte);
    assert_eq!(BinaryOp::Gte, BinaryOp::Gte);
    assert_eq!(BinaryOp::And, BinaryOp::And);
    assert_eq!(BinaryOp::Or, BinaryOp::Or);
    assert_ne!(BinaryOp::Eq, BinaryOp::NotEq);
}

#[test]
fn test_unary_op_equality() {
    assert_eq!(UnaryOp::Not, UnaryOp::Not);
}

#[test]
fn test_field_init_construction() {
    let fi = FieldInit {
        name: "port".to_string(),
        value: Expr::Number("8080".to_string()),
        span: 0..10,
    };
    assert_eq!(fi.name, "port");
    assert_eq!(fi.span, 0..10);
}

// --- Phase 3a Task 5: Interface, Impl, Let, and Field Attributes ---

#[test]
fn test_item_interface_def_variant() {
    let item = Item::InterfaceDef(InterfaceDef {
        name: "Endpoint".to_string(),
        type_params: vec![],
        fields: vec![],
        span: 0..30,
    });
    assert!(matches!(item, Item::InterfaceDef(_)));
}

#[test]
fn test_interface_def_with_fields() {
    let iface = InterfaceDef {
        name: "Endpoint".to_string(),
        type_params: vec![],
        fields: vec![
            InterfaceField {
                name: "host".to_string(),
                ty: TypeExpr::Primitive(PrimitiveType::Str),
                attributes: vec![],
                span: 0..10,
            },
            InterfaceField {
                name: "port".to_string(),
                ty: TypeExpr::Primitive(PrimitiveType::U16),
                attributes: vec![Attribute {
                    name: "default".to_string(),
                    args: Some("5432".to_string()),
                    span: 0..15,
                }],
                span: 11..20,
            },
        ],
        span: 0..40,
    };
    assert_eq!(iface.name, "Endpoint");
    assert_eq!(iface.fields.len(), 2);
    assert_eq!(iface.fields[0].name, "host");
    assert!(iface.fields[0].attributes.is_empty());
    assert_eq!(iface.fields[1].attributes.len(), 1);
    assert_eq!(iface.fields[1].attributes[0].name, "default");
}

#[test]
fn test_interface_def_with_type_params() {
    let iface = InterfaceDef {
        name: "Container".to_string(),
        type_params: vec!["T".to_string()],
        fields: vec![InterfaceField {
            name: "items".to_string(),
            ty: TypeExpr::Named("T".to_string()),
            attributes: vec![],
            span: 0..10,
        }],
        span: 0..30,
    };
    assert_eq!(iface.type_params, vec!["T"]);
}

#[test]
fn test_item_impl_block_variant() {
    let item = Item::ImplBlock(ImplBlock {
        interface_name: "Endpoint".to_string(),
        type_name: "MyServer".to_string(),
        mappings: vec![],
        span: 0..30,
    });
    assert!(matches!(item, Item::ImplBlock(_)));
}

#[test]
fn test_impl_block_with_mappings() {
    let impl_block = ImplBlock {
        interface_name: "Endpoint".to_string(),
        type_name: "MyServer".to_string(),
        mappings: vec![
            FieldMapping {
                name: "host".to_string(),
                value: Expr::FieldAccess {
                    object: Box::new(Expr::Self_),
                    field: "hostname".to_string(),
                },
                span: 0..20,
            },
            FieldMapping {
                name: "port".to_string(),
                value: Expr::FieldAccess {
                    object: Box::new(Expr::FieldAccess {
                        object: Box::new(Expr::Self_),
                        field: "config".to_string(),
                    }),
                    field: "port".to_string(),
                },
                span: 21..50,
            },
        ],
        span: 0..60,
    };
    assert_eq!(impl_block.interface_name, "Endpoint");
    assert_eq!(impl_block.type_name, "MyServer");
    assert_eq!(impl_block.mappings.len(), 2);
    assert_eq!(impl_block.mappings[0].name, "host");
    assert_eq!(impl_block.mappings[1].name, "port");
}

#[test]
fn test_item_let_binding_variant() {
    let item = Item::LetBinding(LetBinding {
        name: "proxy".to_string(),
        ty: None,
        value: Expr::StringLit("hello".to_string()),
        span: 0..20,
    });
    assert!(matches!(item, Item::LetBinding(_)));
}

#[test]
fn test_let_binding_with_type_annotation() {
    let binding = LetBinding {
        name: "port".to_string(),
        ty: Some(TypeExpr::Primitive(PrimitiveType::U16)),
        value: Expr::Number("5432".to_string()),
        span: 0..20,
    };
    assert_eq!(binding.name, "port");
    assert!(binding.ty.is_some());
    match binding.ty.unwrap() {
        TypeExpr::Primitive(PrimitiveType::U16) => {}
        other => panic!("expected Primitive(U16), got {other:?}"),
    }
}

#[test]
fn test_let_binding_without_type_annotation() {
    let binding = LetBinding {
        name: "name".to_string(),
        ty: None,
        value: Expr::StringLit("hello".to_string()),
        span: 0..20,
    };
    assert!(binding.ty.is_none());
}

#[test]
fn test_field_mapping_construction() {
    let mapping = FieldMapping {
        name: "listen_on".to_string(),
        value: Expr::FieldAccess {
            object: Box::new(Expr::Self_),
            field: "listen_on".to_string(),
        },
        span: 0..30,
    };
    assert_eq!(mapping.name, "listen_on");
    assert_eq!(mapping.span, 0..30);
}

// --- Phase 3a Task 6: Parser — Interface Definitions ---

#[test]
fn test_parse_interface_def() {
    let module = spin! { interface Endpoint = host: str, port: u16; };
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::InterfaceDef(i) => {
            assert_eq!(i.name, "Endpoint");
            assert_eq!(i.fields.len(), 2);
            assert_eq!(i.fields[0].name, "host");
            assert_eq!(i.fields[1].name, "port");
        }
        other => panic!("expected InterfaceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_interface_with_field_attributes() {
    let input = r#"interface Endpoint =
  #[default("localhost")]
  host: str,
  port: u16,
;"#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::InterfaceDef(i) => {
            assert_eq!(i.fields[0].attributes.len(), 1);
            assert_eq!(i.fields[0].attributes[0].name, "default");
            assert!(i.fields[0].attributes[0].args.is_some());
            assert_eq!(i.fields[1].attributes.len(), 0);
        }
        other => panic!("expected InterfaceDef, got {other:?}"),
    }
}

#[test]
fn test_parse_interface_with_generic() {
    let module = spin! { interface Container<T> = items: T; };
    match &module.items[0] {
        Item::InterfaceDef(i) => {
            assert_eq!(i.type_params, vec!["T"]);
        }
        other => panic!("expected InterfaceDef, got {other:?}"),
    }
}

#[test]
fn test_interface_field_construction() {
    let field = InterfaceField {
        name: "host".to_string(),
        ty: TypeExpr::Primitive(PrimitiveType::Str),
        attributes: vec![Attribute {
            name: "default".to_string(),
            args: Some("\"localhost\"".to_string()),
            span: 0..20,
        }],
        span: 0..30,
    };
    assert_eq!(field.name, "host");
    assert_eq!(field.attributes.len(), 1);
}

// --- Phase 3a Task 8: Parser — Impl Blocks ---

#[test]
fn test_parse_impl_block() {
    let module = spin! {
        impl Endpoint for MyServer {
            host: self.hostname,
            port: self.config.port,
        }
    };
    assert_eq!(module.items.len(), 1);
    match &module.items[0] {
        Item::ImplBlock(i) => {
            assert_eq!(i.interface_name, "Endpoint");
            assert_eq!(i.type_name, "MyServer");
            assert_eq!(i.mappings.len(), 2);
            assert_eq!(i.mappings[0].name, "host");
            assert_eq!(i.mappings[1].name, "port");
        }
        other => panic!("expected ImplBlock, got {other:?}"),
    }
}

#[test]
fn test_parse_impl_block_with_expression() {
    let module = spin! {
        impl Endpoint for MyServer {
            greeting: "hello",
        }
    };
    match &module.items[0] {
        Item::ImplBlock(i) => {
            assert_eq!(i.mappings.len(), 1);
            assert!(matches!(&i.mappings[0].value, Expr::StringLit(s) if s == "hello"));
        }
        other => panic!("expected ImplBlock, got {other:?}"),
    }
}

// --- Phase 3a Task 10: Parser — As-Interface Blocks in Type Constructions ---

#[test]
fn test_parse_as_interface_in_construction() {
    let module = spin! {
        let x = MyType {
            name: "foo",
            <as Endpoint> {
                port: 8080,
            }
        }
    };
    match &module.items[0] {
        Item::LetBinding(l) => match &l.value {
            Expr::TypeConstruction {
                type_name,
                fields,
                as_interfaces,
            } => {
                assert_eq!(type_name, "MyType");
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].name, "name");
                assert_eq!(as_interfaces.len(), 1);
                assert_eq!(as_interfaces[0].interface_name, "Endpoint");
                assert_eq!(as_interfaces[0].fields.len(), 1);
                assert_eq!(as_interfaces[0].fields[0].name, "port");
            }
            other => panic!("expected TypeConstruction, got {other:?}"),
        },
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

// --- Step 4: String Interpolation Parsing ---

#[test]
fn test_parse_plain_string_no_interpolation() {
    let module = spin! { let x = "hello world" };
    match &module.items[0] {
        Item::LetBinding(l) => {
            assert!(matches!(&l.value, Expr::StringLit(s) if s == "hello world"));
        }
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

#[test]
fn test_parse_string_interpolation_simple() {
    let input = r#"let x = "hello ${name}""#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::LetBinding(l) => match &l.value {
            Expr::StringInterpolation(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], StringPart::Literal(s) if s == "hello "));
                assert!(matches!(&parts[1], StringPart::Expr(Expr::Ident(n)) if n == "name"));
            }
            other => panic!("expected StringInterpolation, got {other:?}"),
        },
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

#[test]
fn test_parse_string_interpolation_dotted_path() {
    let input = r#"let x = "host: ${postgres.host}""#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::LetBinding(l) => match &l.value {
            Expr::StringInterpolation(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], StringPart::Literal(s) if s == "host: "));
                match &parts[1] {
                    StringPart::Expr(Expr::FieldAccess { field, .. }) => {
                        assert_eq!(field, "host");
                    }
                    other => panic!("expected FieldAccess, got {other:?}"),
                }
            }
            other => panic!("expected StringInterpolation, got {other:?}"),
        },
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

#[test]
fn test_parse_string_multiple_interpolations() {
    let input = r#"let x = "${host}:${port}""#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::LetBinding(l) => match &l.value {
            Expr::StringInterpolation(parts) => {
                assert_eq!(parts.len(), 3);
                assert!(matches!(&parts[0], StringPart::Expr(Expr::Ident(n)) if n == "host"));
                assert!(matches!(&parts[1], StringPart::Literal(s) if s == ":"));
                assert!(matches!(&parts[2], StringPart::Expr(Expr::Ident(n)) if n == "port"));
            }
            other => panic!("expected StringInterpolation, got {other:?}"),
        },
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

#[test]
fn test_parse_string_interpolation_trailing_literal() {
    let input = r#"let x = "${name}!""#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::LetBinding(l) => match &l.value {
            Expr::StringInterpolation(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], StringPart::Expr(Expr::Ident(n)) if n == "name"));
                assert!(matches!(&parts[1], StringPart::Literal(s) if s == "!"));
            }
            other => panic!("expected StringInterpolation, got {other:?}"),
        },
        other => panic!("expected LetBinding, got {other:?}"),
    }
}

#[test]
fn test_parse_string_interpolation_deep_dotted_path() {
    let input = r#"let x = "${a.b.c}""#;
    let module = parse(input).unwrap();
    match &module.items[0] {
        Item::LetBinding(l) => match &l.value {
            Expr::StringInterpolation(parts) => {
                assert_eq!(parts.len(), 1);
                match &parts[0] {
                    StringPart::Expr(Expr::FieldAccess { object, field }) => {
                        assert_eq!(field, "c");
                        match object.as_ref() {
                            Expr::FieldAccess {
                                object: inner,
                                field: mid,
                            } => {
                                assert_eq!(mid, "b");
                                assert!(matches!(inner.as_ref(), Expr::Ident(n) if n == "a"));
                            }
                            other => panic!("expected nested FieldAccess, got {other:?}"),
                        }
                    }
                    other => panic!("expected FieldAccess, got {other:?}"),
                }
            }
            other => panic!("expected StringInterpolation, got {other:?}"),
        },
        other => panic!("expected LetBinding, got {other:?}"),
    }
}
