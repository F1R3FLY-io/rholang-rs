//! Error types for for-comprehension elaboration
//!
//! ## Error Hierarchy
//!
//! - **ElaborationError**: Top-level errors during elaboration orchestration
//! - **ValidationError**: Specific validation failures (arrow type homogeneity)

use crate::sem::{Diagnostic, ErrorKind, PID};
use rholang_parser::SourcePos;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ElaborationError {
    /// PID not found in semantic db
    InvalidPid { pid: PID },

    /// AST node is incomplete or invalid
    IncompleteAstNode {
        pid: PID,
        position: Option<SourcePos>,
        reason: String,
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
    /// Mixed arrow types in concurrent join group
    MixedArrowTypes {
        receipt_index: usize,
        found_types: Vec<&'static str>,
        pos: Option<SourcePos>,
    },
}

pub type ValidationResult<T = ()> = Result<T, ValidationError>;

impl fmt::Display for ElaborationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElaborationError::InvalidPid { pid } => {
                write!(f, "Invalid PID: {}", pid)
            }
            ElaborationError::IncompleteAstNode { reason, .. } => {
                write!(f, "Incomplete AST node: {}", reason)
            }
            ElaborationError::InvalidPattern { reason, .. } => {
                write!(f, "Invalid pattern: {}", reason)
            }
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::MixedArrowTypes {
                receipt_index,
                found_types,
                ..
            } => {
                write!(
                    f,
                    "Mixed arrow types in concurrent join group {}: found {}. \
                     All bindings in a concurrent join (separated by &) must use the same arrow type: \
                     all linear (<-), all repeated (<=), or all peek (<<-).",
                    receipt_index,
                    found_types.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for ElaborationError {}
impl std::error::Error for ValidationError {}

// Conversion to SemanticDb diagnostics
impl ElaborationError {
    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic::from(self.clone())
    }
}

impl From<ElaborationError> for Diagnostic {
    fn from(error: ElaborationError) -> Self {
        match error {
            ElaborationError::InvalidPid { pid } => {
                Diagnostic::error(pid, ErrorKind::BadCode, None)
            }
            ElaborationError::IncompleteAstNode { pid, position, .. } => {
                Diagnostic::error(pid, ErrorKind::BadCode, position)
            }
            ElaborationError::InvalidPattern { pid, position, .. } => {
                Diagnostic::error(pid, ErrorKind::BadCode, position)
            }
        }
    }
}
