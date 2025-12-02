//! Rholang Compiler
//!
//! ## Usage
//! ```ignore
//! use rholang_compiler::Compiler;
//! use rholang_lib::sem::{SemanticDb, Pipeline};
//! use rholang_parser::RholangParser;
//!
//! let parser = RholangParser::new();
//! let ast = parser.parse("new x in { x!(42) | for (y <- x) { y } }").unwrap();
//!
//! let mut db = SemanticDb::new();
//! db.build_index(&ast);
//!
//! let compiler = Compiler::new(&db);
//! let processes = compiler.compile(&[ast])?;
//! ```

mod codegen;
mod disassembler;

use anyhow::Result;
use librho::sem::SemanticDb;
use rholang_parser::ast::AnnProc;
use rholang_vm::api::Process;

pub use codegen::CodegenContext;
pub use disassembler::{Disassembler, DisassemblerConfig, DisassemblyFormat};

/// The main compiler that transforms Rholang AST into bytecode processes
///
/// The compiler is stateless and uses the SemanticDb for variable resolution
/// and semantic information
pub struct Compiler<'a> {
    db: &'a SemanticDb<'a>,
}

impl<'a> Compiler<'a> {
    pub fn new(db: &'a SemanticDb<'a>) -> Self {
        Self { db }
    }

    /// Compile a list of top-level processes into executable bytecode processes
    ///
    /// Each process in the input list is compiled independently and produces
    /// one output Process with its instruction stream
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Unsupported language features are encountered
    /// - Compilation fails due to invalid AST structure
    pub fn compile(&self, procs: &[AnnProc<'a>]) -> Result<Vec<Process>> {
        let mut results = Vec::with_capacity(procs.len());

        for (idx, proc) in procs.iter().enumerate() {
            let mut ctx = CodegenContext::new(self.db, idx);
            ctx.compile_proc(proc)?;
            results.push(ctx.finalize()?);
        }

        Ok(results)
    }

    /// Compile a single top-level process into an executable bytecode process
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails (see `compile` for details)
    pub fn compile_single(&self, proc: &AnnProc<'a>) -> Result<Process> {
        let mut ctx = CodegenContext::new(self.db, 0);
        ctx.compile_proc(proc)?;
        ctx.finalize()
    }
}
