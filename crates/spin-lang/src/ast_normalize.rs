/// Normalized AST types with spans stripped for structural comparison.
use crate::ast::{self, Item};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizedItem {
    RecordDef(NormalizedRecordDef),
    ChoiceDef(NormalizedChoiceDef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedRecordDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub attributes: Vec<NormalizedAttribute>,
    pub fields: Vec<NormalizedField>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedChoiceDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub attributes: Vec<NormalizedAttribute>,
    pub variants: Vec<NormalizedVariant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedAttribute {
    pub name: String,
    pub args: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedField {
    pub name: String,
    pub ty: NormalizedTypeExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedVariant {
    pub name: String,
    pub fields: Vec<NormalizedTypeExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizedTypeExpr {
    Named(String),
    Primitive(ast::PrimitiveType),
    Path {
        module: String,
        name: String,
    },
    Generic {
        name: String,
        args: Vec<NormalizedTypeExpr>,
    },
    SelfPath(String),
    List(Box<NormalizedTypeExpr>),
    HashMap {
        key: Box<NormalizedTypeExpr>,
        value: Box<NormalizedTypeExpr>,
    },
}

/// Normalize an AST item by stripping all span information for structural comparison.
pub fn normalize_item(item: &Item) -> NormalizedItem {
    match item {
        Item::RecordDef(record) => NormalizedItem::RecordDef(normalize_record(record)),
        Item::ChoiceDef(choice) => NormalizedItem::ChoiceDef(normalize_choice(choice)),
        Item::InterfaceDef(_) => {
            panic!("normalization of InterfaceDef is not yet implemented")
        }
        Item::ImplBlock(_) => {
            panic!("normalization of ImplBlock is not yet implemented")
        }
        Item::LetBinding(_) => {
            panic!("normalization of LetBinding is not yet implemented")
        }
    }
}

fn normalize_record(record: &ast::RecordDef) -> NormalizedRecordDef {
    NormalizedRecordDef {
        name: record.name.clone(),
        type_params: record.type_params.clone(),
        attributes: record.attributes.iter().map(normalize_attribute).collect(),
        fields: record.fields.iter().map(normalize_field).collect(),
    }
}

fn normalize_choice(choice: &ast::ChoiceDef) -> NormalizedChoiceDef {
    NormalizedChoiceDef {
        name: choice.name.clone(),
        type_params: choice.type_params.clone(),
        attributes: choice.attributes.iter().map(normalize_attribute).collect(),
        variants: choice.variants.iter().map(normalize_variant).collect(),
    }
}

fn normalize_attribute(attr: &ast::Attribute) -> NormalizedAttribute {
    NormalizedAttribute {
        name: attr.name.clone(),
        args: attr.args.clone(),
    }
}

fn normalize_field(field: &ast::Field) -> NormalizedField {
    NormalizedField {
        name: field.name.clone(),
        ty: normalize_type_expr(&field.ty),
    }
}

fn normalize_variant(variant: &ast::Variant) -> NormalizedVariant {
    NormalizedVariant {
        name: variant.name.clone(),
        fields: variant.fields.iter().map(normalize_type_expr).collect(),
    }
}

fn normalize_type_expr(ty: &ast::TypeExpr) -> NormalizedTypeExpr {
    match ty {
        ast::TypeExpr::Named(name) => NormalizedTypeExpr::Named(name.clone()),
        ast::TypeExpr::Primitive(prim) => NormalizedTypeExpr::Primitive(prim.clone()),
        ast::TypeExpr::Path { module, name } => NormalizedTypeExpr::Path {
            module: module.clone(),
            name: name.clone(),
        },
        ast::TypeExpr::Generic { name, args } => NormalizedTypeExpr::Generic {
            name: name.clone(),
            args: args.iter().map(normalize_type_expr).collect(),
        },
        ast::TypeExpr::SelfPath(name) => NormalizedTypeExpr::SelfPath(name.clone()),
        ast::TypeExpr::List(element) => {
            NormalizedTypeExpr::List(Box::new(normalize_type_expr(element)))
        }
        ast::TypeExpr::HashMap { key, value } => NormalizedTypeExpr::HashMap {
            key: Box::new(normalize_type_expr(key)),
            value: Box::new(normalize_type_expr(value)),
        },
    }
}
