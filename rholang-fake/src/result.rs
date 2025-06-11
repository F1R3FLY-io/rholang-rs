use std::fmt;

/// Represents the result of an interpreter execution
#[derive(Debug, Clone)]
pub enum InterpreterResult {
    /// The interpreter executed successfully and returned a result
    Success(String),
    /// The interpreter encountered a parsing error
    ParsingError {
        message: String,
        line: Option<usize>,
        column: Option<usize>,
    },
    /// The interpreter encountered a runtime error
    RuntimeError {
        message: String,
        details: Option<String>,
    },
}

impl fmt::Display for InterpreterResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InterpreterResult::Success(result) => write!(f, "{}", result),
            InterpreterResult::ParsingError {
                message,
                line,
                column,
            } => {
                if let (Some(line), Some(column)) = (line, column) {
                    write!(f, "Parsing error at line {}, column {}: {}", line, column, message)
                } else {
                    write!(f, "Parsing error: {}", message)
                }
            }
            InterpreterResult::RuntimeError { message, details } => {
                if let Some(details) = details {
                    write!(f, "Runtime error: {} - {}", message, details)
                } else {
                    write!(f, "Runtime error: {}", message)
                }
            }
        }
    }
}

impl InterpreterResult {
    /// Convert the result to a string
    pub fn to_string_result(&self) -> Result<String, String> {
        match self {
            InterpreterResult::Success(result) => Ok(result.clone()),
            InterpreterResult::ParsingError {
                message,
                line,
                column,
            } => {
                if let (Some(line), Some(column)) = (line, column) {
                    Err(format!("Parsing error at line {}, column {}: {}", line, column, message))
                } else {
                    Err(format!("Parsing error: {}", message))
                }
            }
            InterpreterResult::RuntimeError { message, details } => {
                if let Some(details) = details {
                    Err(format!("Runtime error: {} - {}", message, details))
                } else {
                    Err(format!("Runtime error: {}", message))
                }
            }
        }
    }
}