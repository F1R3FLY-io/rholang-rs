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

use std::borrow::Cow;
use crate::parser::errors::ParsingError;

fn trim_quotes(raw: &str) -> &str {
    crate::trim_byte(raw, b'"')
}

/// Parse a raw string literal (including quotes) into its unescaped content.
///
/// Single-pass implementation with zero-copy fast path:
/// - If the literal contains no escapes, returns a borrowed slice of the input (without quotes).
/// - If escapes are present, returns an owned `String` with unescaped content.
pub fn parse_string_literal<'a>(raw: &'a str) -> Result<Cow<'a, str>, ParsingError> {
    let s = trim_quotes(raw);

    // Scan once to find the first backslash byte. If none, we can return a borrowed slice.
    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && bytes[i] != b'\\' {
        i += 1; // scanning bytes is fine; backslash is ASCII and won't appear in UTF-8 continuation bytes
    }
    if i == bytes.len() {
        // No escapes at all
        return Ok(Cow::Borrowed(s));
    }

    // We found an escape at position `i`. Start building the output string in provided buffer.
    let mut out = String::with_capacity(s.len());
    // Push the prefix before first backslash
    out.push_str(&s[..i]);

    // Continue processing from the first backslash
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            // Copy a chunk up to the next backslash in one go.
            let start = i;
            if let Some(rel) = s[i..].find('\\') {
                i += rel;
                out.push_str(&s[start..i]);
                continue; // next loop iteration will see a backslash at i
            } else {
                // No more backslashes; copy the remainder and finish
                out.push_str(&s[start..]);
                break;
            }
        }

        // We are at a backslash; handle the escape sequence.
        i += 1; // consume '\\'
        if i >= bytes.len() {
            return Err(ParsingError::InvalidStringEscape);
        }

        match bytes[i] {
            b'\\' => {
                out.push('\\');
                i += 1;
            }
            b'"' => {
                out.push('"');
                i += 1;
            }
            b'n' => {
                out.push('\n');
                i += 1;
            }
            b'r' => {
                out.push('\r');
                i += 1;
            }
            b't' => {
                out.push('\t');
                i += 1;
            }
            b'0'..=b'9' => {
                // Decode decimal Unicode code point starting at current position
                let (ch, consumed) = parse_decimal_escape(&s[i..])?;
                out.push(ch);
                i += consumed;
            }
            _ => return Err(ParsingError::InvalidStringEscape),
        }
    }

    Ok(Cow::Owned(out))
}

fn parse_decimal_escape(s: &str) -> Result<(char, usize), ParsingError> {
    // Collect contiguous decimal digits and count bytes consumed
    let mut digits = String::new();
    let mut bytes_consumed = 0usize;
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            bytes_consumed += ch.len_utf8();
        } else {
            break;
        }
    }
    if digits.is_empty() {
        return Err(ParsingError::InvalidStringEscape);
    }
    let num = match digits.parse::<u32>() {
        Ok(n) => n,
        Err(_) => return Err(ParsingError::InvalidStringCodePoint),
    };
    let c = validate_scalar(num)?;
    Ok((c, bytes_consumed))
}

fn validate_scalar(num: u32) -> Result<char, ParsingError> {
    // `char::from_u32` already rejects values > 0x10FFFF and surrogate code points
    // (0xD800..=0xDFFF). Rely on it directly and map `None` to `InvalidCodePoint`.
    char::from_u32(num).ok_or(ParsingError::InvalidStringCodePoint)
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
        assert_eq!(parse_string_literal("\"a\\n b\\r c\\t d\"").unwrap(), "a\n b\r c\t d");
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
        assert!(matches!(parse_string_literal("\"abc\\\""), Err(ParsingError::InvalidStringEscape)));
    }

    #[test]
    fn test_invalid_escape() {
        assert!(matches!(parse_string_literal("\"foo\\x\""), Err(ParsingError::InvalidStringEscape)));
    }

    #[test]
    fn test_invalid_code_point() {
        // Above max
        assert!(matches!(parse_string_literal("\"\\1114112\""), Err(ParsingError::InvalidStringCodePoint)));
        // Surrogate
        assert!(matches!(parse_string_literal("\"\\55296\""), Err(ParsingError::InvalidStringCodePoint)));
    }
}
