use anyhow::Result;
use rholang_parser::RholangParser;

fn main() -> Result<()> {
    // Create a new parser
    let mut parser = RholangParser::new()?;

    // Example Rholang code
    let code = r#"
    new channel in {
        @"stdout"!("Hello, world!")
    }
    "#;

    // Check if the code is valid
    println!("Is valid: {}", parser.is_valid(code));

    // Get a string representation of the parse tree
    match parser.get_tree_string(code) {
        rholang_parser::errors::ParseResult::Success(tree_string) => {
            println!("Parse tree:\n{}", tree_string);
        }
        rholang_parser::errors::ParseResult::Error(err) => {
            println!("Error: {}", err);
        }
    }

    // Get a pretty-printed representation of the parse tree
    match parser.get_pretty_tree(code) {
        rholang_parser::errors::ParseResult::Success(pretty_tree) => {
            println!("Pretty parse tree:\n{}", pretty_tree);
        }
        rholang_parser::errors::ParseResult::Error(err) => {
            println!("Error: {}", err);
        }
    }

    // Example with invalid code
    let invalid_code = r#"
    new channel in {
        @"stdout"!("Missing closing parenthesis"
    }
    "#;

    println!("\nInvalid code example:");
    println!("Is valid: {}", parser.is_valid(invalid_code));

    // Try to parse invalid code
    match parser.get_pretty_tree(invalid_code) {
        rholang_parser::errors::ParseResult::Success(pretty_tree) => {
            println!("Pretty parse tree:\n{}", pretty_tree);
        }
        rholang_parser::errors::ParseResult::Error(err) => {
            println!("Error: {}", err);
        }
    }

    Ok(())
}
