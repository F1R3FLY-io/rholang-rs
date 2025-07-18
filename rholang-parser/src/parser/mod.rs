mod ast_builder;
pub mod errors;
mod parsing;

use validated::Validated;

use crate::{
    ast::AnnProc,
    parser::{ast_builder::ASTBuilder, errors::AnnParsingError},
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
    ) -> Validated<Vec<AnnProc<'a>>, AnnParsingError> {
        let tree = parsing::parse_to_tree(code);
        let mut walker = tree.walk();

        tree.root_node()
            .named_children(&mut walker)
            .map(|node| parsing::node_to_ast(&node, &self.ast_builder, code))
            .collect()
    }
}
