//! Rholang parser based on tree-sitter grammar
//!
//! This crate provides a parser for the Rholang language using the tree-sitter grammar
//! defined in the rholang-tree-sitter crate.

use std::fmt::{Debug, Display, Write};

pub mod ast;
pub mod parser;

pub use parser::RholangParser;

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
