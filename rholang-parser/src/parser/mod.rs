mod ast_builder;
pub mod errors;
mod parsing;

use validated::Validated;

use crate::{
    ast::AnnProc,
    parser::errors::ParsingFailure,
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
        let mut walker = tree.walk();

        tree.root_node()
            .named_children(&mut walker)
            .map(|node| parsing::node_to_ast(&node, &self.ast_builder, code))
            .collect()
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
