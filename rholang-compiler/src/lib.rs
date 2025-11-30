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

use anyhow::Result;
use librho::sem::SemanticDb;
use rholang_parser::ast::AnnProc;
use rholang_vm::api::Process;

pub use codegen::CodegenContext;

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

// -------------------- High-level async facade (parsing + sem + codegen) --------------------
use librho::sem::{pipeline::Pipeline, EnclosureAnalysisPass, ForCompElaborationPass, ResolverPass};
use rholang_parser::RholangParser;

/// Parse, run semantic pipeline, and compile all top-level processes from a Rholang source string.
///
/// This is an async function because the semantic `Pipeline::run` is async. On wasm this runs
/// sequentially; on native it may use concurrency internally.
pub async fn compile_source_async(
    src: &str,
) -> Result<Vec<rholang_vm::api::Process>> {
    // Parse
    let parser = RholangParser::new();
    let validated = parser.parse(src);
    let ast_vec = match validated {
        validated::Validated::Good(ast) => ast,
        validated::Validated::Fail(err) => {
            // Use anyhow for ergonomic error propagation
            return Err(anyhow::anyhow!("ParseError: {err:#?}"));
        }
    };

    if ast_vec.is_empty() {
        return Ok(Vec::new());
    }

    // Build semantic DB and run essential passes
    let mut db = SemanticDb::new();
    let root = db.build_index(&ast_vec[0]);

    let pipeline = Pipeline::new()
        .add_fact(ResolverPass::new(root))
        .add_fact(ForCompElaborationPass::new(root))
        .add_fact(EnclosureAnalysisPass::new(root));
    pipeline.run(&mut db).await;

    // Compile all procs
    let compiler = Compiler::new(&db);
    compiler.compile(&ast_vec)
}

/// Convenience: compile only the first top-level process in the source.
pub async fn compile_first_process_async(src: &str) -> Result<Process> {
    let procs = compile_source_async(src).await?;
    procs
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No process in source"))
}
