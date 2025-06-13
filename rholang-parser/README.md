# Rholang Parser

A parser for the Rholang language based on the tree-sitter grammar.

## Overview

This crate provides a parser for the Rholang language. It is designed to be used by tools that need to parse and analyze Rholang code, such as IDEs, linters, and other development tools.

This parser uses the tree-sitter grammar provided by the `rholang-tree-sitter` crate.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rholang-parser = { path = "../rholang-parser" }
```

Then, you can use it in your code:

```rust
use anyhow::Result;
use rholang_parser::RholangParser;

fn main() -> Result<()> {
    // Create a new parser
    let mut parser = RholangParser::new()?;

    // Check if code is valid
    let code = "new channel in { @\"stdout\"!(\"Hello, world!\") }";
    if parser.is_valid(code) {
        println!("Code is valid!");
    } else {
        println!("Code is invalid!");
    }

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

    Ok(())
}
```

## API

The main struct is `RholangParser`, which provides the following methods:

- `new()`: Create a new parser
- `is_valid(code: &str) -> bool`: Check if the code is valid Rholang
- `parse(code: &str) -> ParseResult<()>`: Parse the code and return a result
- `get_tree_string(code: &str) -> ParseResult<String>`: Get a string representation of the parse tree
- `get_pretty_tree(code: &str) -> ParseResult<String>`: Get a pretty-printed representation of the parse tree
