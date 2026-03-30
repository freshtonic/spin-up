use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::analysis::registry::TypeRegistry;
use crate::diagnostics::{DiagnosticKind, Diagnostics};
use crate::parser;
use crate::spin_path::SpinPath;

/// The result of resolving all modules reachable from an entry point.
pub struct ResolveResult {
    pub registry: TypeRegistry,
    pub diagnostics: Diagnostics,
}

/// Resolve all modules reachable from the entry point, registering their
/// items into a [`TypeRegistry`] and collecting diagnostics for any errors.
///
/// `entry_path` is the path to the main `.spin` file.
/// `spin_path_dirs` are the directories to search when resolving imports.
pub fn resolve_modules(entry_path: &Path, spin_path_dirs: &[PathBuf]) -> ResolveResult {
    let mut registry = TypeRegistry::new();
    let mut diagnostics = Diagnostics::new();
    let mut visited: HashSet<String> = HashSet::new();

    // Build SpinPath from the provided directories.
    let spin_path_str = spin_path_dirs
        .iter()
        .map(|d| d.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(":");

    let spin_path = match spin_path_str.parse::<SpinPath>() {
        Ok(sp) => sp,
        Err(e) => {
            diagnostics.error(
                DiagnosticKind::UnresolvedImport {
                    module: format!("failed to build SPIN_PATH: {e}"),
                },
                0..0,
                &entry_path.display().to_string(),
            );
            return ResolveResult {
                registry,
                diagnostics,
            };
        }
    };

    // Read and parse the entry point file.
    let source_name = entry_path.display().to_string();
    let source = match std::fs::read_to_string(entry_path) {
        Ok(s) => s,
        Err(e) => {
            diagnostics.error(
                DiagnosticKind::UnresolvedImport {
                    module: format!("failed to read entry point: {e}"),
                },
                0..0,
                &source_name,
            );
            return ResolveResult {
                registry,
                diagnostics,
            };
        }
    };

    // Derive a module name from the filename (without .spin extension).
    let entry_module_name = entry_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "main".to_string());

    resolve_module_recursive(
        &entry_module_name,
        &source,
        &source_name,
        &spin_path,
        &mut registry,
        &mut diagnostics,
        &mut visited,
    );

    ResolveResult {
        registry,
        diagnostics,
    }
}

fn resolve_module_recursive(
    module_name: &str,
    source: &str,
    source_name: &str,
    spin_path: &SpinPath,
    registry: &mut TypeRegistry,
    diagnostics: &mut Diagnostics,
    visited: &mut HashSet<String>,
) {
    if visited.contains(module_name) {
        return;
    }
    visited.insert(module_name.to_string());

    let module = match parser::parse(source) {
        Ok(m) => m,
        Err(_e) => {
            diagnostics.error(
                DiagnosticKind::UnresolvedImport {
                    module: format!("parse error in module '{module_name}'"),
                },
                0..0,
                source_name,
            );
            return;
        }
    };

    // Process imports before registering this module's items, so that
    // imported types are available in the registry.
    for import in &module.imports {
        let imported_name = &import.module_name;

        if visited.contains(imported_name) {
            // Already resolved (or currently resolving -- circular).
            continue;
        }

        match spin_path.resolve_source(imported_name) {
            Ok(imported_source) => {
                let imported_source_name = format!("{imported_name}.spin");
                resolve_module_recursive(
                    imported_name,
                    &imported_source,
                    &imported_source_name,
                    spin_path,
                    registry,
                    diagnostics,
                    visited,
                );
            }
            Err(_) => {
                diagnostics.error(
                    DiagnosticKind::UnresolvedImport {
                        module: imported_name.clone(),
                    },
                    import.span.clone(),
                    source_name,
                );
            }
        }
    }

    registry.register_module(module_name, &module);
}
