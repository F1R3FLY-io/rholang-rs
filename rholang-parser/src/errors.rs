/// Error types for the Rholang parser
use std::fmt;

/// Represents the type of error that occurred during parsing
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    /// Error that occurs during parsing of Rholang code
    ParsingError,
    /// Error that occurs when the tree-sitter parser fails
    TreeSitterError,
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

/// Detailed error information for the Rholang parser
#[derive(Debug, Clone)]
pub struct ParserError {
    /// The kind of error that occurred
    pub kind: ErrorKind,
    /// A human-readable error message
    pub message: String,
    /// The position in the source code where the error occurred (if available)
    pub position: Option<SourcePosition>,
    /// The source code that caused the error (if available)
    pub source: Option<String>,
}

impl ParserError {
    /// Create a new parsing error
    pub fn parsing_error(
        message: impl Into<String>,
        position: Option<SourcePosition>,
        source: Option<String>,
    ) -> Self {
        ParserError {
            kind: ErrorKind::ParsingError,
            message: message.into(),
            position,
            source,
        }
    }

    /// Create a new tree-sitter error
    pub fn tree_sitter_error(
        message: impl Into<String>,
        position: Option<SourcePosition>,
        source: Option<String>,
    ) -> Self {
        ParserError {
            kind: ErrorKind::TreeSitterError,
            message: message.into(),
            position,
            source,
        }
    }

    /// Create a new other error
    pub fn other_error(message: impl Into<String>) -> Self {
        ParserError {
            kind: ErrorKind::OtherError,
            message: message.into(),
            position: None,
            source: None,
        }
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::ParsingError => write!(f, "Parsing error: {}", self.message)?,
            ErrorKind::TreeSitterError => write!(f, "Tree-sitter error: {}", self.message)?,
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

impl std::error::Error for ParserError {}

/// Represents the result of a parsing operation
#[derive(Debug, Clone)]
pub enum ParseResult<T> {
    /// Successful parsing with a result value
    Success(T),
    /// Error during parsing
    Error(ParserError),
}

impl<T> ParseResult<T> {
    /// Create a new success result
    pub fn success(result: T) -> Self {
        ParseResult::Success(result)
    }

    /// Create a new error result
    pub fn error(error: ParserError) -> Self {
        ParseResult::Error(error)
    }

    /// Returns true if the result is a success
    pub fn is_success(&self) -> bool {
        matches!(self, ParseResult::Success(_))
    }

    /// Returns true if the result is an error
    pub fn is_error(&self) -> bool {
        matches!(self, ParseResult::Error(_))
    }

    /// Maps a ParseResult<T> to ParseResult<U> by applying a function to the contained Success value
    pub fn map<U, F>(self, f: F) -> ParseResult<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            ParseResult::Success(value) => ParseResult::Success(f(value)),
            ParseResult::Error(err) => ParseResult::Error(err),
        }
    }

    /// Unwraps the success value, panics if the result is an error
    pub fn unwrap(self) -> T {
        match self {
            ParseResult::Success(value) => value,
            ParseResult::Error(err) => panic!("Called unwrap on an error result: {}", err),
        }
    }

    /// Unwraps the error value, panics if the result is a success
    pub fn unwrap_err(self) -> ParserError {
        match self {
            ParseResult::Success(_) => {
                panic!("Called unwrap_err on a success result")
            }
            ParseResult::Error(err) => err,
        }
    }
}

impl<T> From<ParserError> for ParseResult<T> {
    fn from(error: ParserError) -> Self {
        ParseResult::Error(error)
    }
}
