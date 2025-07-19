use crate::{SourcePos, SourceSpan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsingError {
    SyntaxError,
    MissingToken(&'static str),
    Unexpected(char),
    NumberOutOfRange,
    DuplicateNameDecl { first: SourcePos, second: SourcePos },
    MalformedLetDecl { lhs_arity: usize, rhs_arity: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnParsingError {
    pub error: ParsingError,
    pub span: SourceSpan,
}

impl AnnParsingError {
    pub(super) fn from_error(node: &tree_sitter::Node, code: &[u8]) -> Self {
        let error = if let Some(child) = node.named_child(0) {
            if child.is_error() {
                unsafe {
                    let text = str::from_utf8_unchecked(&code[child.byte_range()]);
                    if text.len() == 1 {
                        ParsingError::Unexpected(text.chars().next().unwrap_unchecked())
                    } else {
                        ParsingError::SyntaxError
                    }
                }
            } else {
                ParsingError::SyntaxError
            }
        } else {
            ParsingError::SyntaxError
        };
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
