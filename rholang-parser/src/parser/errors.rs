use nonempty_collections::NEVec;

use crate::{SourcePos, SourceSpan, ast::AnnProc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsingError {
    SyntaxError { sexp: String },
    MissingToken(&'static str),
    Unexpected(char),
    NumberOutOfRange,
    DuplicateNameDecl { first: SourcePos, second: SourcePos },
    MalformedLetDecl { lhs_arity: usize, rhs_arity: usize },
}

impl ParsingError {
    fn from_error_node(node: &tree_sitter::Node, code: &[u8]) -> Self {
        if let Some(child) = node.named_child(0) {
            if child.is_error() {
                unsafe {
                    // SAFETY: source code is expected to contain valid utf8 and our grammar does not allow to
                    // chop any single character. So, byte ranges of all nodes must start and end on valid UTF-8
                    // slice
                    let text = str::from_utf8_unchecked(&code[child.byte_range()]);
                    let mut chars = text.chars();
                    if let Some(unexpected) = chars.next() {
                        // it's a single char
                        if chars.next().is_none() {
                            return ParsingError::Unexpected(unexpected);
                        }
                    }
                }
            }
        }
        ParsingError::SyntaxError {
            sexp: node.to_sexp(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnParsingError {
    pub error: ParsingError,
    pub span: SourceSpan,
}

impl AnnParsingError {
    pub(super) fn from_error(node: &tree_sitter::Node, code: &[u8]) -> Self {
        let error = ParsingError::from_error_node(node, code);
        AnnParsingError {
            error,
            span: node.range().into(),
        }
    }

    pub(super) fn from_mising(node: &tree_sitter::Node) -> Self {
        let kind = node.kind();
        AnnParsingError {
            error: ParsingError::MissingToken(kind),
            span: node.range().into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsingFailure<'a> {
    pub partial_tree: Option<AnnProc<'a>>,
    pub errors: NEVec<AnnParsingError>,
}
