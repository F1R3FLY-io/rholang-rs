//! String literal parsing and unescaping
//!
//! Supports escape sequences:
//! - \n newline
//! - \r carriage return
//! - \t tab
//! - \\ [0-9]+: decimal Unicode code point producing a single UTF-8 char
//!
//! Input is expected to be the raw literal as it appears in source, including
//! the surrounding quotes. The function will trim the outer quotes if present.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringLitError {
    InvalidEscape,
    InvalidCodePoint,
}

pub fn parse_string_literal(raw: &str) -> Result<String, StringLitError> {
    // Trim surrounding double quotes if present
    let s = crate::trim_byte(raw, b'"');

    // Fast path: no backslashes, return as is
    if !s.as_bytes().contains(&b'\\') {
        return Ok(s.to_string());
    }

    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        // Escape sequence
        let Some(next) = chars.peek().copied() else {
            return Err(StringLitError::InvalidEscape);
        };

        match next {
            '\\' => {
                chars.next();
                out.push('\\');
            }
            'n' => {
                chars.next();
                out.push('\n');
            }
            'r' => {
                chars.next();
                out.push('\r');
            }
            't' => {
                chars.next();
                out.push('\t');
            }
            '0'..='9' => {
                // Collect consecutive digits
                let mut num: u32 = 0;
                let mut consumed = 0usize;
                while let Some(d) = chars.peek().and_then(|c| c.to_digit(10)) {
                    chars.next();
                    consumed += 1;
                    num = match num.checked_mul(10).and_then(|v| v.checked_add(d)) {
                        Some(v) => v,
                        None => return Err(StringLitError::InvalidCodePoint),
                    };
                }
                if consumed == 0 {
                    // Shouldn't happen because branch already matched '0'..='9'
                    return Err(StringLitError::InvalidEscape);
                }
                // Validate Unicode scalar value
                if num > 0x10FFFF || (0xD800..=0xDFFF).contains(&num) {
                    return Err(StringLitError::InvalidCodePoint);
                }
                if let Some(c) = char::from_u32(num) {
                    out.push(c);
                } else {
                    return Err(StringLitError::InvalidCodePoint);
                }
            }
            _ => {
                // Unknown escape
                return Err(StringLitError::InvalidEscape);
            }
        }
    }

    Ok(out)
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
            // \\ -> \
            assert_eq!(parse_string_literal("\"\\\\\"").unwrap(), "\\");
            // in context
            assert_eq!(parse_string_literal("\"a\\\\b\"").unwrap(), "a\\b");
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

