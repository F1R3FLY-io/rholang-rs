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

/// Parse a raw string literal (including quotes) into its unescaped content.
///
/// Single-pass implementation with zero-copy fast path:
/// - If the literal contains no escapes, returns a borrowed slice of the input (without quotes).
/// - If escapes are present, returns an owned `String` with unescaped content.
pub fn parse_string_literal<'a>(raw: &'a str) -> Result<Cow<'a, str>, ParsingError> {
    let s = crate::trim_byte(raw, b'"');
    let bytes = s.as_bytes();

    // Find the first backslash; if none, return borrowed slice.
    let mut i = match bytes.iter().position(|&b| b == b'\\') {
        Some(pos) => pos,
        None => return Ok(Cow::Borrowed(s)),
    };

    // Prepare output and copy prefix before the first backslash.
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..i]);

    // Process remaining content.
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            if !push_until_backslash(&mut out, s, &mut i) {
                break;
            }
            continue;
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
    // Find the end of the contiguous ASCII digit run in the original slice
    // Use `str::find` with a predicate to get the byte position of the first
    // non-ASCII-digit. This avoids relying on any separate length value and
    // reads clearly as a position query.
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        return Err(ParsingError::InvalidStringEscape);
    }

    let num = match s[..end].parse::<u32>() {
        Ok(n) => n,
        Err(_) => return Err(ParsingError::InvalidStringCodePoint),
    };
    let c = char::from_u32(num).ok_or(ParsingError::InvalidStringCodePoint)?;
    Ok((c, end))
}

