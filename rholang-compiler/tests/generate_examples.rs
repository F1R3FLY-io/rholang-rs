//! Generate examples showing code, disassembly, and execution results
//! Run with: cargo test --test generate_examples -- --nocapture

mod common;

use anyhow::Result;
use librho::sem::{
    pipeline::Pipeline, DiagnosticKind, EnclosureAnalysisPass, ErrorKind, ForCompElaborationPass,
    ResolverPass, SemanticDb,
};
use rholang_compiler::Process;
use rholang_compiler::{Compiler, Disassembler, DisassemblyFormat};
use rholang_parser::parser::RholangParser;
use rholang_vm::api::Value;
use validated::Validated;

/// Compile source and return (Process, disassembly string)
fn compile_and_disassemble(source: &str) -> Result<(Process, String)> {
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

fn format_value(v: &Value) -> String {
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

// Shell examples loaded from external files
const COMPLEX_EXAMPLE: &str =
    include_str!("../../rholang-shell/tests/examples/complex_example.rho");
const MAXIMUM_COMPLEXITY: &str =
    include_str!("../../rholang-shell/tests/examples/maximum_complexity.rho");

#[test]
fn generate_examples_markdown() {
    let examples: Vec<(&str, &str)> = vec![
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
        ("Complex Example (from shell tests)", COMPLEX_EXAMPLE),
        ("Maximum Complexity (from shell tests)", MAXIMUM_COMPLEXITY),
    ];

    println!("\n# Rholang Examples: Code, Disassembly, and Results\n");
    println!("This document shows Rholang code examples with their compiled bytecode");
    println!("(disassembly) and execution results.\n");
    println!("---\n");

    for (name, code) in examples {
        println!("### {}\n", name);
        println!("**Code:**");
        println!("```rholang");
        println!("{}", code);
        println!("```\n");

        match compile_and_disassemble(code) {
            Ok((mut process, disasm)) => {
                println!("**Disassembly:**");
                println!("```");
                println!("{}", disasm.trim());
                println!("```\n");

                // Execute (VM is already embedded in Process)
                match process.execute() {
                    Ok(result) => {
                        println!("**Result:** `{}`\n", format_value(&result));
                    }
                    Err(e) => {
                        println!("**Execution Error:** {}\n", e);
                    }
                }
            }
            Err(e) => {
                println!("**Compilation Error:** {}\n", e);
            }
        }
        println!("---\n");
    }
}
