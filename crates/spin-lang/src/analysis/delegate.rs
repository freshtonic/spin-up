use crate::analysis::registry::{TypeDef, TypeRegistry};
use crate::ast::{Field, RecordDef, TypeExpr};
use crate::diagnostics::{DiagnosticKind, Diagnostics};

/// Validate all `#[delegate(X)]` / `#[target(X)]` attribute pairs.
///
/// For every record type with a `#[delegate(X)]` attribute:
/// - There must be exactly one field with `#[target(X)]`
/// - The interface `X` must exist in the registry
/// - The target field's type must match or implement `X`
///
/// Orphan `#[target(X)]` attributes (without a corresponding `#[delegate(X)]`
/// on the type) are also flagged.
pub fn check_delegates(registry: &TypeRegistry) -> Diagnostics {
    let mut diags = Diagnostics::new();

    for type_def in registry.all_types().values() {
        let TypeDef::Record(record) = type_def else {
            continue;
        };

        check_record_delegates(registry, record, &mut diags);
    }

    diags
}

fn check_record_delegates(registry: &TypeRegistry, record: &RecordDef, diags: &mut Diagnostics) {
    let delegate_interfaces: Vec<&str> = record
        .attributes
        .iter()
        .filter(|a| a.name == "delegate")
        .filter_map(|a| a.args.as_deref())
        .collect();

    let target_interfaces: Vec<(&str, &Field)> = record
        .fields
        .iter()
        .flat_map(|field| {
            field
                .attributes
                .iter()
                .filter(|a| a.name == "target")
                .filter_map(|a| a.args.as_deref())
                .map(move |iface| (iface, field))
        })
        .collect();

    // Check each delegate has exactly one matching target
    for iface_name in &delegate_interfaces {
        let matching_targets: Vec<&&Field> = target_interfaces
            .iter()
            .filter(|(name, _)| name == iface_name)
            .map(|(_, field)| field)
            .collect();

        if matching_targets.is_empty() {
            diags.error(
                DiagnosticKind::InvalidDelegate {
                    reason: format!(
                        "type `{}` has #[delegate({iface_name})] but no field with #[target({iface_name})]",
                        record.name
                    ),
                },
                record.span.clone(),
                "check",
            );
            continue;
        }

        if matching_targets.len() > 1 {
            diags.error(
                DiagnosticKind::InvalidDelegate {
                    reason: format!(
                        "type `{}` has multiple fields with #[target({iface_name})]",
                        record.name
                    ),
                },
                record.span.clone(),
                "check",
            );
            continue;
        }

        // Check the interface exists
        if registry.lookup_interface(iface_name).is_none() {
            diags.error(
                DiagnosticKind::UnknownInterface {
                    name: iface_name.to_string(),
                },
                record.span.clone(),
                "check",
            );
            continue;
        }

        // Check target field type matches or implements the interface
        let target_field = matching_targets[0];
        if !field_type_satisfies_interface(registry, target_field, iface_name) {
            diags.error(
                DiagnosticKind::InvalidDelegate {
                    reason: format!(
                        "target field `{}` does not implement interface `{iface_name}`",
                        target_field.name
                    ),
                },
                target_field.span.clone(),
                "check",
            );
        }
    }

    // Check for orphan targets (target without delegate)
    for (iface_name, field) in &target_interfaces {
        if !delegate_interfaces.contains(iface_name) {
            diags.error(
                DiagnosticKind::InvalidDelegate {
                    reason: format!(
                        "field `{}` has #[target({iface_name})] but type `{}` has no #[delegate({iface_name})]",
                        field.name, record.name
                    ),
                },
                field.span.clone(),
                "check",
            );
        }
    }
}

/// Returns true if the field's type is the interface itself or implements it
/// via an impl block.
fn field_type_satisfies_interface(
    registry: &TypeRegistry,
    field: &Field,
    interface_name: &str,
) -> bool {
    let type_name = match &field.ty.kind {
        TypeExpr::Named(name) => name.as_str(),
        TypeExpr::Path { name, .. } => name.as_str(),
        _ => return false,
    };

    // Direct match: field type IS the interface name
    if type_name == interface_name {
        return true;
    }

    // Check if there's an impl block for the field's type implementing the interface
    registry
        .lookup_impls_for_type(type_name)
        .iter()
        .any(|imp| imp.interface_name == interface_name)
}