#[inline]
fn push_until_backslash(out: &mut String, s: &str, i: &mut usize) -> bool {
    let start = *i;
    if let Some(rel) = s[*i..].find('\\') {
        *i += rel;
        out.push_str(&s[start..*i]);
        true
    } else {
        out.push_str(&s[start..]);
        *i = s.len();
        false
    }
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

    // The following tests specifically exercise the branch that uses:
    //   if let Some(rel) = s[i..].find('\\') { i += rel; ... }
    // to ensure `find` returns a byte position within the slice (not a byte value)
    // and that adding it to `i` yields a valid UTF-8 boundary even with multibyte chars.

    #[test]
    fn test_copy_chunk_with_multibyte_between_escapes() {
        // Two escapes separated by multibyte UTF-8 characters ("😀α").
        // After handling the first escape, the loop should copy the multibyte chunk
        // up to the next backslash using s[i..].find('\\').
        let input = "\"\\n😀α\\t\""; // literal: "\n😀α\t"
        let expected = "\n😀α\t";
        assert_eq!(parse_string_literal(input).unwrap(), expected);
    }

    #[test]
    fn test_copy_chunk_with_only_ascii_between_escapes() {
        // Control case with only ASCII between escapes to exercise the same branch.
        let input = "\"\\nabc\\t\""; // literal: "\nabc\t"
        let expected = "\nabc\t";
        assert_eq!(parse_string_literal(input).unwrap(), expected);
    }

    // Additional coverage tests to reach 100%

    #[test]
    fn test_max_unicode_scalar_via_decimal_escape() {
        // 0x10FFFF == 1114111 (max valid Unicode scalar value)
        let input = "\"\\1114111\"";
        assert_eq!(parse_string_literal(input).unwrap(), "\u{10FFFF}");
    }

    #[test]
    fn test_decimal_overflow_parse_error_maps_to_invalid_code_point() {
        // An absurdly large decimal that doesn't fit in u32 should fail at parse::<u32>()
        // and be mapped to InvalidStringCodePoint by parse_decimal_escape.
        let big = "99999999999999999999999999999999999999999";
        let input = format!("\"\\{}\"", big);
        assert!(matches!(
            parse_string_literal(&input),
            Err(ParsingError::InvalidStringCodePoint)
        ));
    }

    #[test]
    fn test_copy_remainder_after_last_escape() {
        // After processing the first escape, there are no more backslashes;
        // the parser should copy the remainder via the `None` branch.
        assert_eq!(parse_string_literal("\"\\nxyz\"").unwrap(), "\nxyz");
    }

    #[test]
    fn test_parse_decimal_escape_end_zero_errors() {
        // Directly exercise the private helper to cover the `end == 0` branch
        assert!(matches!(parse_decimal_escape(""), Err(ParsingError::InvalidStringEscape)));
        assert!(matches!(parse_decimal_escape("x123"), Err(ParsingError::InvalidStringEscape)));
    }

    // ---- Direct tests for push_until_backslash ----

    #[test]
    fn test_push_until_backslash_no_backslash_ascii() {
        let s = "abcdef";
        let mut out = String::new();
        let mut i = 0usize;
        let found = push_until_backslash(&mut out, s, &mut i);
        assert!(!found);
        assert_eq!(out, "abcdef");
        assert_eq!(i, s.len());
    }

    #[test]
    fn test_push_until_backslash_no_backslash_unicode() {
        // Contains multibyte code points only, no backslash
        let s = "😀αβγ👍🏽";
        let mut out = String::from("pre:");
        let mut i = 0usize;
        let found = push_until_backslash(&mut out, s, &mut i);
        assert!(!found);
        assert_eq!(out, format!("pre:{}", s));
        assert_eq!(i, s.len());
    }

    #[test]
    fn test_push_until_backslash_backslash_at_current_index() {
        let s = "\\rest"; // backslash at index 0
        let mut out = String::new();
        let mut i = 0usize;
        let found = push_until_backslash(&mut out, s, &mut i);
        // Should stop immediately at the backslash, push nothing, return true
        assert!(found);
        assert_eq!(out, "");
        assert_eq!(i, 0);
    }

    #[test]
    fn test_push_until_backslash_ascii_before_backslash() {
        let s = "hello\\world";
        let mut out = String::new();
        let mut i = 0usize;
        let found = push_until_backslash(&mut out, s, &mut i);
        assert!(found);
        // Should copy up to but not including the backslash
        assert_eq!(out, "hello");
        // i should now point at the backslash byte position
        assert_eq!(&s[i..i + 1], "\\");
    }

    #[test]
    fn test_push_until_backslash_unicode_before_backslash() {
        // Sequence of complex multibyte graphemes before a backslash
        // Includes: ZWJ sequence and combining mark
        let woman_technologist = "👩‍💻"; // ZWJ sequence
        let e_acute_combining = "e\u{0301}"; // 'e' + COMBINING ACUTE ACCENT
        let chunk = format!("{}{}★α", woman_technologist, e_acute_combining);
        let s = format!("{}\\tail", chunk);
        let mut out = String::new();
        let mut i = 0usize;
        let found = push_until_backslash(&mut out, &s, &mut i);
        assert!(found);
        assert_eq!(out, chunk);
        // Ensure we did not split inside UTF-8: index should be at backslash and slicing is valid
        assert_eq!(&s[i..i + 1], "\\");
    }

    #[test]
    fn test_push_until_backslash_mid_string_start_index() {
        // Start from the middle (simulate parser loop after first escape)
        let s = "lead\\mid\\tail";
        // Start just after 'lead'
        let mut i = 4usize; // points to backslash
        let mut out = String::new();
        let found1 = push_until_backslash(&mut out, s, &mut i);
        // Since i was at a backslash, found1 should be true and nothing appended
        assert!(found1);
        assert_eq!(out, "");
        assert_eq!(&s[i..i + 1], "\\");

        // Advance past the first backslash and some ascii, then copy until the next
        i += 1; // simulate consuming the backslash by the caller; now i points at 'm'
        // From 'm', the helper should copy "mid" and stop at the next backslash
        let found2 = push_until_backslash(&mut out, s, &mut i);
        assert!(found2);
        assert_eq!(out, "mid");
        assert_eq!(&s[i..i + 1], "\\");
    }

    #[test]
    fn test_push_until_backslash_multiple_backslashes() {
        let s = "ab\\cd\\ef";
        let mut out = String::new();
        let mut i = 0usize;
        let found = push_until_backslash(&mut out, s, &mut i);
        assert!(found);
        assert_eq!(out, "ab");
        assert_eq!(&s[i..i + 1], "\\");
    }

    #[test]
    fn test_push_until_backslash_backslash_is_last_char() {
        let s = "αβγ\\"; // backslash is the last byte
        let mut out = String::new();
        let mut i = 0usize;
        let found = push_until_backslash(&mut out, s, &mut i);
        assert!(found);
        assert_eq!(out, "αβγ");
        assert_eq!(&s[i..i + 1], "\\");
        // Calling again without advancing should still return true and append nothing
        let found2 = push_until_backslash(&mut out, s, &mut i);
        assert!(found2);
        assert_eq!(out, "αβγ");
        assert_eq!(i, s.len() - 1);
    }
}
