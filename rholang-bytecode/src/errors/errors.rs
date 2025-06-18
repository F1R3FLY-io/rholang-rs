use thiserror::Error;

/// Main error types for bytecode operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum BytecodeError {
    #[error("Invalid instruction: {0}")]
    InvalidInstruction(String),

    #[error("Constant pool error: {0}")]
    ConstantPool(String),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Index out of bounds: {index} >= {max}")]
    IndexOutOfBounds { index: usize, max: usize },

    #[error("Other error: {0}")]
    Other(String),
}

pub type BytecodeResult<T> = Result<T, BytecodeError>;
