use crate::{SourcePos, SourceSpan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsingError {
    SyntaxError,
    MissingToken(&'static str),
    NumberOutOfRange,
    DuplicateNameDecl { first: SourcePos, second: SourcePos },
    MalformedLetDecl { lhs_arity: usize, rhs_arity: usize },
}

pub struct AnnParsingError {
    pub error: ParsingError,
    pub span: SourceSpan,
}
