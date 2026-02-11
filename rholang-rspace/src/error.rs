//! Execution error types for RSpace operations.

use std::error::Error;
use std::fmt;

/// Execution error that can occur during process execution.
#[derive(Debug)]
pub enum ExecError {
    /// Error related to opcode parameter validation or execution.
    OpcodeParamError {
        opcode: &'static str,
        message: String,
    },
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecError::OpcodeParamError { opcode, message } => {
                write!(f, "{} parameter error: {}", opcode, message)
            }
        }
    }
}

impl Error for ExecError {}
