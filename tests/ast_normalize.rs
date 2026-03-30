use spin_up::ast::{
    Attribute, ChoiceDef, Field, Item, PrimitiveType, RecordDef, TypeExpr, Variant,
};
use spin_up::ast_normalize::normalize_item;

#[test]
fn test_normalize_strips_spans() {
    let item1 = Item::RecordDef(RecordDef {
        name: "Foo".to_string(),
        type_params: vec![],
        attributes: vec![],
        fields: vec![Field {
            name: "x".to_string(),
            ty: TypeExpr::Primitive(PrimitiveType::U32),
            span: 0..10,
        }],
        span: 0..20,
    });

    let item2 = Item::RecordDef(RecordDef {
        name: "Foo".to_string(),
        type_params: vec![],
        attributes: vec![],
        fields: vec![Field {
            name: "x".to_string(),
            ty: TypeExpr::Primitive(PrimitiveType::U32),
            span: 50..60,
        }],
        span: 50..100,
    });

    assert_eq!(normalize_item(&item1), normalize_item(&item2));
}

#[test]
fn test_normalize_different_items_not_equal() {
    let item1 = Item::RecordDef(RecordDef {
        name: "Foo".to_string(),
        type_params: vec![],
        attributes: vec![],
        fields: vec![Field {
            name: "x".to_string(),
            ty: TypeExpr::Primitive(PrimitiveType::U32),
            span: 0..10,
        }],
        span: 0..20,
    });

    let item2 = Item::RecordDef(RecordDef {
        name: "Bar".to_string(),
        type_params: vec![],
        attributes: vec![],
        fields: vec![Field {
            name: "x".to_string(),
            ty: TypeExpr::Primitive(PrimitiveType::U32),
            span: 0..10,
        }],
        span: 0..20,
    });

    assert_ne!(normalize_item(&item1), normalize_item(&item2));
}

#[test]
fn test_normalize_choice() {
    let item = Item::ChoiceDef(ChoiceDef {
        name: "IpAddr".to_string(),
        type_params: vec![],
        attributes: vec![Attribute {
            name: "lang-item".to_string(),
            args: None,
            span: 0..11,
        }],
        variants: vec![
            Variant {
                name: "V4".to_string(),
                fields: vec![TypeExpr::Named("IpAddrV4".to_string())],
                span: 0..15,
            },
            Variant {
                name: "V6".to_string(),
                fields: vec![TypeExpr::Named("IpAddrV6".to_string())],
                span: 16..30,
            },
        ],
        span: 0..50,
    });

    let normalized = normalize_item(&item);
    // Verify it's deterministic
    assert_eq!(normalized, normalize_item(&item));
}
