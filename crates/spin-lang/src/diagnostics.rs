use std::ops::Range;

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
}
