//! Rholang parser
//!
//! Non-wasm builds use the tree-sitter based implementation. For `wasm32` target we
//! provide a minimal stub parser to avoid compiling native C code.

use std::fmt::{Debug, Display, Write};

pub mod ast;
#[cfg(not(target_arch = "wasm32"))]
pub mod parser;
#[cfg(target_arch = "wasm32")]
pub mod parser_wasm;
#[cfg(target_arch = "wasm32")]
pub use parser_wasm as parser;
mod traverse;

pub use parser::RholangParser;

// Unified parse failure type alias for consumers
#[cfg(not(target_arch = "wasm32"))]
pub type ParseFailure<'a> = parser::errors::ParsingFailure<'a>;
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone)]
pub struct ParseFailure<'a> {
    pub _phantom: core::marker::PhantomData<&'a ()>,
}
pub use traverse::{DfsEvent, DfsEventExt};

/// a position in the source code. 1-based
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcePos {
    pub line: usize,
    pub col: usize,
}

impl SourcePos {
    pub fn span_of(self, chars: usize) -> SourceSpan {
        let end = SourcePos {
            line: self.line,
            col: self.col + chars,
        };
        SourceSpan { start: self, end }
    }

    pub fn at_line(line: usize) -> SourcePos {
        SourcePos {
            line: line.max(1),
            col: 1,
        }
    }

    pub fn at_col(col: usize) -> SourcePos {
        SourcePos {
            line: 1,
            col: col.max(1),
        }
    }
}

impl Display for SourcePos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.line, f)?;
        f.write_char(':')?;
        Display::fmt(&self.col, f)?;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<tree_sitter::Point> for SourcePos {
    fn from(value: tree_sitter::Point) -> Self {
        SourcePos {
            line: value.row + 1,
            col: value.column + 1,
        }
    }
}

impl Default for SourcePos {
    fn default() -> Self {
        Self { line: 1, col: 1 }
    }
}

/// a span in the source code (exclusive)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: SourcePos,
    pub end: SourcePos,
}

impl SourceSpan {
    pub fn empty_at(start: SourcePos) -> Self {
        Self { start, end: start }
    }
}

impl Default for SourceSpan {
    fn default() -> Self {
        Self::empty_at(SourcePos::default())
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<tree_sitter::Range> for SourceSpan {
    fn from(value: tree_sitter::Range) -> Self {
        SourceSpan {
            start: value.start_point.into(),
            end: value.end_point.into(),
        }
    }
}

impl Display for SourceSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.start, f)?;
        f.write_str(" - ")?;
        Display::fmt(&self.end, f)?;
        Ok(())
    }
}

// helper function for literals
fn trim_byte(s: &str, a: u8) -> &str {
    let bytes = s.as_bytes();
    let mut start = 0;
    let mut end = bytes.len();

    if start < end && bytes[0] == a {
        start += 1;
    }
    if start < end && bytes[end - 1] == a {
        end -= 1;
    }

    &s[start..end]
}
