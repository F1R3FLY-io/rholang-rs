pub(crate) mod ast_builder;
pub mod errors;
mod parsing;
mod string_literal;
pub use string_literal::parse_string_literal;

use nonempty_collections::NEVec;
use validated::Validated;

use crate::{
    ast::AnnProc,
    parser::{
        ast_builder::ASTBuilder,
        errors::{AnnParsingError, ParsingFailure},
    },
};

pub struct RholangParser<'a> {
    ast_builder: ASTBuilder<'a>,
}

impl<'a> RholangParser<'a> {
    pub fn new() -> Self {
        RholangParser {
            ast_builder: ASTBuilder::new(),
        }
    }

    pub fn parse<'code: 'a>(
        &'a self,
        code: &'code str,
    ) -> Validated<Vec<AnnProc<'a>>, ParsingFailure<'a>> {
        let tree = parsing::parse_to_tree(code);
        let root = tree.root_node();
        if root.is_error() {
            let mut errors_inside = Vec::new();
            errors::query_errors(&root, code, &mut errors_inside);
            let errors = NEVec::try_from_vec(errors_inside)
                .unwrap_or_else(|| NEVec::new(AnnParsingError::from_error(&root, code.as_bytes())));
            return Validated::fail(ParsingFailure {
                partial_tree: None, // perhaps we're thrwoing away too much information here. FIXME
                errors,
            });
        }
        let mut walker = tree.walk();

        root.named_children(&mut walker)
            .map(|node| parsing::node_to_ast(&node, &self.ast_builder, code))
            .collect()
    }
}

impl Default for RholangParser<'_> {
    fn default() -> Self {
        Self::new()
    }
}
