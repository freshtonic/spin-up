use std::collections::HashMap;
use std::ops::Range;

use miette::{Diagnostic as MietteDiagnostic, NamedSource, SourceSpan};

/// A rendered error suitable for display via miette.
#[derive(Debug, Clone, thiserror::Error, MietteDiagnostic)]
#[error("{message}")]
pub struct SpinError {
    message: String,

    #[help]
    help: Option<String>,

    #[source_code]
    src: NamedSource<String>,

    #[label("{label}")]
    span: SourceSpan,

    label: String,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub span: Range<usize>,
    pub source_name: String,
}

#[derive(Debug, Clone)]
pub enum DiagnosticKind {
    // Type unification errors
    TypeMismatch {
        expected: String,
        found: String,
    },
    UnknownType {
        name: String,
    },
    UnknownInterface {
        name: String,
    },
    MissingField {
        field: String,
        interface: String,
    },
    DuplicateField {
        field: String,
    },
    RedefinitionTypeMismatch {
        name: String,
        expected: String,
        found: String,
    },

    // Impl errors
    InvalidDelegate {
        reason: String,
    },
    InvalidAsInterface {
        type_name: String,
        interface: String,
    },

    // Constraint errors
    ConstraintViolation {
        description: String,
    },
    InvalidPredicate {
        description: String,
    },

    // Graph errors
    CyclicDependency {
        cycle: Vec<String>,
    },

    // Resolution errors
    UnresolvedImport {
        module: String,
    },
    CircularImport {
        chain: Vec<String>,
    },

    // Parse errors
    ParseError {
        message: String,
    },
}

#[derive(Debug, Default)]
pub struct Diagnostics {
    errors: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn error(&mut self, kind: DiagnosticKind, span: Range<usize>, source_name: &str) {
        self.errors.push(Diagnostic {
            kind,
            span,
            source_name: source_name.to_string(),
        });
    }

    pub fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn merge(&mut self, other: Diagnostics) {
        self.errors.extend(other.errors);
    }

    /// Convert collected diagnostics into miette-renderable `SpinError` values.
    ///
    /// `sources` maps source names (e.g. file paths) to their source text,
    /// enabling miette to display labelled spans within the original source.
    pub fn into_reports(self, sources: &HashMap<String, String>) -> Vec<SpinError> {
        self.errors
            .into_iter()
            .map(|diag| {
                let source_text = sources.get(&diag.source_name).cloned().unwrap_or_default();
                let (message, label, help) = format_diagnostic(&diag.kind);
                SpinError {
                    message,
                    help,
                    src: NamedSource::new(&diag.source_name, source_text),
                    span: (diag.span.start, diag.span.end - diag.span.start).into(),
                    label,
                }
            })
            .collect()
    }
}

/// Produce user-friendly (message, label, help) for a diagnostic kind.
pub fn format_diagnostic(kind: &DiagnosticKind) -> (String, String, Option<String>) {
    match kind {
        DiagnosticKind::TypeMismatch { expected, found } => (
            "type mismatch".to_string(),
            format!("expected {expected}, found {found}"),
            None,
        ),
        DiagnosticKind::UnknownType { name } => (
            format!("unknown type `{name}`"),
            "not found in scope".to_string(),
            Some("did you forget an import?".to_string()),
        ),
        DiagnosticKind::UnknownInterface { name } => (
            format!("unknown interface `{name}`"),
            "not found in scope".to_string(),
            Some("did you forget an import?".to_string()),
        ),
        DiagnosticKind::MissingField { field, interface } => (
            format!("missing field `{field}` in impl for `{interface}`"),
            "required by interface".to_string(),
            Some(
                "add a mapping for this field, or add #[default(...)] to the interface".to_string(),
            ),
        ),
        DiagnosticKind::DuplicateField { field } => (
            format!("duplicate field `{field}`"),
            "already defined".to_string(),
            None,
        ),
        DiagnosticKind::RedefinitionTypeMismatch {
            name,
            expected,
            found,
        } => (
            format!("type mismatch in redefinition of `{name}`"),
            format!("expected {expected}, found {found}"),
            None,
        ),
        DiagnosticKind::InvalidDelegate { reason } => {
            ("invalid delegate".to_string(), reason.clone(), None)
        }
        DiagnosticKind::InvalidAsInterface {
            type_name,
            interface,
        } => (
            format!("`{type_name}` does not implement interface `{interface}`"),
            "interface not satisfied".to_string(),
            None,
        ),
        DiagnosticKind::ConstraintViolation { description } => (
            "constraint violation".to_string(),
            description.clone(),
            None,
        ),
        DiagnosticKind::InvalidPredicate { description } => {
            ("invalid predicate".to_string(), description.clone(), None)
        }
        DiagnosticKind::CyclicDependency { cycle } => (
            "cyclic dependency detected".to_string(),
            format!("cycle involves: {}", cycle.join(", ")),
            None,
        ),
        DiagnosticKind::UnresolvedImport { module } => (
            format!("unresolved import `{module}`"),
            "module not found".to_string(),
            Some("check your SPIN_PATH".to_string()),
        ),
        DiagnosticKind::CircularImport { chain } => (
            "circular import detected".to_string(),
            format!("import chain: {}", chain.join(", ")),
            None,
        ),
        DiagnosticKind::ParseError { message } => (
            "parse error".to_string(),
            message.clone(),
            Some("check the syntax of your .spin file".to_string()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_parse_error_diagnostic() {
        let kind = DiagnosticKind::ParseError {
            message: "expected `;` at position 10".to_string(),
        };
        let (message, label, help) = format_diagnostic(&kind);
        assert_eq!(message, "parse error");
        assert_eq!(label, "expected `;` at position 10");
        assert!(help.is_some());
    }
}
