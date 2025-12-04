use rholang_parser::parser::{RholangParser, errors::ParsingError, parse_string_literal};

fn extract_first_string_literal<'a>(proc: &'a rholang_parser::ast::AnnProc<'a>) -> Option<&'a str> {
    use rholang_parser::ast::Proc;
    match &proc.proc {
        Proc::Send { inputs, .. } => {
            if let Some(first) = inputs.first() {
                if let Proc::StringLiteral(s) = first.proc {
                    return Some(s.as_ref());
                }
            }
            None
        }
        Proc::Method { args, .. } => {
            if let Some(first) = args.first() {
                if let Proc::StringLiteral(s) = first.proc {
                    return Some(s.as_ref());
                }
            }
            None
        }
        _ => None,
    }
}

#[test]
fn unit_plain_fast_path_no_escapes() {
    // Exercises the early return borrowed path
    let s = parse_string_literal("\"abcdef\"").unwrap();
    assert_eq!(s, "abcdef");
}

#[test]
fn unit_prefix_copy_and_escape_match_arms() {
    // Ensures we hit: first backslash detection, prefix push, and multiple match arms
    let s = parse_string_literal("\"pre\\\\mid\\\"post\"").unwrap();
    assert_eq!(s, "pre\\mid\"post");
}

#[test]
fn unit_copy_chunk_some_and_none_branches() {
    // After handling an escape, copy chunk until next backslash (Some), then copy remainder (None)
    let s = parse_string_literal("\"\\nXYZ\\tRest\"").unwrap();
    assert_eq!(s, "\nXYZ\tRest");
}

#[test]
fn unit_decimal_escape_and_increase_index() {
    // Hit the decimal escape branch and ensure consumed index advances
    let s = parse_string_literal("\"A: \\65, max: \\1114111\"").unwrap();
    assert_eq!(s, "A: A, max: \u{10FFFF}");
}

#[test]
fn unit_invalid_trailing_backslash_error_path() {
    // Exercises the i >= bytes.len() error path after consuming a backslash
    let parser = RholangParser::new();
    let src = r#"stdout!("abc\")"#;
    match parser.parse(src).ok() {
        Ok(_) => panic!("expected failure for invalid escape"),
        Err(errors) => {
            let has_expected = errors.iter().any(|pf| pf.errors.iter().any(|ae| matches!(ae.error, ParsingError::InvalidStringEscape | ParsingError::SyntaxError { .. })));
            assert!(has_expected, "errors were: {:?}", errors);
        }
    }
}
