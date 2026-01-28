use anyhow::Result;
use librho::sem::{
    pipeline::Pipeline, DiagnosticKind, EnclosureAnalysisPass, ErrorKind, ForCompElaborationPass,
    ResolverPass, SemanticDb,
};
use rholang_compiler::Compiler;
use rholang_parser::parser::RholangParser;
use rholang_vm::{api::Value, VM};
use validated::Validated;

/// Compile and run a Rholang source string, returning the final result
///
/// This helper function:
/// 1. Parses the source code
/// 2. Runs semantic analysis (resolver and enclosure analysis)
/// 3. Compiles to bytecode
/// 4. Executes on the VM
/// 5. Returns the final value
///
/// # Errors
///
/// Returns an error if parsing, semantic analysis, compilation, or execution fails.
#[allow(dead_code)]
pub fn compile_and_run(source: &str) -> Result<Value> {
    // Parse
    let parser = RholangParser::new();
    let ast = match parser.parse(source) {
        Validated::Good(procs) => procs,
        Validated::Fail(err) => {
            return Err(anyhow::anyhow!("Parse error: {:?}", err));
        }
    };

    // For simplicity, we assume there's at least one process
    if ast.is_empty() {
        return Err(anyhow::anyhow!("Empty AST"));
    }

    // Semantic analysis - build index for first process
    let mut db = SemanticDb::new();
    let root = db.build_index(&ast[0]);

    let pipeline = Pipeline::new()
        .add_fact(ResolverPass::new(root))
        .add_fact(ForCompElaborationPass::new(root))
        .add_fact(EnclosureAnalysisPass::new(root));

    // Run pipeline (async, but we block on it)
    tokio::runtime::Runtime::new()?.block_on(pipeline.run(&mut db));

    // Filter out NameInProcPosition errors - these represent implicit eval
    // which handled in the compiler by auto-emitting EVAL instructions
    let real_errors: Vec<_> = db
        .errors()
        .filter(|diag| {
            !matches!(
                diag.kind,
                DiagnosticKind::Error(ErrorKind::NameInProcPosition(_, _))
            )
        })
        .collect();

    if !real_errors.is_empty() {
        return Err(anyhow::anyhow!("Semantic errors: {:?}", real_errors));
    }

    // Compile
    let compiler = Compiler::new(&db);
    let mut processes = compiler.compile(&ast)?;

    // Execute
    processes[0].vm = Some(VM::new());
    let result = processes[0].execute()?;

    Ok(result)
}
