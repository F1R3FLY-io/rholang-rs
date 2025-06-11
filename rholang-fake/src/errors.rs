/// Error types for the Rholang fake interpreter
use std::fmt;

/// Represents the type of error that occurred during interpretation
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// Error that occurs during parsing of Rholang code
    ParsingError,
    /// Error that occurs during runtime execution of Rholang code
    RuntimeError,
    /// Error that occurs when a timeout is reached
    TimeoutError,
    /// Error that occurs when an operation is cancelled
    CancellationError,
    /// Other unspecified errors
    OtherError,
}

/// Represents a position in the source code
#[derive(Debug, Clone, PartialEq)]
pub struct SourcePosition {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

/// Detailed error information for the Rholang interpreter
#[derive(Debug, Clone)]
pub struct InterpreterError {
    /// The kind of error that occurred
    pub kind: ErrorKind,
    /// A human-readable error message
    pub message: String,
    /// The position in the source code where the error occurred (if available)
    pub position: Option<SourcePosition>,
    /// The source code that caused the error (if available)
    pub source: Option<String>,
}

impl InterpreterError {
    /// Create a new parsing error
    pub fn parsing_error(
        message: impl Into<String>,
        position: Option<SourcePosition>,
        source: Option<String>,
    ) -> Self {
        InterpreterError {
            kind: ErrorKind::ParsingError,
            message: message.into(),
            position,
            source,
        }
    }

    /// Create a new runtime error
    pub fn runtime_error(
        message: impl Into<String>,
        position: Option<SourcePosition>,
        source: Option<String>,
    ) -> Self {
        InterpreterError {
            kind: ErrorKind::RuntimeError,
            message: message.into(),
            position,
            source,
        }
    }

    /// Create a new timeout error
    pub fn timeout_error(message: impl Into<String>) -> Self {
        InterpreterError {
            kind: ErrorKind::TimeoutError,
            message: message.into(),
            position: None,
            source: None,
        }
    }

    /// Create a new cancellation error
    pub fn cancellation_error(message: impl Into<String>) -> Self {
        InterpreterError {
            kind: ErrorKind::CancellationError,
            message: message.into(),
            position: None,
            source: None,
        }
    }

    /// Create a new other error
    pub fn other_error(message: impl Into<String>) -> Self {
        InterpreterError {
            kind: ErrorKind::OtherError,
            message: message.into(),
            position: None,
            source: None,
        }
    }
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::ParsingError => write!(f, "Parsing error: {}", self.message)?,
            ErrorKind::RuntimeError => write!(f, "Runtime error: {}", self.message)?,
            ErrorKind::TimeoutError => write!(f, "Timeout error: {}", self.message)?,
            ErrorKind::CancellationError => write!(f, "Cancellation error: {}", self.message)?,
            ErrorKind::OtherError => write!(f, "Error: {}", self.message)?,
        }

        if let Some(position) = &self.position {
            write!(f, " at {}", position)?;
        }

        if let Some(source) = &self.source {
            write!(f, "\nSource: {}", source)?;
        }

        Ok(())
    }
}

impl std::error::Error for InterpreterError {}

/// Represents the result of an interpretation
#[derive(Debug, Clone)]
pub enum InterpretationResult {
    /// Successful interpretation with a result value
    Success(String),
    /// Error during interpretation
    Error(InterpreterError),
}

impl InterpretationResult {
    /// Create a new success result
    pub fn success(result: impl Into<String>) -> Self {
        InterpretationResult::Success(result.into())
    }

    /// Create a new error result
    pub fn error(error: InterpreterError) -> Self {
        InterpretationResult::Error(error)
    }

    /// Returns true if the result is a success
    pub fn is_success(&self) -> bool {
        matches!(self, InterpretationResult::Success(_))
    }

    /// Returns true if the result is an error
    pub fn is_error(&self) -> bool {
        matches!(self, InterpretationResult::Error(_))
    }

    /// Unwraps the success value, panics if the result is an error
    pub fn unwrap(self) -> String {
        match self {
            InterpretationResult::Success(value) => value,
            InterpretationResult::Error(err) => panic!("Called unwrap on an error result: {}", err),
        }
    }

    /// Unwraps the error value, panics if the result is a success
    pub fn unwrap_err(self) -> InterpreterError {
        match self {
            InterpretationResult::Success(value) => {
                panic!("Called unwrap_err on a success result: {}", value)
            }
            InterpretationResult::Error(err) => err,
        }
    }
}

impl From<InterpreterError> for InterpretationResult {
    fn from(error: InterpreterError) -> Self {
        InterpretationResult::Error(error)
    }
}

impl From<String> for InterpretationResult {
    fn from(value: String) -> Self {
        InterpretationResult::Success(value)
    }
}

impl From<&str> for InterpretationResult {
    fn from(value: &str) -> Self {
        InterpretationResult::Success(value.to_string())
    }
}
