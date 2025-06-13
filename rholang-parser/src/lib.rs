//! Rholang parser based on tree-sitter grammar
//!
//! This crate provides a parser for the Rholang language using the tree-sitter grammar
//! defined in the rholang-tree-sitter crate.

use anyhow::Result;
use rholang_tree_sitter::LANGUAGE;
use tree_sitter::{Node, Parser};

pub mod errors;
use errors::{ParseResult, ParserError, SourcePosition};

/// A parser for Rholang code
pub struct RholangParser {
    parser: Parser,
}

impl RholangParser {
    /// Create a new instance of the Rholang parser
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();

        // Use the LANGUAGE constant from rholang-tree-sitter
        // Convert it to a type that can be passed to set_language
        let language = LANGUAGE.into();
        parser.set_language(&language)?;
        Ok(RholangParser { parser })
    }

    /// Check if the code is valid Rholang
    pub fn is_valid(&mut self, code: &str) -> bool {
        match self.parser.parse(code, None) {
            Some(tree) => !tree.root_node().has_error(),
            None => false,
        }
    }

    /// Parse Rholang code and return a parse result
    pub fn parse(&mut self, code: &str) -> ParseResult<()> {
        match self.parser.parse(code, None) {
            Some(tree) => {
                let root_node = tree.root_node();
                if root_node.has_error() {
                    // Get error position if possible
                    let position = self.get_error_position(&root_node, code);

                    ParseResult::Error(ParserError::parsing_error(
                        "Invalid Rholang code",
                        position,
                        Some(code.to_string()),
                    ))
                } else {
                    ParseResult::Success(())
                }
            }
            None => ParseResult::Error(ParserError::tree_sitter_error(
                "Failed to parse code",
                None,
                Some(code.to_string()),
            )),
        }
    }

    /// Get a string representation of the parse tree
    pub fn get_tree_string(&mut self, code: &str) -> ParseResult<String> {
        match self.parser.parse(code, None) {
            Some(tree) => {
                let root_node = tree.root_node();
                if root_node.has_error() {
                    // Get error position if possible
                    let position = self.get_error_position(&root_node, code);

                    ParseResult::Error(ParserError::parsing_error(
                        "Parse tree contains errors",
                        position,
                        Some(code.to_string()),
                    ))
                } else {
                    let tree_string = format!("Parse tree: {}", root_node.to_sexp());
                    ParseResult::Success(tree_string)
                }
            }
            None => ParseResult::Error(ParserError::tree_sitter_error(
                "Failed to generate parse tree",
                None,
                Some(code.to_string()),
            )),
        }
    }

    /// Get a pretty-printed string representation of the parse tree
    pub fn get_pretty_tree(&mut self, code: &str) -> ParseResult<String> {
        match self.parser.parse(code, None) {
            Some(tree) => {
                let root_node = tree.root_node();
                if root_node.has_error() {
                    // Get error position if possible
                    let position = self.get_error_position(&root_node, code);

                    ParseResult::Error(ParserError::parsing_error(
                        "Parse tree contains errors",
                        position,
                        Some(code.to_string()),
                    ))
                } else {
                    // Create a more readable tree representation with indentation
                    let pretty_tree = self.format_node_pretty(&root_node, 0, code.as_bytes());
                    ParseResult::Success(pretty_tree)
                }
            }
            None => ParseResult::Error(ParserError::tree_sitter_error(
                "Failed to generate parse tree",
                None,
                Some(code.to_string()),
            )),
        }
    }

    /// Get the position of the first error in the tree
    #[allow(clippy::only_used_in_recursion)]
    fn get_error_position(&self, node: &Node, code: &str) -> Option<SourcePosition> {
        // Find the first error node
        if node.has_error() {
            // If this node is an error, return its position
            if node.is_error() {
                let start_position = node.start_position();
                return Some(SourcePosition {
                    line: start_position.row + 1,      // Convert to 1-based indexing
                    column: start_position.column + 1, // Convert to 1-based indexing
                });
            }

            // Otherwise, search its children
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    let child = cursor.node();
                    if child.has_error() {
                        if let Some(position) = self.get_error_position(&child, code) {
                            return Some(position);
                        }
                    }

                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
        }

        // If no error node was found, return None
        None
    }

    /// Format a node with pretty indentation
    #[allow(clippy::only_used_in_recursion)]
    fn format_node_pretty(&mut self, node: &Node, indent_level: usize, source: &[u8]) -> String {
        let indent = "  ".repeat(indent_level);
        let mut result = format!("{}{}:\n", indent, node.kind());

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                result.push_str(&self.format_node_pretty(&child, indent_level + 1, source));

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        } else if node.child_count() == 0 {
            // For leaf nodes, show the text
            if let Ok(text) = node.utf8_text(source) {
                result.push_str(&format!("{}  text: \"{}\"\n", indent, text));
            } else {
                result.push_str(&format!("{}  text: \"[error getting text]\"\n", indent));
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_new_parser() -> Result<()> {
        let _parser = RholangParser::new()?;
        // Just check that we can create a parser
        Ok(())
    }

    #[test]
    fn test_is_valid() -> Result<()> {
        let mut parser = RholangParser::new()?;

        // Valid Rholang code
        assert!(parser.is_valid("new channel in { @\"stdout\"!(\"Hello, world!\") }"));

        // Invalid Rholang code (missing closing brace)
        assert!(!parser.is_valid("new channel in { @\"stdout\"!(\"Hello, world!\")"));

        Ok(())
    }

    #[rstest]
    #[case("new channel in { @\"stdout\"!(\"Hello, world!\") }", true)]
    #[case("for (msg <- channel) { @\"stdout\"!(msg) }", true)]
    #[case("1 + 2 * 3", true)]
    #[case("new channel in { @\"stdout\"!(\"Hello, world!\") ", false)]
    #[case("for (msg <- channel { @\"stdout\"!(msg) }", false)]
    fn test_is_valid_with_various_inputs(
        #[case] input: &str,
        #[case] expected: bool,
    ) -> Result<()> {
        let mut parser = RholangParser::new()?;
        assert_eq!(parser.is_valid(input), expected);
        Ok(())
    }

    #[test]
    fn test_parse_valid_code() -> Result<()> {
        let mut parser = RholangParser::new()?;
        let code = "new channel in { @\"stdout\"!(\"Hello, world!\") }";

        let result = parser.parse(code);
        assert!(result.is_success());

        Ok(())
    }

    #[test]
    fn test_parse_invalid_code() -> Result<()> {
        let mut parser = RholangParser::new()?;
        let code = "new channel in { @\"stdout\"!(\"Hello, world!\") ";

        let result = parser.parse(code);
        assert!(result.is_error());

        Ok(())
    }

    #[test]
    fn test_get_tree_string() -> Result<()> {
        let mut parser = RholangParser::new()?;
        let code = "new channel in { @\"stdout\"!(\"Hello, world!\") }";

        let result = parser.get_tree_string(code);
        assert!(result.is_success());

        let tree_string = result.unwrap();
        assert!(!tree_string.is_empty());

        Ok(())
    }

    #[test]
    fn test_get_pretty_tree() -> Result<()> {
        let mut parser = RholangParser::new()?;
        let code = "new channel in { @\"stdout\"!(\"Hello, world!\") }";

        let result = parser.get_pretty_tree(code);
        assert!(result.is_success());

        let pretty_tree = result.unwrap();
        assert!(!pretty_tree.is_empty());

        Ok(())
    }
}
