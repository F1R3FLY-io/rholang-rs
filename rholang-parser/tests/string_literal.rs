use rholang_parser::{RholangParser, ast::{Proc, AnnProc}, parser::errors::ParsingError};

fn extract_first_string_literal<'a>(proc: &'a AnnProc<'a>) -> Option<&'a str> {
    match &proc.proc {
        Proc::Send { inputs, .. } => {
            if let Some(first) = inputs.first() {
                if let Proc::StringLiteral(s) = first.proc {
                    return Some(s);
                }
            }
            None
        }
        Proc::Method { args, .. } => {
            if let Some(first) = args.first() {
                if let Proc::StringLiteral(s) = first.proc {
                    return Some(s);
                }
            }
            None
        }
        _ => None,
    }
}

#[test]
fn integration_basic_newline_escape() {
    let src = r#"stdout!("\n")"#; // literal with \n should become actual newline in AST
    let parser = RholangParser::new();
        let parsed = parser.parse(src);
    match parsed.ok() {
        Ok(procs) => {
            assert_eq!(procs.len(), 1);
            let s = extract_first_string_literal(&procs[0]).expect("expected string literal input");
            assert_eq!(s, "\n");
        }
        Err(errors) => panic!("unexpected parse failure: {:?}", errors),
    }
}

#[test]
fn integration_tab_and_cr_escapes() {
    let src = r#"stdout!("a\t b\r c")"#;
    let parser = RholangParser::new();
    match parser.parse(src).ok() {
        Ok(procs) => {
            let s = extract_first_string_literal(&procs[0]).unwrap();
            assert_eq!(s, "a\t b\r c");
        }
        Err(errors) => panic!("unexpected parse failure: {:?}", errors),
    }
}

#[test]
fn integration_decimal_codepoint_emoji() {
    // 128512 = 😀
    let src = r#"stdout!("smile: \128512")"#;
    let parser = RholangParser::new();
    match parser.parse(src).ok() {
        Ok(procs) => {
            let s = extract_first_string_literal(&procs[0]).unwrap();
            assert_eq!(s, "smile: 😀");
        }
        Err(errors) => panic!("unexpected parse failure: {:?}", errors),
    }
}

#[test]
fn integration_codepoint_max() {
    // 1114111 = 0x10FFFF, last valid scalar
    let src = r#"stdout!("\1114111")"#;
    let parser = RholangParser::new();
    match parser.parse(src).ok() {
        Ok(procs) => {
            let s = extract_first_string_literal(&procs[0]).unwrap();
            assert_eq!(s, "\u{10FFFF}");
        }
        Err(errors) => panic!("unexpected parse failure: {:?}", errors),
    }
}

#[test]
fn integration_backslash_via_decimal_escape() {
    // 92 is backslash
    let src = r#"stdout!("\92")"#;
    let parser = RholangParser::new();
    match parser.parse(src).ok() {
        Ok(procs) => {
            let s = extract_first_string_literal(&procs[0]).unwrap();
            assert_eq!(s, "\\");
        }
        Err(errors) => panic!("unexpected parse failure: {:?}", errors),
    }
}

#[test]
fn integration_double_quote_escape() {
    // simple \" inside string
    let parser = RholangParser::new();
    let src1 = r#"stdout!("\"")"#;
    match parser.parse(src1).ok() {
        Ok(procs) => {
            let s = extract_first_string_literal(&procs[0]).unwrap();
            assert_eq!(s, "\"");
        }
        Err(errors) => panic!("unexpected parse failure: {:?}", errors),
    }

    // in context a\"b -> a"b
    let parser = RholangParser::new();
    let src2 = r#"stdout!("a\"b")"#;
    match parser.parse(src2).ok() {
        Ok(procs) => {
            let s = extract_first_string_literal(&procs[0]).unwrap();
            assert_eq!(s, "a\"b");
        }
        Err(errors) => panic!("unexpected parse failure: {:?}", errors),
    }
}

#[test]
fn integration_invalid_escape_reports_error() {
    let src = r#"stdout!("\x")"#;
    let parser = RholangParser::new();
    match parser.parse(src).ok() {
        Ok(_) => panic!("expected failure for invalid escape"),
        Err(errors) => {
            // should contain an InvalidStringEscape error OR a SyntaxError (lexer-level), depending on how the grammar tokenizes it
            let has_expected = errors.iter().any(|pf| pf.errors.iter().any(|ae| matches!(ae.error, ParsingError::InvalidStringEscape | ParsingError::SyntaxError { .. })));
            assert!(has_expected, "errors were: {:?}", errors);
        }
    }
}

#[test]
fn integration_invalid_codepoint_reports_error() {
    // 1114112 is out of range
    let src = r#"stdout!("\1114112")"#;
    let parser = RholangParser::new();
    match parser.parse(src).ok() {
        Ok(_) => panic!("expected failure for out-of-range code point"),
        Err(errors) => {
            assert!(errors.iter().any(|pf| pf.errors.iter().any(|ae| matches!(ae.error, ParsingError::InvalidStringCodePoint))),
                "errors were: {:?}", errors);
        }
    }
}
