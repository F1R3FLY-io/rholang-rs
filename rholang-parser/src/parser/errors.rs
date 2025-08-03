use std::{collections::HashSet, ops::Range, sync::OnceLock};

use nonempty_collections::NEVec;

use crate::{SourcePos, SourceSpan, ast::AnnProc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsingError {
    SyntaxError { sexp: String },
    MissingToken(&'static str),
    Unexpected(char),
    UnexpectedVar(String),
    NumberOutOfRange,
    DuplicateNameDecl { first: SourcePos, second: SourcePos },
    MalformedLetDecl { lhs_arity: usize, rhs_arity: usize },
}

impl ParsingError {
    fn from_error_node(node: &tree_sitter::Node, code: &[u8]) -> Self {
        if let Some(child) = node.named_child(0) {
            if child.is_error() {
                let text = get_text(&child, code);
                let mut chars = text.chars();
                if let Some(unexpected) = chars.next() {
                    // it's a single char
                    if chars.next().is_none() {
                        return ParsingError::Unexpected(unexpected);
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

    pub(super) fn from_var(var_node: &tree_sitter::Node, code: &[u8]) -> Self {
        let var = get_text(var_node, code);
        AnnParsingError {
            error: ParsingError::UnexpectedVar(var.to_owned()),
            span: var_node.range().into(),
        }
    }
}

fn get_text<'a>(of: &tree_sitter::Node, code: &'a [u8]) -> &'a str {
    unsafe {
        // SAFETY: source code is expected to contain valid utf8 and our grammar does not allow to
        // chop any single character. So, byte ranges of all nodes must start and end on valid UTF-8
        // slice
        str::from_utf8_unchecked(&code[of.byte_range()])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsingFailure<'a> {
    pub partial_tree: Option<AnnProc<'a>>,
    pub errors: NEVec<AnnParsingError>,
}

pub(super) fn query_errors(of: &tree_sitter::Node, source: &str, into: &mut Vec<AnnParsingError>) {
    use tree_sitter::StreamingIterator;

    static QUERY: OnceLock<tree_sitter::Query> = OnceLock::new();

    let query = QUERY.get_or_init(|| {
        let rholang_language = rholang_tree_sitter::LANGUAGE.into();
        tree_sitter::Query::new(
            &rholang_language,
            "(ERROR (var) @error-var)
            (MISSING) @missing-node 
            (ERROR) @fallback",
        )
        .expect("failed to compile error query")
    });

    let mut cursor = tree_sitter::QueryCursor::new();
    let source_bytes = source.as_bytes();

    // Record parent ranges of specific errors like @error-var
    let mut claimed_error_ranges: HashSet<Range<usize>> = HashSet::new();

    // Temporarily hold general error nodes
    let mut general_errors: Vec<tree_sitter::Node> = Vec::new();

    let mut matches = cursor.matches(query, *of, source_bytes);
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let node = capture.node;

            match capture.index {
                0 => {
                    // @error-var
                    if let Some(parent) = node.parent() {
                        claimed_error_ranges.insert(parent.byte_range());
                    }
                    into.push(AnnParsingError::from_var(&node, source_bytes));
                }
                1 => {
                    // @missing-node
                    into.push(AnnParsingError::from_mising(&node));
                }
                _ => {
                    if node.parent().is_some_and(|p| p.is_error()) {
                        continue; // skip UNEXPECTED, we process it somewhere else
                    }
                    general_errors.push(node);
                }
            }
        }
    }

    // Emit only general errors not already claimed by more specific ones
    for node in general_errors {
        let range = node.byte_range();
        if claimed_error_ranges.contains(&range) {
            continue; // already handled by more specific pattern
        }

        into.push(AnnParsingError::from_error(&node, source_bytes));
    }
}
