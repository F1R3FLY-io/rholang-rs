use super::ConsumptionMode;
use crate::sem::{BinderId, Diagnostic, ErrorKind, PID, Symbol, SymbolOccurence, WarningKind};
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
    /// Parent context is not available when required
    MissingParentContext {
        pid: PID,
        position: Option<SourcePos>,
    },
    /// PID not found in semantic db
    InvalidPid { pid: PID },
    /// Child nodes are not properly indexed
    UnindexedChildNodes {
        pid: PID,
        missing_children: Vec<String>,
    },
    /// Unbound variable reference
    UnboundVariable {
        pid: PID,
        var: Symbol,
        pos: SourcePos,
    },
    /// Duplicate variable definition
    DuplicateVarDef {
        pid: PID,
        original: SymbolOccurence,
        duplicate: SymbolOccurence,
    },
    /// Name used in process position
    NameInProcPosition {
        pid: PID,
        binder: BinderId,
        symbol: Symbol,
        pos: Option<SourcePos>,
    },
    /// Process used in name position
    ProcInNamePosition {
        pid: PID,
        binder: BinderId,
        symbol: Symbol,
        pos: Option<SourcePos>,
    },
    /// Connective used outside pattern context
    ConnectiveOutsidePattern { pid: PID, pos: SourcePos },
    /// Pattern type mismatch
    PatternTypeMismatch {
        pid: PID,
        expected: String,
        found: String,
        pos: SourcePos,
    },
    /// Circular channel dependency
    CircularChannelDependency {
        pid: PID,
        channels: Vec<Symbol>,
        pos: Option<SourcePos>,
    },
    /// Pattern cannot be satisfied
    UnsatisfiablePattern {
        pid: PID,
        pattern: String,
        pos: SourcePos,
    },
    /// Invalid consumption mode
    InvalidConsumptionMode {
        pid: PID,
        expected: ConsumptionMode,
        found: ConsumptionMode,
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
    /// Conflicting binder types in the same pattern
    ConflictingBinderTypes {
        pid: PID,
        position: Option<SourcePos>,
        first_occurrence: SymbolOccurence,
        second_occurrence: SymbolOccurence,
    },
    /// Pattern contains unreachable branches
    UnreachablePattern {
        pid: PID,
        position: Option<SourcePos>,
    },
    /// Invalid channel reference in source expression
    InvalidChannelReference {
        pid: PID,
        position: Option<SourcePos>,
        symbol: Symbol,
    },
    /// Contradictory pattern constraints
    ContradictoryConstraints {
        pid: PID,
        position: Option<SourcePos>,
        constraint1: String,
        constraint2: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElaborationWarning {
    /// Pattern may not match expected message type
    PatternTypeMismatch {
        pid: PID,
        position: Option<SourcePos>,
        expected: String,
        actual: String,
    },
    /// Unused pattern variable
    UnusedPatternVariable {
        pid: PID,
        position: Option<SourcePos>,
        symbol: crate::sem::Symbol,
    },
    /// Potentially inefficient pattern structure
    InefficiientPattern {
        pid: PID,
        position: Option<SourcePos>,
        suggestion: String,
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
    /// Duplicate variable definition
    DuplicateVarDef {
        original: SymbolOccurence,
        duplicate: SymbolOccurence,
    },
    /// Name used in process position
    NameInProcPosition { binder: BinderId, symbol: Symbol },
    /// Process used in name position
    ProcInNamePosition { binder: BinderId, symbol: Symbol },
    /// Connective used outside pattern context
    ConnectiveOutsidePattern { pos: SourcePos },
    /// Circular channel dependency
    CircularChannelDependency { channels: Vec<Symbol> },
    /// Pattern cannot be satisfied
    UnsatisfiablePattern { pattern: String },
    /// Deadlock potential
    DeadlockPotential { receipts: String },
}

// Conversion traits for integrating with SemanticDb diagnostics

impl ElaborationError {
    /// Convert to a semantic database diagnostic
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            ElaborationError::IncompleteAstNode { pid, position, .. } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, *position)
            }
            ElaborationError::MissingParentContext { pid, position } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, *position)
            }
            ElaborationError::InvalidPid { pid } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, None)
            }
            ElaborationError::UnindexedChildNodes { pid, .. } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, None)
            }
            ElaborationError::UnboundVariable { pid, pos, .. } => {
                Diagnostic::error(*pid, ErrorKind::UnboundVariable, Some(*pos))
            }
            ElaborationError::DuplicateVarDef { pid, original, .. } => Diagnostic::error(
                *pid,
                ErrorKind::DuplicateVarDef {
                    original: *original,
                },
                Some(original.position),
            ),
            ElaborationError::NameInProcPosition {
                pid,
                binder,
                symbol,
                pos,
            } => Diagnostic::error(*pid, ErrorKind::NameInProcPosition(*binder, *symbol), *pos),
            ElaborationError::ProcInNamePosition {
                pid,
                binder,
                symbol,
                pos,
            } => Diagnostic::error(*pid, ErrorKind::ProcInNamePosition(*binder, *symbol), *pos),
            ElaborationError::ConnectiveOutsidePattern { pid, pos } => {
                Diagnostic::error(*pid, ErrorKind::ConnectiveOutsidePattern, Some(*pos))
            }
            ElaborationError::PatternTypeMismatch { pid, pos, .. } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, Some(*pos))
            }
            ElaborationError::CircularChannelDependency { pid, pos, .. } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, *pos)
            }
            ElaborationError::UnsatisfiablePattern { pid, pos, .. } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, Some(*pos))
            }
            ElaborationError::InvalidConsumptionMode { pid, pos, .. } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, Some(*pos))
            }
            ElaborationError::DeadlockPotential { pid, pos, .. } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, Some(*pos))
            }
            ElaborationError::InvalidPattern { pid, position, .. } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, *position)
            }
            ElaborationError::ConflictingBinderTypes {
                pid,
                position,
                first_occurrence,
                ..
            } => Diagnostic::error(
                *pid,
                ErrorKind::DuplicateVarDef {
                    original: *first_occurrence,
                },
                *position,
            ),
            ElaborationError::UnreachablePattern { pid, position } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, *position)
            }
            ElaborationError::InvalidChannelReference { pid, position, .. } => {
                Diagnostic::error(*pid, ErrorKind::UnboundVariable, *position)
            }
            ElaborationError::ContradictoryConstraints { pid, position, .. } => {
                Diagnostic::error(*pid, ErrorKind::BadCode, *position)
            }
        }
    }
}

