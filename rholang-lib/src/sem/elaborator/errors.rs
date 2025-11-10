use crate::sem::{Diagnostic, ErrorKind, PID, Symbol};
use rholang_parser::SourcePos;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ElaborationError {
    /// AST node is incomplete or invalid
    IncompleteAstNode {
        pid: PID,
        position: Option<SourcePos>,
        reason: String,
    },
    /// PID not found in semantic db
    InvalidPid { pid: PID },
    /// Unbound variable reference
    UnboundVariable {
        pid: PID,
        var: Symbol,
        pos: SourcePos,
    },
    /// Connective used outside pattern context
    ConnectiveOutsidePattern { pid: PID, pos: SourcePos },
    /// Pattern cannot be satisfied
    UnsatisfiablePattern {
        pid: PID,
        pattern: String,
        pos: SourcePos,
    },
    /// Deadlock potential
    DeadlockPotential {
        pid: PID,
        receipts: String,
        pos: SourcePos,
    },
    /// Invalid pattern structure in for-comp
    InvalidPattern {
        pid: PID,
        position: Option<SourcePos>,
        reason: String,
    },
}

pub type ElaborationResult<T> = Result<T, ElaborationError>;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    /// Pattern contains invalid structure
    InvalidPatternStructure {
        pid: PID,
        position: Option<SourcePos>,
        reason: String,
    },
    /// Unbound variable reference
    UnboundVariable { var: Symbol, pos: SourcePos },
    /// Connective used outside pattern context
    ConnectiveOutsidePattern { pos: SourcePos },
    /// Pattern cannot be satisfied
    UnsatisfiablePattern {
        pattern: String,
        pos: Option<SourcePos>,
    },
    /// Deadlock potential
    DeadlockPotential {
        receipts: String,
        pos: Option<SourcePos>,
    },
}

// Conversion traits for integrating with SemanticDb diagnostics

impl ElaborationError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic::from(self.clone())
    }
}

impl From<ElaborationError> for Diagnostic {
    fn from(error: ElaborationError) -> Self {
        match error {
            ElaborationError::IncompleteAstNode { pid, position, .. } => {
                Diagnostic::error(pid, ErrorKind::BadCode, position)
            }
            ElaborationError::InvalidPid { pid } => {
                Diagnostic::error(pid, ErrorKind::BadCode, None)
            }
            ElaborationError::UnboundVariable { pid, pos, .. } => {
                Diagnostic::error(pid, ErrorKind::UnboundVariable, Some(pos))
            }
            ElaborationError::ConnectiveOutsidePattern { pid, pos } => {
                Diagnostic::error(pid, ErrorKind::ConnectiveOutsidePattern, Some(pos))
            }
            ElaborationError::UnsatisfiablePattern { pid, pos, .. } => {
                Diagnostic::error(pid, ErrorKind::BadCode, Some(pos))
            }
            ElaborationError::DeadlockPotential { pid, pos, .. } => {
                Diagnostic::error(pid, ErrorKind::BadCode, Some(pos))
            }
            ElaborationError::InvalidPattern { pid, position, .. } => {
                Diagnostic::error(pid, ErrorKind::BadCode, position)
            }
        }
    }
}

impl fmt::Display for ElaborationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElaborationError::IncompleteAstNode { reason, .. } => {
                write!(f, "Incomplete AST node: {}", reason)
            }
            ElaborationError::InvalidPid { pid } => {
                write!(f, "Invalid PID: {}", pid)
            }
            ElaborationError::UnboundVariable { .. } => {
                write!(f, "Unbound variable")
            }
            ElaborationError::ConnectiveOutsidePattern { .. } => {
                write!(f, "Connective used outside pattern")
            }
            ElaborationError::UnsatisfiablePattern { pattern, .. } => {
                write!(f, "Unsatisfiable pattern: {}", pattern)
            }
            ElaborationError::DeadlockPotential { receipts, .. } => {
                write!(f, "Deadlock potential: {}", receipts)
            }
            ElaborationError::InvalidPattern { reason, .. } => {
                write!(f, "Invalid pattern: {}", reason)
            }
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::InvalidPatternStructure { reason, .. } => {
                write!(f, "Invalid pattern structure: {}", reason)
            }
            ValidationError::UnboundVariable { var, .. } => {
                write!(f, "Unbound variable: {:?}", var)
            }
            ValidationError::ConnectiveOutsidePattern { pos } => {
                write!(f, "Connective used outside pattern at {:?}", pos)
            }
            ValidationError::UnsatisfiablePattern { pattern, .. } => {
                write!(f, "Unsatisfiable pattern: {}", pattern)
            }
            ValidationError::DeadlockPotential { receipts, .. } => {
                write!(f, "Deadlock potential: {}", receipts)
            }
        }
    }
}

impl std::error::Error for ElaborationError {}
impl std::error::Error for ValidationError {}

/// Result type for validation operations
pub type ValidationResult<T = ()> = Result<T, ValidationError>;
