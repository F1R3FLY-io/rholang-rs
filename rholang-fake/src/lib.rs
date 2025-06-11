use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

pub mod errors;
pub mod parser;

// Re-export error types for convenience
pub use errors::{ErrorKind, InterpretationResult, InterpreterError, SourcePosition};

/// A simple fake interpreter for Rholang language
/// This is not a real Rholang interpreter, but it uses the RholangParser
/// to validate and parse Rholang code
pub struct FakeRholangInterpreter {
    parser: parser::RholangParser,
    // Store variables for the interpreter
    variables: HashMap<String, String>,
    // Delay for async interpretation (in milliseconds)
    delay_ms: u64,
}

impl FakeRholangInterpreter {
    /// Create a new instance of the fake Rholang interpreter
    /// Default delay for async interpretation is 2 seconds
    pub fn new() -> Result<Self> {
        let parser = parser::RholangParser::new()?;
        Ok(FakeRholangInterpreter {
            parser,
            variables: HashMap::new(),
            delay_ms: 2000, // Default delay: 2 seconds
        })
    }

    /// Set the delay for async interpretation
    pub fn set_delay(&mut self, delay_ms: u64) {
        self.delay_ms = delay_ms;
    }

    /// Interpret a string of Rholang code synchronously
    /// This implementation uses the RholangParser to validate the code
    /// and returns a meaningful result based on the type of Rholang construct
    pub fn interpret(&mut self, code: &str) -> InterpretationResult {
        // Check if the code is valid Rholang
        if !self.parser.is_valid(code) {
            return InterpretationResult::Error(InterpreterError::parsing_error(
                "Invalid Rholang code",
                None,
                Some(code.to_string()),
            ));
        }

        // Trim the code to remove leading/trailing whitespace
        let code = code.trim();

        // Handle different Rholang constructs
        // Check for the more specific constructs first
        if code.starts_with("new ") && code.contains(" in ") {
            self.handle_new_declaration(code)
        } else if code.starts_with("for (") && code.contains("<-") {
            self.handle_for_comprehension(code)
        } else if code.contains("@\"stdout\"!(") {
            self.handle_print(code)
        } else if self.is_arithmetic_expression(code) {
            self.handle_arithmetic(code)
        } else {
            // If no specific handler, return a generic parse tree
            self.parser.get_tree_string(code)
        }
    }

    /// Interpret a string of Rholang code asynchronously
    /// This implementation uses the RholangParser to validate the code
    /// and returns a meaningful result based on the type of Rholang construct
    pub async fn interpret_async(&mut self, code: &str) -> InterpretationResult {
        // Check if the code is valid Rholang
        if !self.parser.is_valid(code) {
            return InterpretationResult::Error(InterpreterError::parsing_error(
                "Invalid Rholang code",
                None,
                Some(code.to_string()),
            ));
        }

        // Trim the code to remove leading/trailing whitespace
        let code = code.trim();

        // Simulate a delay to represent processing time
        // This makes the interpreter run asynchronously
        sleep(Duration::from_millis(self.delay_ms)).await;

        // Handle different Rholang constructs
        // Check for the more specific constructs first
        if code.starts_with("new ") && code.contains(" in ") {
            self.handle_new_declaration(code)
        } else if code.starts_with("for (") && code.contains("<-") {
            self.handle_for_comprehension(code)
        } else if code.contains("@\"stdout\"!(") {
            self.handle_print(code)
        } else if self.is_arithmetic_expression(code) {
            self.handle_arithmetic(code)
        } else {
            // If no specific handler, return a generic parse tree
            self.parser.get_tree_string(code)
        }
    }

    /// Check if the code is valid Rholang
    pub fn is_valid(&mut self, code: &str) -> bool {
        self.parser.is_valid(code)
    }

    /// Handle print statements like @"stdout"!("Hello, world!")
    fn handle_print(&mut self, code: &str) -> InterpretationResult {
        // Extract the message from the print statement
        if let Some(start_idx) = code.find("@\"stdout\"!(") {
            let content_start = start_idx + "@\"stdout\"!(".len();
            let content_end = code[content_start..].rfind(')').map(|i| content_start + i);

            if let Some(end_idx) = content_end {
                let message = &code[content_start..end_idx];

                // Remove quotes if present
                let message = if message.starts_with('"') && message.ends_with('"') {
                    &message[1..message.len() - 1]
                } else {
                    message
                };

                return InterpretationResult::Success(format!("Output: {}", message));
            }
        }

        // Fallback to generic parse tree if we couldn't extract the message
        self.parser.get_tree_string(code)
    }

    /// Handle new declarations like new x in { ... }
    fn handle_new_declaration(&mut self, code: &str) -> InterpretationResult {
        // Extract the name from the new declaration
        if let Some(start_idx) = code.find("new ") {
            let name_start = start_idx + "new ".len();
            let name_end = code[name_start..].find(" in ").map(|i| name_start + i);

            if let Some(end_idx) = name_end {
                let name = &code[name_start..end_idx];

                // Store the name in our variables
                self.variables
                    .insert(name.to_string(), "channel".to_string());

                return InterpretationResult::Success(format!("Created new name: {}", name));
            }
        }

        // Fallback to generic parse tree if we couldn't extract the name
        self.parser.get_tree_string(code)
    }