impl ElaborationWarning {
    /// Convert to a semantic database diagnostic
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            ElaborationWarning::PatternTypeMismatch { pid, position, .. } => {
                Diagnostic::warning(*pid, WarningKind::UnusedVariable, *position)
            }
            ElaborationWarning::UnusedPatternVariable { pid, position, .. } => {
                Diagnostic::warning(*pid, WarningKind::UnusedVariable, *position)
            }
            ElaborationWarning::InefficiientPattern { pid, position, .. } => {
                Diagnostic::warning(*pid, WarningKind::UnusedVariable, *position)
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
            ElaborationError::MissingParentContext { .. } => {
                write!(f, "Missing parent context")
            }
            ElaborationError::InvalidPid { pid } => {
                write!(f, "Invalid PID: {}", pid)
            }
            ElaborationError::UnindexedChildNodes {
                missing_children, ..
            } => {
                write!(f, "Unindexed child nodes: {}", missing_children.join(", "))
            }
            ElaborationError::UnboundVariable { .. } => {
                write!(f, "Unbound variable")
            }
            ElaborationError::DuplicateVarDef { .. } => {
                write!(f, "Duplicate variable definition")
            }
            ElaborationError::NameInProcPosition { .. } => {
                write!(f, "Name used in process position")
            }
            ElaborationError::ProcInNamePosition { .. } => {
                write!(f, "Process used in name position")
            }
            ElaborationError::ConnectiveOutsidePattern { .. } => {
                write!(f, "Connective used outside pattern")
            }
            ElaborationError::PatternTypeMismatch {
                expected, found, ..
            } => {
                write!(
                    f,
                    "Pattern type mismatch: expected {}, found {}",
                    expected, found
                )
            }
            ElaborationError::CircularChannelDependency { .. } => {
                write!(f, "Circular channel dependency")
            }
            ElaborationError::UnsatisfiablePattern { pattern, .. } => {
                write!(f, "Unsatisfiable pattern: {}", pattern)
            }
            ElaborationError::InvalidConsumptionMode {
                expected, found, ..
            } => {
                write!(
                    f,
                    "Invalid consumption mode: expected {:?}, found {:?}",
                    expected, found
                )
            }
            ElaborationError::DeadlockPotential { receipts, .. } => {
                write!(f, "Deadlock potential: {}", receipts)
            }
            ElaborationError::InvalidPattern { reason, .. } => {
                write!(f, "Invalid pattern: {}", reason)
            }
            ElaborationError::ConflictingBinderTypes { .. } => {
                write!(f, "Conflicting binder types in pattern")
            }
            ElaborationError::UnreachablePattern { .. } => {
                write!(f, "Unreachable pattern detected")
            }
            ElaborationError::InvalidChannelReference { .. } => {
                write!(f, "Invalid channel reference")
            }
            ElaborationError::ContradictoryConstraints {
                constraint1,
                constraint2,
                ..
            } => {
                write!(
                    f,
                    "Contradictory constraints: {} vs {}",
                    constraint1, constraint2
                )
            }
        }
    }
}

impl fmt::Display for ElaborationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ElaborationWarning::PatternTypeMismatch {
                expected, actual, ..
            } => {
                write!(
                    f,
                    "Pattern type mismatch: expected {}, got {}",
                    expected, actual
                )
            }
            ElaborationWarning::UnusedPatternVariable { .. } => {
                write!(f, "Unused pattern variable")
            }
            ElaborationWarning::InefficiientPattern { suggestion, .. } => {
                write!(f, "Inefficient pattern: {}", suggestion)
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
            ValidationError::UnboundVariable { .. } => {
                write!(f, "Unbound variable")
            }
            ValidationError::DuplicateVarDef { .. } => {
                write!(f, "Duplicate variable definition")
            }
            ValidationError::NameInProcPosition { .. } => {
                write!(f, "Name used in process position")
            }
            ValidationError::ProcInNamePosition { .. } => {
                write!(f, "Process used in name position")
            }
            ValidationError::ConnectiveOutsidePattern { .. } => {
                write!(f, "Connective used outside pattern")
            }
            ValidationError::CircularChannelDependency { .. } => {
                write!(f, "Circular channel dependency")
            }
            ValidationError::UnsatisfiablePattern { pattern } => {
                write!(f, "Unsatisfiable pattern: {}", pattern)
            }
            ValidationError::DeadlockPotential { receipts } => {
                write!(f, "Deadlock potential: {}", receipts)
            }
        }
    }
}

impl std::error::Error for ElaborationError {}
impl std::error::Error for ElaborationWarning {}
impl std::error::Error for ValidationError {}

/// Result type for validation operations
pub type ValidationResult<T = ()> = Result<T, ValidationError>;
