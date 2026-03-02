use librho::sem::{Diagnostic, DiagnosticKind, ErrorKind, SemanticDb, PID};
use rholang_parser::{SourcePos, SourceSpan};
use thiserror::Error;

pub type CompileResult<T> = Result<T, CompileError>;

/// A single compilation error with source location and context
#[derive(Debug, Clone)]
pub struct CompileErrorInfo {
    /// Error message
    pub message: String,
    /// Source position (single point) - for backward compatibility
    pub position: Option<SourcePos>,
    /// Source span (range) - preferred for better error highlighting
    pub span: Option<SourceSpan>,
    /// The error kind for programmatic handling
    pub kind: ErrorKind,
    /// Process ID for additional context lookup
    pub pid: PID,
}

/// Compilation errors
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Semantic analysis failed")]
    SemanticErrors(Vec<CompileErrorInfo>),

    #[error("Code generation failed: {0}")]
    CodegenError(#[from] anyhow::Error),

    #[error("Internal compiler error: {0}")]
    InternalError(String),
}

impl CompileError {
    /// Create from semantic database diagnostics
    pub fn from_diagnostics(db: &SemanticDb, diagnostics: &[Diagnostic]) -> Self {
        let errors: Vec<CompileErrorInfo> = diagnostics
            .iter()
            .filter_map(|d| match &d.kind {
                DiagnosticKind::Error(kind) => Some(CompileErrorInfo {
                    message: format_error_kind(db, kind, d.pid),
                    position: d.exact_position,
                    span: None, // TODO: Extract span from diagnostic if available
                    kind: *kind,
                    pid: d.pid,
                }),
                _ => None,
            })
            .collect();

        CompileError::SemanticErrors(errors)
    }

    pub fn error_count(&self) -> usize {
        match self {
            CompileError::SemanticErrors(errors) => errors.len(),
            _ => 1,
        }
    }

    /// Filter out recoverable errors
    /// Returns None if all errors were recoverable
    pub fn filter_recoverable(self) -> Option<Self> {
        match self {
            CompileError::SemanticErrors(errors) => {
                let filtered: Vec<_> = errors
                    .into_iter()
                    .filter(|e| !e.kind.is_recoverable())
                    .collect();

                if filtered.is_empty() {
                    None
                } else {
                    Some(CompileError::SemanticErrors(filtered))
                }
            }
            other => Some(other),
        }
    }
}

/// Format an ErrorKind into a readable message
fn format_error_kind(db: &SemanticDb, kind: &ErrorKind, _pid: PID) -> String {
    match kind {
        ErrorKind::UnboundVariable => "Use of undeclared variable".to_string(),
        ErrorKind::DuplicateVarDef { original } => {
            let name = db
                .resolve_symbol_owned(original.symbol)
                .unwrap_or_else(|| format!("<symbol#{}>", original.symbol));
            format!("Duplicate definition of variable '{}'", name)
        }
        ErrorKind::NameInProcPosition(_binder_id, sym) => {
            let name = db
                .resolve_symbol_owned(*sym)
                .unwrap_or_else(|| format!("<symbol#{}>", sym));
            format!(
                "Name '{}' used in process position (will be dereferenced)",
                name
            )
        }
        ErrorKind::ProcInNamePosition(_binder_id, sym) => {
            let name = db
                .resolve_symbol_owned(*sym)
                .unwrap_or_else(|| format!("<symbol#{}>", sym));
            format!(
                "Process variable '{}' used in name position (expected a channel)",
                name
            )
        }
        ErrorKind::ConnectiveOutsidePattern => {
            "Logical connective (/\\, \\/, ~) used outside of a pattern context".to_string()
        }
        ErrorKind::BundleInsidePattern => {
            "Bundle expression not allowed inside a pattern".to_string()
        }
        ErrorKind::UnmatchedVarInDisjunction(sym) => {
            let name = db
                .resolve_symbol_owned(*sym)
                .unwrap_or_else(|| format!("<symbol#{}>", sym));
            format!(
                "Variable '{}' not matched in all branches of disjunction pattern",
                name
            )
        }
        ErrorKind::FreeVariable(occ) => {
            let name = db
                .resolve_symbol_owned(occ.symbol)
                .unwrap_or_else(|| format!("<symbol#{}>", occ.symbol));
            format!("Free variable '{}' in non-pattern context", name)
        }
        ErrorKind::BadCode => "Invalid code structure".to_string(),
    }
}
