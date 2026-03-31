use crate::analysis::registry::{TypeDef, TypeRegistry};
use crate::ast::{Expr, PrimitiveType, TypeExpr};

/// Represents the inferred type of an expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeInfo {
    /// A primitive type (str, u32, bool, etc.)
    Primitive(PrimitiveType),
    /// A user-defined type referenced by name
    Named(String),
    /// A generic type, e.g. `Option<str>`
    Generic { name: String, args: Vec<TypeInfo> },
    /// Type could not be determined
    Unknown,
}

impl std::fmt::Display for TypeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeInfo::Primitive(p) => write!(f, "{}", primitive_name(p)),
            TypeInfo::Named(name) => write!(f, "{name}"),
            TypeInfo::Generic { name, args } => {
                write!(f, "{name}<")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ">")
            }
            TypeInfo::Unknown => write!(f, "unknown"),
        }
    }
}

/// Convert a `TypeExpr` from the AST into a `TypeInfo` for comparison.
pub fn type_expr_to_type_info(ty: &TypeExpr) -> TypeInfo {
    match ty {
        TypeExpr::Primitive(p) => TypeInfo::Primitive(p.clone()),
        TypeExpr::Named(name) => TypeInfo::Named(name.clone()),
        TypeExpr::Generic { name, args } => TypeInfo::Generic {
            name: name.clone(),
            args: args.iter().map(type_expr_to_type_info).collect(),
        },
        TypeExpr::Path { module, name } => TypeInfo::Named(format!("{module}::{name}")),
        TypeExpr::SelfPath(name) => TypeInfo::Named(format!("Self::{name}")),
        TypeExpr::List(element) => {
            let inner = type_expr_to_type_info(element);
            TypeInfo::Generic {
                name: "List".to_string(),
                args: vec![inner],
            }
        }
        TypeExpr::HashMap { key, value } => {
            let key_info = type_expr_to_type_info(key);
            let value_info = type_expr_to_type_info(value);
            TypeInfo::Generic {
                name: "HashMap".to_string(),
                args: vec![key_info, value_info],
            }
        }
    }
}

/// Infer the type of an expression within the context of an implementing type.
///
/// `impl_type_name` is the name of the type in the `impl ... for <Type>` block.
pub fn infer_expr_type(expr: &Expr, impl_type_name: &str, registry: &TypeRegistry) -> TypeInfo {
    match expr {
        Expr::Self_ => TypeInfo::Named(impl_type_name.to_string()),

        Expr::StringLit(_) | Expr::StringInterpolation(_) => {
            TypeInfo::Primitive(PrimitiveType::String)
        }

        Expr::BoolLit(_) => TypeInfo::Primitive(PrimitiveType::Bool),

        Expr::Number(_) => TypeInfo::Unknown, // numeric type inference is deferred

        Expr::None_ => TypeInfo::Generic {
            name: "Option".to_string(),
            args: vec![TypeInfo::Unknown],
        },

        Expr::Ident(name) => {
            // The parser may produce Ident("self") instead of Self_
            if name == "self" {
                return TypeInfo::Named(impl_type_name.to_string());
            }
            // Look up in let bindings
            if let Some(binding) = registry.lookup_binding(name)
                && let Some(ty) = &binding.ty
            {
                return type_expr_to_type_info(ty);
            }
            TypeInfo::Unknown
        }

        Expr::FieldAccess { object, field } => {
            let object_type = infer_expr_type(object, impl_type_name, registry);
            resolve_field_type(&object_type, field, registry)
        }

        // For other expression types, return Unknown
        _ => TypeInfo::Unknown,
    }
}

/// Given an object type and a field name, resolve the field's type.
fn resolve_field_type(
    object_type: &TypeInfo,
    field_name: &str,
    registry: &TypeRegistry,
) -> TypeInfo {
    let type_name = match object_type {
        TypeInfo::Named(name) => name.as_str(),
        _ => return TypeInfo::Unknown,
    };

    let Some(type_def) = registry.lookup_type(type_name) else {
        return TypeInfo::Unknown;
    };

    match type_def {
        TypeDef::Record(record) => {
            for f in &record.fields {
                if f.name == field_name {
                    return type_expr_to_type_info(&f.ty);
                }
            }
            TypeInfo::Unknown
        }
        TypeDef::Choice(_) => TypeInfo::Unknown,
    }
}

/// Check whether two types are compatible.
///
/// `Unknown` is compatible with anything (lenient where we cannot infer).
/// `Option<Unknown>` is compatible with any `Option<T>`.
pub fn types_compatible(expected: &TypeInfo, actual: &TypeInfo) -> bool {
    // Unknown is always compatible (we can't check)
    if matches!(expected, TypeInfo::Unknown) || matches!(actual, TypeInfo::Unknown) {
        return true;
    }

    match (expected, actual) {
        (TypeInfo::Primitive(a), TypeInfo::Primitive(b)) => a == b,
        (TypeInfo::Named(a), TypeInfo::Named(b)) => a == b,
        (TypeInfo::Generic { name: en, args: ea }, TypeInfo::Generic { name: an, args: aa }) => {
            if en != an {
                return false;
            }
            if ea.len() != aa.len() {
                return false;
            }
            ea.iter()
                .zip(aa.iter())
                .all(|(e, a)| types_compatible(e, a))
        }
        _ => false,
    }
}

fn primitive_name(p: &PrimitiveType) -> &'static str {
    match p {
        PrimitiveType::Bool => "bool",
        PrimitiveType::Number => "number",
        PrimitiveType::String => "string",
    }
}
