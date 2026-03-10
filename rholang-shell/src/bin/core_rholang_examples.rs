//! Core Rholang Examples - Run, compile, disassemble and execute Rholang code
//!
//! This binary can:
//! - Run all built-in examples with `--all`
//! - Run a specific example by name with `--example <name>`
//! - List all available examples with `--list`
//! - Process Rholang code from stdin with `--stdin`
//! - Process Rholang code from a file with `--file <path>`
//!
//! Usage:
//!   cargo run --bin core_rholang_examples -- --all
//!   cargo run --bin core_rholang_examples -- --example "Addition"
//!   cargo run --bin core_rholang_examples -- --list
//!   cargo run --bin core_rholang_examples -- --file path/to/code.rho
//!   echo "1 + 2" | cargo run --bin core_rholang_examples -- --stdin

use anyhow::Result;
use clap::Parser;
use librho::sem::{
    pipeline::Pipeline, DiagnosticKind, EnclosureAnalysisPass, ErrorKind, ForCompElaborationPass,
    ResolverPass, SemanticDb,
};
use rholang_compiler::{Compiler, Disassembler, DisassemblyFormat, Process};
use rholang_parser::parser::RholangParser;
use rholang_vm::api::Value;
use std::fs;
use std::io::{self, BufRead};
use validated::Validated;

/// Core Rholang Examples - Compile, disassemble, and execute Rholang code
#[derive(Parser, Debug)]
#[command(name = "core_rholang_examples")]
#[command(about = "Run, compile, disassemble and execute Rholang code examples")]
struct Args {
    /// Run all built-in examples only (without example files)
    #[arg(long)]
    all: bool,

    /// Run a specific example by name
    #[arg(long)]
    example: Option<String>,

    /// List all available examples
    #[arg(long)]
    list: bool,

    /// Read Rholang code from stdin
    #[arg(long)]
    stdin: bool,

    /// Read Rholang code from a file
    #[arg(long, short)]
    file: Option<String>,

    /// Also run all .rho files from rholang-shell/tests/corpus/
    #[arg(long)]
    corpus: bool,

    /// Show disassembly output
    #[arg(long, short, default_value = "true")]
    disassembly: bool,

    /// Output format: markdown or plain
    #[arg(long, default_value = "markdown")]
    format: String,
}

// Shell examples loaded from external files
const COMPLEX_EXAMPLE: &str = include_str!("../../tests/examples/complex_example.rho");
const MAXIMUM_COMPLEXITY: &str = include_str!("../../tests/examples/maximum_complexity.rho");