    /// Handle for comprehensions like for (x <- y) { ... }
    fn handle_for_comprehension(&mut self, code: &str) -> InterpretationResult {
        // Extract the pattern and channel from the for comprehension
        if let Some(start_idx) = code.find("for (") {
            let pattern_start = start_idx + "for (".len();
            let pattern_end = code[pattern_start..].find(")").map(|i| pattern_start + i);

            if let Some(end_idx) = pattern_end {
                let pattern = &code[pattern_start..end_idx];

                if let Some(arrow_idx) = pattern.find("<-") {
                    let var_name = pattern[..arrow_idx].trim();
                    let channel = pattern[arrow_idx + 2..].trim();

                    return InterpretationResult::Success(format!(
                        "Listening for messages on {} as {}",
                        channel, var_name
                    ));
                }
            }
        }

        // Fallback to generic parse tree if we couldn't extract the pattern
        self.parser.get_tree_string(code)
    }

    /// Check if the code is an arithmetic expression
    fn is_arithmetic_expression(&self, code: &str) -> bool {
        // Simple check for arithmetic operators
        code.contains('+') || code.contains('-') || code.contains('*') || code.contains('/')
    }

    /// Handle arithmetic expressions like 1 + 2 * 3
    fn handle_arithmetic(&mut self, code: &str) -> InterpretationResult {
        // This is a very simplified evaluator for arithmetic expressions
        // In a real interpreter, we would use a proper parser and evaluator

        // For this fake interpreter, we'll just return a fake result
        if code.contains('+') {
            InterpretationResult::Success(format!("Addition expression: {}", code))
        } else if code.contains('-') {
            InterpretationResult::Success(format!("Subtraction expression: {}", code))
        } else if code.contains('*') {
            InterpretationResult::Success(format!("Multiplication expression: {}", code))
        } else if code.contains('/') {
            InterpretationResult::Success(format!("Division expression: {}", code))
        } else {
            // Fallback to generic parse tree
            self.parser.get_tree_string(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arithmetic() -> Result<()> {
        let mut interpreter = FakeRholangInterpreter::new()?;

        // Valid arithmetic expressions
        assert!(interpreter.is_valid("1 + 2"));
        assert!(interpreter.is_valid("5 - 3"));
        assert!(interpreter.is_valid("2 * 3"));
        assert!(interpreter.is_valid("6 / 2"));

        // The result should contain the arithmetic expression
        let result = interpreter.interpret("1 + 2");
        match result {
            InterpretationResult::Success(output) => {
                assert!(output.contains("Addition expression: 1 + 2"));
            }
            InterpretationResult::Error(err) => {
                panic!("Expected success, got error: {}", err);
            }
        }

        Ok(())
    }

    #[test]
    fn test_print_statement() -> Result<()> {
        let mut interpreter = FakeRholangInterpreter::new()?;

        let input = "@\"stdout\"!(\"Hello, world!\")";
        assert!(interpreter.is_valid(input));

        // The result should contain the output message
        let result = interpreter.interpret(input);
        match result {
            InterpretationResult::Success(output) => {
                assert!(output.contains("Output: Hello, world!"));
            }
            InterpretationResult::Error(err) => {
                panic!("Expected success, got error: {}", err);
            }
        }

        Ok(())
    }

    #[test]
    fn test_for_comprehension() -> Result<()> {
        let mut interpreter = FakeRholangInterpreter::new()?;

        let input = "for (msg <- channel) { @\"stdout\"!(msg) }";
        assert!(interpreter.is_valid(input));

        // The result should contain the listening message
        let result = interpreter.interpret(input);
        match result {
            InterpretationResult::Success(output) => {
                assert!(output.contains("Listening for messages on channel as msg"));
            }
            InterpretationResult::Error(err) => {
                panic!("Expected success, got error: {}", err);
            }
        }

        Ok(())
    }

    #[test]
    fn test_new_declaration() -> Result<()> {
        let mut interpreter = FakeRholangInterpreter::new()?;

        let input = "new channel in { @\"stdout\"!(\"Using channel\") }";
        assert!(interpreter.is_valid(input));

        // The result should contain the created name
        let result = interpreter.interpret(input);
        match result {
            InterpretationResult::Success(output) => {
                assert!(output.contains("Created new name: channel"));
            }
            InterpretationResult::Error(err) => {
                panic!("Expected success, got error: {}", err);
            }
        }

        Ok(())
    }

    #[test]
    fn test_invalid_code() -> Result<()> {
        let mut interpreter = FakeRholangInterpreter::new()?;

        // Missing closing brace
        let input = "new channel in { @\"stdout\"!(\"Using channel\")";
        assert!(!interpreter.is_valid(input));

        // Interpret should return an error for invalid code
        let result = interpreter.interpret(input);
        assert!(result.is_error());

        Ok(())
    }
}
