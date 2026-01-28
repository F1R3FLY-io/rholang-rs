use validated::Validated;

use crate::{ParseFailure, ast::AnnProc};

/// Minimal wasm-friendly parser stub.
///
/// For the `wasm32` target we avoid compiling the C-based tree-sitter backend.
/// This stub exposes the same API but returns an empty AST, which higher levels
/// treat as "no-op" input.
pub struct RholangParser<'a> {
    _phantom: core::marker::PhantomData<&'a ()>,
}

impl<'a> RholangParser<'a> {
    pub fn new() -> Self {
        RholangParser {
            _phantom: core::marker::PhantomData,
        }
    }

    pub fn parse<'code: 'a>(
        &'a self,
        _code: &'code str,
    ) -> Validated<Vec<AnnProc<'a>>, ParseFailure<'a>> {
        Validated::Good(Vec::new())
    }
}

impl Default for RholangParser<'_> {
    fn default() -> Self {
        Self::new()
    }
}
