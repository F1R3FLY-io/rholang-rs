//! High-level compilation driver that orchestrates the full pipeline

use crate::error::{CompileError, CompileResult};
use crate::reporter::{ErrorReporter, ReporterConfig};
use crate::Compiler;
use librho::sem::{
    pipeline::Pipeline, EnclosureAnalysisPass, ForCompElaborationPass, ResolverPass, SemanticDb,
    PID,
};
use rholang_parser::RholangParser;
use rholang_vm::api::Process;
use validated::Validated;

/// Pipeline builder function type
pub type PipelineBuilder = Box<dyn Fn(&[PID]) -> Pipeline>;

/// Options for the compilation driver
#[derive(Default)]
pub struct CompileOptions {
    /// Custom pipeline builder - if None, uses default passes
    pub pipeline_builder: Option<PipelineBuilder>,
    /// Enable warning diagnostics (unused variables, shadows, etc.)
    pub enable_warnings: bool,
    /// Treat warnings as errors
    pub warnings_as_errors: bool,
    /// Enable strict mode (no recoverable error filtering)
    pub strict_mode: bool,
    /// Reporter configuration for error display
    pub reporter_config: ReporterConfig,
}

#[derive(Debug)]
pub struct CompileOutput {
    /// The compiled bytecode processes
    pub processes: Vec<Process>,
    /// Warning messages
    pub warnings: Vec<String>,
    /// The semantic database (for inspection)
    pub db_stats: DbStats,
}

/// Statistics about the semantic database
#[derive(Debug, Clone)]
pub struct DbStats {
    pub process_count: usize,
    pub binder_count: usize,
    pub scope_count: usize,
    pub diagnostic_count: usize,
}

/// High-level compilation driver
pub struct CompileDriver {
    options: CompileOptions,
    reporter: ErrorReporter,
}

impl CompileDriver {
    pub fn new(options: CompileOptions) -> Self {
        let reporter = ErrorReporter::new(options.reporter_config.clone());
        Self { options, reporter }
    }

    /// Compile source code to bytecode processes
    pub async fn compile_async(&self, source: &str) -> CompileResult<CompileOutput> {
        self.compile_async_with_filename(source, None).await
    }

    /// Compile source code with a filename for error reporting
    pub async fn compile_async_with_filename(
        &self,
        source: &str,
        filename: Option<&str>,
    ) -> CompileResult<CompileOutput> {
        // Phase 1: Parse
        let parser = RholangParser::new();
        let validated = parser.parse(source);

        let ast_vec = match validated {
            Validated::Good(ast) => ast,
            Validated::Fail(errors) => {
                return Err(CompileError::ParseError(format!(
                    "Parse errors: {:?}",
                    errors
                )));
            }
        };

        if ast_vec.is_empty() {
            return Err(CompileError::ParseError("Empty source code".to_string()));
        }

        // Phase 2: Build semantic database
        let mut db = SemanticDb::new();
        let mut roots: Vec<PID> = Vec::with_capacity(ast_vec.len());

        for proc in &ast_vec {
            roots.push(db.build_index(proc));
        }

        // Phase 3: Run semantic pipeline
        let pipeline = self.build_pipeline(&roots);
        pipeline.run(&mut db).await;

        // Phase 4: Check for errors and compile
        let compiler = if self.options.strict_mode {
            Compiler::strict(&db)
        } else {
            Compiler::new(&db)
        };

        // Collect warnings (formatted through reporter)
        let warnings: Vec<String> = if self.options.enable_warnings {
            db.warnings()
                .map(|w| self.reporter.format_warning(w, &db, source, filename))
                .collect()
        } else {
            vec![]
        };

        // Compile (errors are returned, not printed)
        let procs_ref: Vec<&_> = ast_vec.iter().collect();
        let processes = compiler.compile_checked(&procs_ref)?;

        Ok(CompileOutput {
            processes,
            warnings,
            db_stats: DbStats {
                process_count: db.pid_count(),
                binder_count: 0,
                scope_count: 0,
                diagnostic_count: db.diagnostics().len(),
            },
        })
    }

    fn build_pipeline(&self, roots: &[PID]) -> Pipeline {
        if let Some(ref builder) = self.options.pipeline_builder {
            builder(roots)
        } else {
            // Default pipeline
            let mut pipeline = Pipeline::new();
            for &root in roots {
                // TODO: change after for-comp PR merge
                pipeline = pipeline
                    .add_fact(ResolverPass::new(root))
                    .add_fact(EnclosureAnalysisPass::new(root))
                    .add_fact(ForCompElaborationPass::new(root));
            }
            pipeline
        }
    }

    pub fn reporter(&self) -> &ErrorReporter {
        &self.reporter
    }
}

impl Default for CompileDriver {
    fn default() -> Self {
        Self::new(CompileOptions::default())
    }
}