/// Built-in examples with name and code
fn get_examples() -> Vec<(&'static str, &'static str)> {
    vec![
        // Literals
        ("Nil Literal", "Nil"),
        ("Integer Literal", "42"),
        ("Negative Integer", "-123"),
        ("Boolean True", "true"),
        ("Boolean False", "false"),
        ("String Literal", r#""hello world""#),
        ("Empty String", r#""""#),
        // Arithmetic
        ("Addition", "1 + 2"),
        ("Subtraction", "10 - 3"),
        ("Multiplication", "4 * 5"),
        ("Division", "20 / 4"),
        ("Complex Arithmetic", "(1 + 2) * (3 + 4)"),
        // Comparisons
        ("Equality", "1 == 1"),
        ("Inequality", "1 != 2"),
        ("Less Than", "1 < 2"),
        ("Greater Than", "2 > 1"),
        ("Less or Equal", "1 <= 1"),
        ("Greater or Equal", "2 >= 2"),
        // Boolean Operations
        ("Boolean And", "true and true"),
        ("Boolean Or", "true or false"),
        ("Complex Boolean", "(1 < 2) and (3 > 2)"),
        // Collections
        ("List", "[1, 2, 3]"),
        ("Empty List", "[]"),
        ("Nested List", "[[1, 2], [3, 4]]"),
        ("Tuple", "(1, 2, 3)"),
        ("Empty Tuple", "()"),
        ("Mixed Tuple", r#"(true, 42, "hello")"#),
        // Control Flow
        ("If True Branch", "if (true) { 1 } else { 2 }"),
        ("If False Branch", "if (false) { 1 } else { 2 }"),
        (
            "If With Comparison",
            r#"if (1 < 2) { "yes" } else { "no" }"#,
        ),
        // Parallel Composition
        ("Parallel", "1 | 2"),
        ("Multiple Parallel", "Nil | Nil | 42"),
        // Channels
        ("New Channel", "new x in { Nil }"),
        (
            "Send and Receive",
            "new x in { x!(42) | for (y <- x) { y } }",
        ),
        (
            "Multiple Channels",
            "new a, b in { a!(1) | b!(2) | for (x <- a) { x } }",
        ),
        // Shell examples (from rholang-shell/tests/examples/)
        ("Complex Example", COMPLEX_EXAMPLE),
        ("Maximum Complexity", MAXIMUM_COMPLEXITY),
    ]
}

/// Compile Rholang source and return (Process, disassembly string)
pub fn compile_and_disassemble(source: &str) -> Result<(Process, String)> {
    let parser = RholangParser::new();
    let ast = match parser.parse(source) {
        Validated::Good(procs) => procs,
        Validated::Fail(err) => return Err(anyhow::anyhow!("Parse error: {:?}", err)),
    };

    if ast.is_empty() {
        return Err(anyhow::anyhow!("Empty AST"));
    }

    let mut db = SemanticDb::new();
    let root = db.build_index(&ast[0]);

    let pipeline = Pipeline::new()
        .add_fact(ResolverPass::new(root))
        .add_fact(ForCompElaborationPass::new(root))
        .add_fact(EnclosureAnalysisPass::new(root));

    tokio::runtime::Runtime::new()?.block_on(pipeline.run(&mut db));

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

    let compiler = Compiler::new(&db);
    let process = compiler.compile_single(&ast[0])?;

    // Disassemble with verbose format
    let disasm = Disassembler::with_format(DisassemblyFormat::Verbose)
        .show_addresses(true)
        .show_comments(true)
        .disassemble(&process);

    Ok((process, disasm))
}

/// Compile and run Rholang code, returning the result value
pub fn compile_and_run(source: &str) -> Result<Value> {
    let (mut process, _) = compile_and_disassemble(source)?;
    // VM is already embedded in Process
    let result = process.execute()?;
    Ok(result)
}

/// Format a Value for display
pub fn format_value(v: &Value) -> String {
    match v {
        Value::Nil => "Nil".to_string(),
        Value::Int(i) => i.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Str(s) => format!("\"{}\"", s),
        Value::List(items) => {
            let inner: Vec<String> = items.iter().map(format_value).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Tuple(items) => {
            let inner: Vec<String> = items.iter().map(format_value).collect();
            format!("({})", inner.join(", "))
        }
        Value::Name(n) => format!("@\"{}\"", n),
        Value::Map(m) => {
            let inner: Vec<String> = m
                .iter()
                .map(|(k, v)| format!("{}: {}", format_value(k), format_value(v)))
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
        Value::Par(ps) => {
            let inner: Vec<String> = ps.iter().map(|p| format!("<{}>", p.source_ref())).collect();
            inner.join(" | ")
        }
    }
}

/// Process and display a single example
fn process_example(name: &str, code: &str, show_disassembly: bool, format: &str) {
    if format == "markdown" {
        println!("### {}\n", name);
        println!("**Code:**");
        println!("```rholang");
        println!("{}", code);
        println!("```\n");
    } else {
        println!("=== {} ===", name);
        println!("Code: {}", code);
    }

    match compile_and_disassemble(code) {
        Ok((mut process, disasm)) => {
            if show_disassembly {
                if format == "markdown" {
                    println!("**Disassembly:**");
                    println!("```");
                    println!("{}", disasm.trim());
                    println!("```\n");
                } else {
                    println!("Disassembly:\n{}", disasm.trim());
                }
            }

            // Execute (VM is already embedded in Process)
            match process.execute() {
                Ok(result) => {
                    if format == "markdown" {
                        println!("**Result:** `{}`\n", format_value(&result));
                    } else {
                        println!("Result: {}\n", format_value(&result));
                    }
                }
                Err(e) => {
                    if format == "markdown" {
                        println!("**Execution Error:** {}\n", e);
                    } else {
                        println!("Execution Error: {}\n", e);
                    }
                }
            }
        }
        Err(e) => {
            if format == "markdown" {
                println!("**Compilation Error:** {}\n", e);
            } else {
                println!("Compilation Error: {}\n", e);
            }
        }
    }

    if format == "markdown" {
        println!("---\n");
    }
}

/// List all available examples
fn list_examples() {
    println!("Available examples:\n");
    for (name, _) in get_examples() {
        println!("  - {}", name);
    }
}

/// Run all examples
fn run_all_examples(show_disassembly: bool, format: &str) {
    if format == "markdown" {
        println!("# Rholang Examples: Code, Disassembly, and Results\n");
        println!("This document shows Rholang code examples with their compiled bytecode");
        println!("(disassembly) and execution results.\n");
        println!("---\n");
    }

    for (name, code) in get_examples() {
        process_example(name, code, show_disassembly, format);
    }
}

/// Run a specific example by name
fn run_example_by_name(name: &str, show_disassembly: bool, format: &str) -> bool {
    let examples = get_examples();
    for (example_name, code) in &examples {
        if example_name.to_lowercase().contains(&name.to_lowercase()) {
            process_example(example_name, code, show_disassembly, format);
            return true;
        }
    }
    eprintln!(
        "Example '{}' not found. Use --list to see available examples.",
        name
    );
    false
}

/// Process code from stdin
fn process_stdin(show_disassembly: bool, format: &str) -> Result<()> {
    let stdin = io::stdin();
    let mut code = String::new();
    for line in stdin.lock().lines() {
        code.push_str(&line?);
        code.push('\n');
    }
    if !code.trim().is_empty() {
        process_example("stdin", code.trim(), show_disassembly, format);
    }
    Ok(())
}

/// Process code from a file
fn process_file(path: &str, show_disassembly: bool, format: &str) -> Result<()> {
    let code = fs::read_to_string(path)?;
    let name = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);
    process_example(name, &code, show_disassembly, format);
    Ok(())
}

/// Get the project root directory (where Cargo.toml is located)
fn get_project_root() -> Option<std::path::PathBuf> {
    // Try to find project root by looking for Cargo.toml
    let mut current = std::env::current_dir().ok()?;
    loop {
        if current.join("Cargo.toml").exists() {
            // Check if it's the workspace root (has rholang-shell directory)
            if current.join("rholang-shell").exists() {
                return Some(current);
            }
        }
        if !current.pop() {
            break;
        }
    }
    // Fallback: assume we're running from project root
    std::env::current_dir().ok()
}

/// Run all .rho files from a directory
fn run_files_from_directory(
    dir_path: &std::path::Path,
    show_disassembly: bool,
    format: &str,
) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir_path) {
        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rho"))
            .collect();
        files.sort_by_key(|e| e.path());

        for entry in files {
            let path = entry.path();
            if let Ok(code) = fs::read_to_string(&path) {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                process_example(name, &code, show_disassembly, format);
                count += 1;
            }
        }
    }
    count
}

/// Run all built-in examples and example files from tests/examples/
fn run_default(show_disassembly: bool, format: &str, include_corpus: bool) {
    if format == "markdown" {
        println!("# Rholang Examples: Code, Disassembly, and Results\n");
        println!("This document shows Rholang code examples with their compiled bytecode");
        println!("(disassembly) and execution results.\n");
        println!("---\n");
    }

    // Run built-in examples
    if format == "markdown" {
        println!("## Built-in Examples\n");
    } else {
        println!("=== Built-in Examples ===\n");
    }
    for (name, code) in get_examples() {
        process_example(name, code, show_disassembly, format);
    }

    // Run files from rholang-shell/tests/examples/
    if let Some(root) = get_project_root() {
        let examples_dir = root.join("rholang-shell/tests/examples");
        if examples_dir.exists() {
            if format == "markdown" {
                println!("## Example Files (rholang-shell/tests/examples/)\n");
            } else {
                println!("\n=== Example Files (rholang-shell/tests/examples/) ===\n");
            }
            let count = run_files_from_directory(&examples_dir, show_disassembly, format);
            if count == 0 {
                println!("No .rho files found in examples directory.\n");
            }
        }

        // Optionally run corpus files
        if include_corpus {
            let corpus_dir = root.join("rholang-parser/tests/corpus");
            if corpus_dir.exists() {
                if format == "markdown" {
                    println!("## Corpus Files (rholang-parser/tests/corpus/)\n");
                } else {
                    println!("\n=== Corpus Files (rholang-parser/tests/corpus/) ===\n");
                }
                let count = run_files_from_directory(&corpus_dir, show_disassembly, format);
                if count == 0 {
                    println!("No .rho files found in corpus directory.\n");
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.list {
        list_examples();
        return Ok(());
    }

    if args.stdin {
        return process_stdin(args.disassembly, &args.format);
    }

    if let Some(path) = &args.file {
        return process_file(path, args.disassembly, &args.format);
    }

    if let Some(name) = &args.example {
        if !run_example_by_name(name, args.disassembly, &args.format) {
            std::process::exit(1);
        }
        return Ok(());
    }

    if args.all {
        // --all: run only built-in examples (without example files)
        run_all_examples(args.disassembly, &args.format);
        return Ok(());
    }

    // Default (no parameters): run all examples + example files, optionally with corpus
    run_default(args.disassembly, &args.format, args.corpus);

    Ok(())
}

// ============================================================================
// Inline tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_and_run_nil() {
        let result = compile_and_run("Nil").unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn test_compile_and_run_integer() {
        let result = compile_and_run("42").unwrap();
        assert_eq!(result, Value::Int(42));
    }

    #[test]
    fn test_compile_and_run_addition() {
        let result = compile_and_run("1 + 2").unwrap();
        assert_eq!(result, Value::Int(3));
    }

    #[test]
    fn test_compile_and_run_bool() {
        let result = compile_and_run("true").unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_compile_and_run_string() {
        let result = compile_and_run(r#""hello""#).unwrap();
        assert_eq!(result, Value::Str("hello".to_string()));
    }

    #[test]
    fn test_compile_and_run_list() {
        let result = compile_and_run("[1, 2, 3]").unwrap();
        assert_eq!(
            result,
            Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn test_compile_and_run_tuple() {
        let result = compile_and_run("(1, 2)").unwrap();
        assert_eq!(result, Value::Tuple(vec![Value::Int(1), Value::Int(2)]));
    }

    #[test]
    fn test_compile_and_run_if_true() {
        let result = compile_and_run("if (true) { 1 } else { 2 }").unwrap();
        assert_eq!(result, Value::Int(1));
    }

    #[test]
    fn test_compile_and_run_if_false() {
        let result = compile_and_run("if (false) { 1 } else { 2 }").unwrap();
        assert_eq!(result, Value::Int(2));
    }

    #[test]
    fn test_compile_and_disassemble_contains_instructions() {
        let (_, disasm) = compile_and_disassemble("1 + 2").unwrap();
        assert!(disasm.contains("PUSH_INT"));
        assert!(disasm.contains("ADD"));
    }

    #[test]
    fn test_format_value_int() {
        assert_eq!(format_value(&Value::Int(42)), "42");
    }

    #[test]
    fn test_format_value_bool() {
        assert_eq!(format_value(&Value::Bool(true)), "true");
    }

    #[test]
    fn test_format_value_string() {
        assert_eq!(format_value(&Value::Str("hello".to_string())), "\"hello\"");
    }

    #[test]
    fn test_format_value_list() {
        let list = Value::List(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(format_value(&list), "[1, 2]");
    }

    #[test]
    fn test_format_value_tuple() {
        let tuple = Value::Tuple(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(format_value(&tuple), "(1, 2)");
    }

    #[test]
    fn test_format_value_nil() {
        assert_eq!(format_value(&Value::Nil), "Nil");
    }

    #[test]
    fn test_get_examples_not_empty() {
        let examples = get_examples();
        assert!(!examples.is_empty());
    }

    #[test]
    fn test_all_examples_compile_and_run() {
        for (name, code) in get_examples() {
            let result = compile_and_run(code);
            assert!(
                result.is_ok(),
                "Example '{}' failed to compile and run: {:?}",
                name,
                result
            );
        }
    }

    #[test]
    fn test_complex_example_compiles() {
        let result = compile_and_run(COMPLEX_EXAMPLE);
        assert!(result.is_ok(), "Complex example failed: {:?}", result);
    }

    #[test]
    fn test_maximum_complexity_compiles() {
        let result = compile_and_run(MAXIMUM_COMPLEXITY);
        assert!(
            result.is_ok(),
            "Maximum complexity example failed: {:?}",
            result
        );
    }

    #[test]
    fn test_compile_error_on_invalid_syntax() {
        let result = compile_and_run("(");
        assert!(result.is_err());
    }

    #[test]
    fn test_arithmetic_operations() {
        assert_eq!(compile_and_run("10 - 3").unwrap(), Value::Int(7));
        assert_eq!(compile_and_run("4 * 5").unwrap(), Value::Int(20));
        assert_eq!(compile_and_run("20 / 4").unwrap(), Value::Int(5));
    }

    #[test]
    fn test_comparison_operations() {
        assert_eq!(compile_and_run("1 == 1").unwrap(), Value::Bool(true));
        assert_eq!(compile_and_run("1 != 2").unwrap(), Value::Bool(true));
        assert_eq!(compile_and_run("1 < 2").unwrap(), Value::Bool(true));
        assert_eq!(compile_and_run("2 > 1").unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_boolean_operations() {
        assert_eq!(compile_and_run("true and true").unwrap(), Value::Bool(true));
        assert_eq!(
            compile_and_run("true and false").unwrap(),
            Value::Bool(false)
        );
        assert_eq!(compile_and_run("true or false").unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_channel_operations() {
        // Simple new channel
        let result = compile_and_run("new x in { Nil }").unwrap();
        assert_eq!(result, Value::Nil);
    }
}
