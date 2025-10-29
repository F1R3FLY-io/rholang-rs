//! String literal parsing and unescaping
//!
//! Supports escape sequences:
//! - \\ backslash
//! - \" double quote
//! - \n newline
//! - \r carriage return
//! - \t tab
//! - \\ [0-9]+: decimal Unicode code point producing a single UTF-8 char
//!
//! Unicode policy: Only valid Unicode scalar values are accepted. Surrogate code points (0xD800..=0xDFFF)
//! and values above 0x10FFFF are rejected with `StringLitError::InvalidCodePoint`.
//!
//! Input is expected to be the raw literal as it appears in source, including
//! the surrounding quotes. The function will trim the outer quotes if present.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringLitError {
    InvalidEscape,
    InvalidCodePoint,
}

pub fn parse_string_literal(raw: &str) -> Result<String, StringLitError> {
    let s = trim_quotes(raw);
    if !s.as_bytes().contains(&b'\\') {
        return Ok(s.to_string());
    }
    decode_with_escapes(s)
}

fn trim_quotes(raw: &str) -> &str {
    crate::trim_byte(raw, b'"')
}

fn decode_with_escapes(s: &str) -> Result<String, StringLitError> {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(ch) = it.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match parse_escape(&mut it)? {
            ParsedEscape::Char(c) => out.push(c),
        }
    }
    Ok(out)
}

enum ParsedEscape {
    Char(char),
}

fn parse_escape<I>(it: &mut std::iter::Peekable<I>) -> Result<ParsedEscape, StringLitError>
where
    I: Iterator<Item = char>,
{
    let Some(next) = it.peek().copied() else {
        return Err(StringLitError::InvalidEscape);
    };
    match next {
        '\\' => {
            it.next();
            Ok(ParsedEscape::Char('\\'))
        }
        '"' => {
            it.next();
            Ok(ParsedEscape::Char('"'))
        }
        'n' => {
            it.next();
            Ok(ParsedEscape::Char('\n'))
        }
        'r' => {
            it.next();
            Ok(ParsedEscape::Char('\r'))
        }
        't' => {
            it.next();
            Ok(ParsedEscape::Char('\t'))
        }
        '0'..='9' => parse_decimal_escape(it).map(ParsedEscape::Char),
        _ => Err(StringLitError::InvalidEscape),
    }
}

fn parse_decimal_escape<I>(it: &mut std::iter::Peekable<I>) -> Result<char, StringLitError>
where
    I: Iterator<Item = char>,
{
    let mut num: u32 = 0;
    let mut consumed = 0usize;
    while let Some(d) = it.peek().and_then(|c| c.to_digit(10)) {
        it.next();
        consumed += 1;
        num = match num.checked_mul(10).and_then(|v| v.checked_add(d)) {
            Some(v) => v,
            None => return Err(StringLitError::InvalidCodePoint),
        };
    }
    if consumed == 0 {
        return Err(StringLitError::InvalidEscape);
    }
    validate_scalar(num)
}

fn validate_scalar(num: u32) -> Result<char, StringLitError> {
    if num > 0x10FFFF || (0xD800..=0xDFFF).contains(&num) {
        return Err(StringLitError::InvalidCodePoint);
    }
    char::from_u32(num).ok_or(StringLitError::InvalidCodePoint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain() {
        assert_eq!(parse_string_literal("\"hello\"").unwrap(), "hello");
    }

    #[test]
    fn test_basic_escapes() {
        assert_eq!(
            parse_string_literal("\"a\\n b\\r c\\t d\"").unwrap(),
            "a\n b\r c\t d"
        );
    }

    #[test]
    fn test_decimal_code_points() {
        assert_eq!(parse_string_literal("\"smile: \\128512\"").unwrap(), "smile: 😀");
        assert_eq!(parse_string_literal("\"nul: \\0\"").unwrap(), "nul: \u{0}");
        assert_eq!(parse_string_literal("\"A: \\65\"").unwrap(), "A: A");
    }

    #[test]
    fn test_backslash_via_decimal_escape() {
        // 92 is '\\'
        assert_eq!(parse_string_literal("\"\\92\"").unwrap(), "\\");
        // in context
        assert_eq!(parse_string_literal("\"x\\92y\"").unwrap(), "x\\y");
    }

    #[test]
    fn test_backslash_escape_simple_and_context() {
        // \\\\ -> \\
        assert_eq!(parse_string_literal("\"\\\\\"").unwrap(), "\\");
        // in context
        assert_eq!(parse_string_literal("\"a\\\\b\"").unwrap(), "a\\b");
    }

    #[test]
    fn test_double_quote_escape_simple_and_context() {
        // \" -> "
        assert_eq!(parse_string_literal("\"\\\"\"").unwrap(), "\"");
        // in context
        assert_eq!(parse_string_literal("\"a\\\"b\"").unwrap(), "a\"b");
        // mixed with others
        assert_eq!(parse_string_literal("\"x\\\"\\n\"\"").unwrap(), "x\"\n\"");
    }

    #[test]
    fn test_invalid_trailing_backslash() {
        // Lone trailing backslash
        assert!(matches!(parse_string_literal("\"abc\\\""), Err(StringLitError::InvalidEscape)));
    }

    #[test]
    fn test_invalid_escape() {
        assert!(matches!(parse_string_literal("\"foo\\x\""), Err(StringLitError::InvalidEscape)));
    }

    #[test]
    fn test_invalid_code_point() {
        // Above max
        assert!(matches!(parse_string_literal("\"\\1114112\""), Err(StringLitError::InvalidCodePoint)));
        // Surrogate
        assert!(matches!(parse_string_literal("\"\\55296\""), Err(StringLitError::InvalidCodePoint)));
    }
}

