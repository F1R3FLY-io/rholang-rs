use anyhow::Result;
use rholang_parser::{errors::ParseResult, RholangParser};
use rstest::rstest;

#[test]
fn test_new_parser() -> Result<()> {
    let _parser = RholangParser::new()?;
    Ok(())
}

#[test]
fn test_is_valid_with_valid_code() -> Result<()> {
    let mut parser = RholangParser::new()?;
    let code = "new channel in { @\"stdout\"!(\"Hello, world!\") }";
    assert!(parser.is_valid(code));
    Ok(())
}

#[test]
fn test_is_valid_with_invalid_code() -> Result<()> {
    let mut parser = RholangParser::new()?;
    let code = "new channel in { @\"stdout\"!(\"Hello, world!\") "; // Missing closing brace
    assert!(!parser.is_valid(code));
    Ok(())
}

#[rstest]
#[case("new channel in { @\"stdout\"!(\"Hello, world!\") }", true)]
#[case("for (msg <- channel) { @\"stdout\"!(msg) }", true)]
#[case("1 + 2 * 3", true)]
#[case("new channel in { @\"stdout\"!(\"Hello, world!\") ", false)]
#[case("for (msg <- channel { @\"stdout\"!(msg) }", false)]
fn test_is_valid_with_various_inputs(#[case] input: &str, #[case] expected: bool) -> Result<()> {
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
    let code = "new channel in { @\"stdout\"!(\"Hello, world!\") "; // Missing closing brace

    let result = parser.parse(code);
    assert!(result.is_error());

    Ok(())
}

#[test]
fn test_get_tree_string_with_valid_code() -> Result<()> {
    let mut parser = RholangParser::new()?;
    let code = "new channel in { @\"stdout\"!(\"Hello, world!\") }";

    let result = parser.get_tree_string(code);
    assert!(result.is_success());

    let tree_string = match result {
        ParseResult::Success(s) => s,
        ParseResult::Error(e) => panic!("Expected success, got error: {}", e),
    };

    assert!(!tree_string.is_empty());
    assert!(tree_string.contains("Parse tree:"));

    Ok(())
}

#[test]
fn test_get_tree_string_with_invalid_code() -> Result<()> {
    let mut parser = RholangParser::new()?;
    let code = "new channel in { @\"stdout\"!(\"Hello, world!\") "; // Missing closing brace

    let result = parser.get_tree_string(code);
    assert!(result.is_error());

    Ok(())
}

#[test]
fn test_get_pretty_tree_with_valid_code() -> Result<()> {
    let mut parser = RholangParser::new()?;
    let code = "new channel in { @\"stdout\"!(\"Hello, world!\") }";

    let result = parser.get_pretty_tree(code);
    assert!(result.is_success());

    let pretty_tree = match result {
        ParseResult::Success(s) => s,
        ParseResult::Error(e) => panic!("Expected success, got error: {}", e),
    };

    assert!(!pretty_tree.is_empty());

    Ok(())
}

#[test]
fn test_get_pretty_tree_with_invalid_code() -> Result<()> {
    let mut parser = RholangParser::new()?;
    let code = "new channel in { @\"stdout\"!(\"Hello, world!\") "; // Missing closing brace

    let result = parser.get_pretty_tree(code);
    assert!(result.is_error());

    Ok(())
}

// Test with different types of Rholang constructs
#[rstest]
#[case("new channel in { @\"stdout\"!(\"Hello, world!\") }", true)]
#[case("for (msg <- channel) { @\"stdout\"!(msg) }", true)]
#[case("1 + 2 * 3", true)]
#[case("@\"stdout\"!(\"Hello, world!\")", true)]
#[case("contract @\"add\"(a, b, return) = { return!(a + b) }", true)]
fn test_different_rholang_constructs(#[case] input: &str, #[case] expected: bool) -> Result<()> {
    let mut parser = RholangParser::new()?;
    assert_eq!(parser.is_valid(input), expected);

    let parse_result = parser.parse(input);
    assert_eq!(parse_result.is_success(), expected);

    let tree_result = parser.get_tree_string(input);
    assert_eq!(tree_result.is_success(), expected);

    let pretty_result = parser.get_pretty_tree(input);
    assert_eq!(pretty_result.is_success(), expected);

    Ok(())
}

// Test with edge cases
#[rstest]
#[case("", true)] // Empty string
#[case("// This is a comment", true)] // Just a comment
#[case("/* This is a block comment */", true)] // Block comment
#[case("\"\"", true)] // Empty string literal
#[case("\"\\\"\"", true)] // Escaped quote
fn test_edge_cases(#[case] input: &str, #[case] expected: bool) -> Result<()> {
    let mut parser = RholangParser::new()?;
    assert_eq!(parser.is_valid(input), expected);

    let parse_result = parser.parse(input);
    assert_eq!(parse_result.is_success(), expected);

    Ok(())
}
