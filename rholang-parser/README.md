# Rholang Parser

Parser and AST builder for the Rholang language, backed by the workspace tree-sitter grammar.

## Overview

`rholang-parser` parses source into typed AST nodes (`AnnProc`) and returns either:

- `Validated::Good(Vec<AnnProc>)` on success
- `Validated::Fail(...)` with structured parse failures

This crate is consumed by the shell runtime, semantic analysis (`rholang-lib`), and tooling.

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
rholang-parser = { path = "../rholang-parser" }
validated = "1"
```

Example:

```rust
use rholang_parser::RholangParser;
use validated::Validated;

fn main() {
    let parser = RholangParser::new();
    let code = "1u8 + 2u8";

    match parser.parse(code) {
        Validated::Good(procs) => {
            println!("Parsed {} top-level process(es)", procs.len());
        }
        Validated::Fail(failures) => {
            for failure in failures {
                for err in failure.errors.iter() {
                    println!("Parse error at {}: {:?}", err.span.start, err.error);
                }
            }
        }
    }
}
```

## Numeric Literals

The parser recognizes the numeric literal families used in the workspace:

- default signed 64-bit integer: `42`
- signed fixed-width integer: `-52i64`
- unsigned fixed-width integer: `65535u16`
- bigint: `10n`
- bigrat: `3r`
- floating point: `-1.234e5f32`, `123400f64`, `1f128`, `1f256`
- fixed point: `10p1`, `3.3p1`, `.25p2`

Notes:

- integer widths must be powers of two and at least 8
- float literal widths are currently `32`, `64`, `128`, `256` in grammar/parser

## API Surface

Primary entrypoint:

- `RholangParser::new() -> RholangParser`
- `RholangParser::parse(&self, code: &str) -> Validated<Vec<AnnProc>, ParsingFailure>`
