//! Rholang parser based on tree-sitter grammar
//!
//! This crate provides a parser for the Rholang language using the tree-sitter grammar
//! defined in the rholang-tree-sitter crate.

use std::fmt::{Debug, Display, Write};

pub mod ast;

/// a position in the source code. 1-based
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourcePos {
    pub line: usize,
    pub col: usize,
}

impl Display for SourcePos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.line, f)?;
        f.write_char(':')?;
        Display::fmt(&self.col, f)?;
        Ok(())
    }
}

/// a span in the source code (inclusive)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: SourcePos,
    pub end: SourcePos,
}

impl Display for SourceSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.start, f)?;
        f.write_str(" - ")?;
        Display::fmt(&self.end, f)?;
        Ok(())
    }
}
