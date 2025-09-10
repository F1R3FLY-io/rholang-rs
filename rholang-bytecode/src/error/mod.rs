//! Error types for bytecode operations

use thiserror::Error;

/// Main error type for bytecode operations
#[derive(Error, Debug)]
pub enum BytecodeError {
    #[error("Invalid opcode: {0:#04x}")]
    InvalidOpcode(u8),

    #[error("Invalid instruction format at offset {offset:#x}")]
    InvalidInstruction { offset: usize },

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Reference counting error: {0}")]
    ReferenceError(String),

    #[error("Memory mapping failed: {0}")]
    MemoryMapError(#[from] std::io::Error),

    #[error("String interning error: {0}")]
    InternError(String),

    #[error("Pattern compilation failed: {0}")]
    PatternError(String),

    #[error("RSpace type mismatch: {0}")]
    RSpaceError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    #[error("Invalid module: {0}")]
    InvalidModule(String),

    #[error("Incompatible version: expected {expected}, found {found}")]
    IncompatibleVersion { expected: u16, found: u16 },

    #[error("Invalid constant index {index} for {pool_type} pool")]
    InvalidConstantIndex { index: u32, pool_type: String },

    #[error("Invalid RSpace type: {0}")]
    InvalidRSpaceType(u8),
}

/// Convenient Result type
pub type Result<T> = std::result::Result<T, BytecodeError>;

/// Validation error details
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub offset: usize,
    pub instruction_index: usize,
    pub message: String,
}
