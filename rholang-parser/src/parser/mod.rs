mod ast_builder;

use crate::parser::ast_builder::ASTBuilder;

pub struct RholangParser<'a> {
    ast_builder: ASTBuilder<'a>,
}
