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
mod driver;
mod error;
mod reporter;

use anyhow::Result;
use librho::sem::SemanticDb;
use rholang_parser::ast::AnnProc;
use rholang_vm::api::Process;

pub use codegen::CodegenContext;
pub use disassembler::{Disassembler, DisassemblerConfig, DisassemblyFormat};
pub use driver::{CompileDriver, CompileOptions, CompileOutput, DbStats, PipelineBuilder};
pub use error::{CompileError, CompileErrorInfo, CompileResult};
pub use reporter::{ErrorReporter, ReporterConfig};

/// The main compiler that transforms Rholang AST into bytecode processes
pub struct Compiler<'a> {
    db: &'a SemanticDb<'a>,
    /// Whether to filter recoverable errors (e.g., NameInProcPosition)
    filter_recoverable: bool,
}

impl<'a> Compiler<'a> {
    pub fn new(db: &'a SemanticDb<'a>) -> Self {
        Self {
            db,
            filter_recoverable: true, // Default: filter NameInProcPosition
        }
    }

    /// Create compiler with strict error checking (no recoverable error filtering)
    pub fn strict(db: &'a SemanticDb<'a>) -> Self {
        Self {
            db,
            filter_recoverable: false,
        }
    }

    /// Compile with automatic error checking
    ///
    /// This is the recommended entry point. It checks for semantic errors
    /// before proceeding with code generation.
    ///
    /// # Errors
    ///
    /// Returns `CompileError::SemanticErrors` if the semantic database contains errors.
    pub fn compile_checked(&self, procs: &[&AnnProc<'a>]) -> CompileResult<Vec<Process>> {
        self.check_errors()?;
        self.compile_unchecked(procs)
    }

    /// Compile without error checking (internal use or when errors are pre-validated)
    ///
    /// # Safety
    ///
    /// Caller must ensure semantic analysis has passed without errors.
    /// Use `compile_checked` for normal compilation.
    pub fn compile_unchecked(&self, procs: &[&AnnProc<'a>]) -> CompileResult<Vec<Process>> {
        let mut results = Vec::with_capacity(procs.len());

        for (idx, proc) in procs.iter().enumerate() {
            let mut ctx = CodegenContext::new(self.db, idx);
            ctx.compile_proc(proc).map_err(CompileError::CodegenError)?;
            results.push(ctx.finalize().map_err(CompileError::CodegenError)?);
        }

        Ok(results)
    }

    /// Check for semantic errors, returning an error if any exist
    fn check_errors(&self) -> CompileResult<()> {
        if !self.db.has_errors() {
            return Ok(());
        }

        let error = CompileError::from_diagnostics(self.db, self.db.diagnostics());

        if self.filter_recoverable {
            match error.filter_recoverable() {
                Some(e) => Err(e),
                None => Ok(()), // All errors were recoverable
            }
        } else {
            Err(error)
        }
    }

    /// Compile a single process with error checking
    pub fn compile_single_checked(&self, proc: &AnnProc<'a>) -> CompileResult<Process> {
        self.check_errors()?;
        self.compile_single_unchecked(proc)
    }

    /// Compile a single process without error checking
    pub fn compile_single_unchecked(&self, proc: &AnnProc<'a>) -> CompileResult<Process> {
        let mut ctx = CodegenContext::new(self.db, 0);
        ctx.compile_proc(proc).map_err(CompileError::CodegenError)?;
        ctx.finalize().map_err(CompileError::CodegenError)
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
        let refs: Vec<&AnnProc<'a>> = procs.iter().collect();
        self.compile_checked(&refs).map_err(Into::into)
    }

    /// Compile a single top-level process into an executable bytecode process
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails (see `compile` for details)
    pub fn compile_single(&self, proc: &AnnProc<'a>) -> Result<Process> {
        self.compile_single_checked(proc).map_err(Into::into)
    }
}
