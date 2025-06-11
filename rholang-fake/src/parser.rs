use crate::errors::{InterpretationResult, InterpreterError};
use anyhow::anyhow;
use anyhow::Result;

/// A simple parser for Rholang code
pub struct RholangParser;

impl RholangParser {
    /// Create a new instance of the Rholang parser
    pub fn new() -> Result<Self> {
        Ok(RholangParser)
    }

    /// Check if the code is valid Rholang
    /// This is a very simple implementation that just checks for balanced braces, parentheses, and brackets
    pub fn is_valid(&mut self, code: &str) -> bool {
        // Check for balanced braces, parentheses, and brackets
        let mut brace_count = 0;
        let mut paren_count = 0;
        let mut bracket_count = 0;

        for c in code.chars() {
            match c {
                '{' => brace_count += 1,
                '}' => brace_count -= 1,
                '(' => paren_count += 1,
                ')' => paren_count -= 1,
                '[' => bracket_count += 1,
                ']' => bracket_count -= 1,
                _ => {}
            }

            // If any count goes negative, the code is invalid
            if brace_count < 0 || paren_count < 0 || bracket_count < 0 {
                return false;
            }
        }

        // If all counts are 0, the code is valid
        brace_count == 0 && paren_count == 0 && bracket_count == 0
    }

    /// Get a string representation of the parse tree
    /// This is a very simple implementation that just returns the input code
    pub fn get_tree_string(&mut self, code: &str) -> InterpretationResult {
        if self.is_valid(code) {
            InterpretationResult::Success(format!("Parse tree: {}", code))
        } else {
            InterpretationResult::Error(InterpreterError::parsing_error(
                "Parse tree contains errors",
                None,
                Some(code.to_string()),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid() -> Result<()> {
        let mut parser = RholangParser::new()?;

        // Valid Rholang code
        assert!(parser.is_valid("new channel in { @\"stdout\"!(\"Hello, world!\") }"));

        // Invalid Rholang code (missing closing brace)
        assert!(!parser.is_valid("new channel in { @\"stdout\"!(\"Hello, world!\")"));

        Ok(())
    }

    #[test]
    fn test_get_tree_string() -> Result<()> {
        let mut parser = RholangParser::new()?;
        let code = "new channel in { @\"stdout\"!(\"Hello, world!\") }";

        // Test the InterpretationResult-based method
        let result = parser.get_tree_string(code);
        match result {
            InterpretationResult::Success(tree_string) => {
                assert!(tree_string.starts_with("Parse tree:"));
            }
            InterpretationResult::Error(err) => {
                panic!("Expected success, got error: {}", err);
            }
        }

        Ok(())
    }
}
