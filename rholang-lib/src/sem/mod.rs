use by_address::ByAddress;
use indexmap::IndexMap;
use rholang_parser::ast;

mod alg;
mod db;

pub type ProcRef<'a> = &'a ast::AnnProc<'a>;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PID(i32);

pub struct SemanticDb<'a> {
    rev: IndexMap<ByAddress<ProcRef<'a>>, PID>,
}
