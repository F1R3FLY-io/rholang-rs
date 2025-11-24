pub(crate) mod ast_builder;
pub mod errors;
mod parsing;

use nonempty_collections::NEVec;
use validated::Validated;

use crate::{
    ast::AnnProc,
    parser::errors::{AnnParsingError, ParsingFailure},
};

pub use ast_builder::ASTBuilder;

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

        #[cfg(feature = "named-comments")]
        {
            // Skip comment nodes at the top level when named-comments is enabled
            parsing::named_children_no_comments(&root, &mut walker)
                .map(|node| parsing::node_to_ast(&node, &self.ast_builder, code))
                .collect()
        }
        #[cfg(not(feature = "named-comments"))]
        {
            // When named-comments is disabled, comments are unnamed and automatically excluded
            root.named_children(&mut walker)
                .map(|node| parsing::node_to_ast(&node, &self.ast_builder, code))
                .collect()
        }
    }

    // Expose AST builder for accessing const_nil
    pub fn ast_builder(&self) -> &ASTBuilder<'a> {
        &self.ast_builder
    }
}

impl Default for RholangParser<'_> {
    fn default() -> Self {
        Self::new()
    }
}
