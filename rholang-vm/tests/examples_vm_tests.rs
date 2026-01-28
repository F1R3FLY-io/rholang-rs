//! Tests for running Rholang examples on the VM
//!
//! These tests compile Rholang source code and execute it on the VM,
//! verifying that all supported constructs run without errors.
//! Each test shows the source code inline for clarity.

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
fn compile_and_run(source: &str) -> Result<Value> {
    // Parse
    let parser = RholangParser::new();
    let ast = match parser.parse(source) {
        Validated::Good(procs) => procs,
        Validated::Fail(err) => {
            return Err(anyhow::anyhow!("Parse error: {:?}", err));
        }
    };

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
    // which is handled in the compiler by auto-emitting EVAL instructions
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

// ============================================================================
// Basic constructs tests
// ============================================================================

#[test]
fn test_vm_nil() {
    // Code: Nil
    let result = compile_and_run("Nil").unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_vm_bool_true() {
    // Code: true
    let result = compile_and_run("true").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vm_bool_false() {
    // Code: false
    let result = compile_and_run("false").unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_vm_integer() {
    // Code: 42
    let result = compile_and_run("42").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_vm_negative_integer() {
    // Code: -123
    let result = compile_and_run("-123").unwrap();
    assert_eq!(result, Value::Int(-123));
}

#[test]
fn test_vm_string() {
    // Code: "hello world"
    let result = compile_and_run(r#""hello world""#).unwrap();
    assert_eq!(result, Value::Str("hello world".to_string()));
}

// ============================================================================
// Arithmetic tests
// ============================================================================

#[test]
fn test_vm_addition() {
    // Code: 1 + 2
    let result = compile_and_run("1 + 2").unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_vm_subtraction() {
    // Code: 10 - 3
    let result = compile_and_run("10 - 3").unwrap();
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_vm_multiplication() {
    // Code: 4 * 5
    let result = compile_and_run("4 * 5").unwrap();
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_vm_division() {
    // Code: 20 / 4
    let result = compile_and_run("20 / 4").unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_vm_complex_arithmetic() {
    // Code: (1 + 2) * (3 + 4)
    let result = compile_and_run("(1 + 2) * (3 + 4)").unwrap();
    assert_eq!(result, Value::Int(21));
}

#[test]
fn test_vm_chained_arithmetic() {
    // Code: ((10 - 2) * 3) + 1
    let result = compile_and_run("((10 - 2) * 3) + 1").unwrap();
    assert_eq!(result, Value::Int(25));
}

#[test]
fn test_vm_division_chain() {
    // Code: 100 / 10 / 2
    let result = compile_and_run("100 / 10 / 2").unwrap();
    assert_eq!(result, Value::Int(5));
}

// ============================================================================
// Comparison tests
// ============================================================================

#[test]
fn test_vm_equality() {
    // Code: 1 == 1
    let result = compile_and_run("1 == 1").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vm_inequality() {
    // Code: 1 != 2
    let result = compile_and_run("1 != 2").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vm_less_than() {
    // Code: 1 < 2
    let result = compile_and_run("1 < 2").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vm_greater_than() {
    // Code: 2 > 1
    let result = compile_and_run("2 > 1").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vm_less_or_equal() {
    // Code: 1 <= 1
    let result = compile_and_run("1 <= 1").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vm_greater_or_equal() {
    // Code: 2 >= 2
    let result = compile_and_run("2 >= 2").unwrap();
    assert_eq!(result, Value::Bool(true));
}

// ============================================================================
// Boolean operations tests
// ============================================================================

#[test]
fn test_vm_and() {
    // Code: true and true
    let result = compile_and_run("true and true").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vm_and_false() {
    // Code: true and false
    let result = compile_and_run("true and false").unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_vm_or() {
    // Code: true or false
    let result = compile_and_run("true or false").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vm_or_false() {
    // Code: false or false
    let result = compile_and_run("false or false").unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_vm_complex_boolean() {
    // Code: (1 < 2) and (3 > 2)
    let result = compile_and_run("(1 < 2) and (3 > 2)").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vm_boolean_chain_and() {
    // Code: (1 < 2) and (2 < 3) and (3 < 4)
    let result = compile_and_run("(1 < 2) and (2 < 3) and (3 < 4)").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_vm_boolean_chain_or() {
    // Code: (1 > 2) or (2 > 3) or (3 < 4)
    let result = compile_and_run("(1 > 2) or (2 > 3) or (3 < 4)").unwrap();
    assert_eq!(result, Value::Bool(true));
}

// ============================================================================
// Collection tests - Lists
// ============================================================================

#[test]
fn test_vm_list() {
    // Code: [1, 2, 3]
    let result = compile_and_run("[1, 2, 3]").unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_vm_empty_list() {
    // Code: []
    let result = compile_and_run("[]").unwrap();
    assert_eq!(result, Value::List(vec![]));
}

#[test]
fn test_vm_string_list() {
    // Code: ["a", "b", "c"]
    let result = compile_and_run(r#"["a", "b", "c"]"#).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Str("a".to_string()),
            Value::Str("b".to_string()),
            Value::Str("c".to_string())
        ])
    );
}

#[test]
fn test_vm_boolean_list() {
    // Code: [true, false, true]
    let result = compile_and_run("[true, false, true]").unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Bool(true),
            Value::Bool(false),
            Value::Bool(true)
        ])
    );
}

#[test]
fn test_vm_mixed_list() {
    // Code: [1, "mixed", true]
    let result = compile_and_run(r#"[1, "mixed", true]"#).unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::Int(1),
            Value::Str("mixed".to_string()),
            Value::Bool(true)
        ])
    );
}

#[test]
fn test_vm_nested_list() {
    // Code: [[1, 2], [3, 4]]
    let result = compile_and_run("[[1, 2], [3, 4]]").unwrap();
    assert_eq!(
        result,
        Value::List(vec![
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::List(vec![Value::Int(3), Value::Int(4)])
        ])
    );
}

#[test]
fn test_vm_list_with_expressions() {
    // Code: [(1 + 1), (2 + 2), (3 + 3)]
    let result = compile_and_run("[(1 + 1), (2 + 2), (3 + 3)]").unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(2), Value::Int(4), Value::Int(6)])
    );
}

// ============================================================================
// Collection tests - Tuples
// ============================================================================

#[test]
fn test_vm_tuple() {
    // Code: (1, 2)
    let result = compile_and_run("(1, 2)").unwrap();
    assert_eq!(result, Value::Tuple(vec![Value::Int(1), Value::Int(2)]));
}

#[test]
fn test_vm_empty_tuple() {
    // Code: ()
    let result = compile_and_run("()").unwrap();
    assert_eq!(result, Value::Tuple(vec![]));
}

#[test]
fn test_vm_triple_tuple() {
    // Code: (1, 2, 3)
    let result = compile_and_run("(1, 2, 3)").unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_vm_string_tuple() {
    // Code: ("x", "y", "z")
    let result = compile_and_run(r#"("x", "y", "z")"#).unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![
            Value::Str("x".to_string()),
            Value::Str("y".to_string()),
            Value::Str("z".to_string())
        ])
    );
}

#[test]
fn test_vm_mixed_tuple() {
    // Code: (true, 42, "hello")
    let result = compile_and_run(r#"(true, 42, "hello")"#).unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![
            Value::Bool(true),
            Value::Int(42),
            Value::Str("hello".to_string())
        ])
    );
}

#[test]
fn test_vm_nested_tuple() {
    // Code: ((1, 2), (3, 4))
    let result = compile_and_run("((1, 2), (3, 4))").unwrap();
    assert_eq!(
        result,
        Value::Tuple(vec![
            Value::Tuple(vec![Value::Int(1), Value::Int(2)]),
            Value::Tuple(vec![Value::Int(3), Value::Int(4)])
        ])
    );
}

#[test]
fn test_vm_tuple_with_expression() {
    // Code: (100 - 50, 200 / 4)
    let result = compile_and_run("(100 - 50, 200 / 4)").unwrap();
    assert_eq!(result, Value::Tuple(vec![Value::Int(50), Value::Int(50)]));
}

// ============================================================================
// Control flow tests
// ============================================================================

#[test]
fn test_vm_if_true() {
    // Code: if (true) { 1 } else { 2 }
    let result = compile_and_run("if (true) { 1 } else { 2 }").unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_vm_if_false() {
    // Code: if (false) { 1 } else { 2 }
    let result = compile_and_run("if (false) { 1 } else { 2 }").unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_vm_if_with_comparison() {
    // Code: if (1 < 2) { "yes" } else { "no" }
    let result = compile_and_run(r#"if (1 < 2) { "yes" } else { "no" }"#).unwrap();
    assert_eq!(result, Value::Str("yes".to_string()));
}

#[test]
fn test_vm_nested_if() {
    // Code: if (true) { if (1 < 2) { 10 } else { 20 } } else { 30 }
    let result =
        compile_and_run("if (true) { if (1 < 2) { 10 } else { 20 } } else { 30 }").unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_vm_if_with_expression_condition() {
    // Code: if ((1 + 2) == 3) { "correct" } else { "wrong" }
    let result = compile_and_run(r#"if ((1 + 2) == 3) { "correct" } else { "wrong" }"#).unwrap();
    assert_eq!(result, Value::Str("correct".to_string()));
}

#[test]
fn test_vm_deeply_nested_if() {
    // Code:
    // if (true) {
    //     if (1 < 2) {
    //         "nested-true-true"
    //     } else {
    //         "nested-true-false"
    //     }
    // } else {
    //     if (3 > 4) {
    //         "nested-false-true"
    //     } else {
    //         "nested-false-false"
    //     }
    // }
    let code = r#"
        if (true) {
            if (1 < 2) {
                "nested-true-true"
            } else {
                "nested-true-false"
            }
        } else {
            if (3 > 4) {
                "nested-false-true"
            } else {
                "nested-false-false"
            }
        }
    "#;
    let result = compile_and_run(code).unwrap();
    assert_eq!(result, Value::Str("nested-true-true".to_string()));
}

#[test]
fn test_vm_if_with_complex_condition() {
    // Code: if ((1 + 2) == 3) { if ((4 * 5) > 15) { [(1 + 1), (2 + 2)] } else { 0 } } else { 0 }
    let code =
        "if ((1 + 2) == 3) { if ((4 * 5) > 15) { [(1 + 1), (2 + 2)] } else { 0 } } else { 0 }";
    let result = compile_and_run(code).unwrap();
    assert_eq!(result, Value::List(vec![Value::Int(2), Value::Int(4)]));
}

// ============================================================================
// Parallel composition tests
// ============================================================================

#[test]
fn test_vm_par() {
    // Code: 1 | 2
    let result = compile_and_run("1 | 2").unwrap();
    // Par executes left (discards), then right (returns)
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_vm_par_multiple() {
    // Code: 1 | 2 | 3
    let result = compile_and_run("1 | 2 | 3").unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_vm_nil_in_par() {
    // Code: Nil | Nil | Nil
    let result = compile_and_run("Nil | Nil | Nil").unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_vm_long_par_chain() {
    // Code: Nil | Nil | Nil | Nil | Nil | 42
    let result = compile_and_run("Nil | Nil | Nil | Nil | Nil | 42").unwrap();
    assert_eq!(result, Value::Int(42));
}

// ============================================================================
// Channel operations tests
// ============================================================================

#[test]
fn test_vm_new_channel() {
    // Code: new x in { Nil }
    let result = compile_and_run("new x in { Nil }").unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_vm_send_receive() {
    // Code: new x in { x!(42) | for (y <- x) { y } }
    let result = compile_and_run("new x in { x!(42) | for (y <- x) { y } }").unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_vm_multiple_sends() {
    // Code: new x in { x!(1) | x!(2) | for (a <- x) { a } }
    let result = compile_and_run(r#"new x in { x!(1) | x!(2) | for (a <- x) { a } }"#).unwrap();
    // Should receive first message 1
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_vm_send_string() {
    // Code: new ch in { ch!("hello") | for (msg <- ch) { msg } }
    let result =
        compile_and_run(r#"new ch in { ch!("hello") | for (msg <- ch) { msg } }"#).unwrap();
    assert_eq!(result, Value::Str("hello".to_string()));
}

#[test]
fn test_vm_send_list() {
    // Code: new ch in { ch!([1, 2, 3]) | for (data <- ch) { data } }
    let result =
        compile_and_run("new ch in { ch!([1, 2, 3]) | for (data <- ch) { data } }").unwrap();
    assert_eq!(
        result,
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
    );
}

#[test]
fn test_vm_multiple_channels() {
    // Code: new ch1, ch2 in { ch1!(10) | ch2!(20) | for (x <- ch1) { x } }
    let result =
        compile_and_run("new ch1, ch2 in { ch1!(10) | ch2!(20) | for (x <- ch1) { x } }").unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_vm_nested_new() {
    // Code:
    // new outer in {
    //     new inner in {
    //         inner!(42) |
    //         for (x <- inner) { x }
    //     }
    // }
    let code = r#"
        new outer in {
            new inner in {
                inner!(42) |
                for (x <- inner) { x }
            }
        }
    "#;
    let result = compile_and_run(code).unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_vm_deeply_nested_new() {
    // Code:
    // new a in {
    //     new b in {
    //         new c in {
    //             c!("deep") |
    //             for (x <- c) { x }
    //         }
    //     }
    // }
    let code = r#"
        new a in {
            new b in {
                new c in {
                    c!("deep") |
                    for (x <- c) { x }
                }
            }
        }
    "#;
    let result = compile_and_run(code).unwrap();
    assert_eq!(result, Value::Str("deep".to_string()));
}

// ============================================================================
// Complex example tests
// ============================================================================

#[test]
fn test_vm_complex_example() {
    // Complex example using supported constructs in the MVP compiler:
    // - new channels
    // - send operations
    // - for comprehension (receive)
    // - parallel composition
    // - arithmetic expressions
    // - conditionals
    // - collections (lists, tuples)
    // - string literals
    // - boolean literals
    let code = r#"
        new channel1, channel2, result in {
            channel1!(42) |
            channel1!("hello") |
            channel2!(true) |
            for (x <- channel1) {
                result!(100)
            } |
            if (1 + 2 == 3) {
                result!([1, 2, 3])
            } else {
                result!(false)
            }
        }
    "#;
    // Test passes if it compiles and runs without error
    let result = compile_and_run(code);
    assert!(
        result.is_ok(),
        "Complex example should execute: {:?}",
        result
    );
}

#[test]
fn test_vm_maximum_complexity() {
    // Maximum complexity example - exercises all supported MVP compiler features:
    // 1. Multiple nested new declarations
    // 2. Multiple send operations with different data types
    // 3. Multiple for comprehensions (receives)
    // 4. Parallel composition at multiple levels
    // 5. All arithmetic operators: +, -, *, /
    // 6. All comparison operators: ==, !=, <, >, <=, >=
    // 7. Boolean operators: and, or
    // 8. Nested if-then-else conditionals
    // 9. Collections: lists, tuples
    // 10. Various literal types: integers, strings, booleans
    let code = r#"
        new main, worker1, worker2, worker3, collector, logger in {
            // Section 1: Send various data types
            main!(0) |
            main!(1) |
            main!(2) |
            main!(42) |
            main!(-100) |
            main!(32767) |
            main!("hello") |
            main!("world") |
            main!("rholang") |
            main!("disassembly") |
            main!("test") |
            main!(true) |
            main!(false) |
            
            // Section 2: Multiple workers with receives
            for (a <- worker1) {
                logger!(1)
            } |
            for (b <- worker2) {
                logger!(2)
            } |
            for (c <- worker3) {
                logger!(3)
            } |
            
            // Section 3: Arithmetic expressions
            collector!(1 + 2) |
            collector!(10 - 5) |
            collector!(3 * 4) |
            collector!(20 / 4) |
            collector!((1 + 2) * (3 + 4)) |
            collector!(((10 - 2) * 3) + 1) |
            collector!(100 / 10 / 2) |
            
            // Section 4: Comparison operations
            collector!(1 == 1) |
            collector!(1 != 2) |
            collector!(1 < 2) |
            collector!(2 > 1) |
            collector!(1 <= 1) |
            collector!(2 >= 2) |
            collector!(5 <= 10) |
            collector!(10 >= 5) |
            
            // Section 5: Boolean operations
            collector!(true and true) |
            collector!(true or false) |
            collector!((1 < 2) and (3 > 2)) |
            collector!((1 == 1) or (2 == 3)) |
            
            // Section 6: Nested conditionals
            if (true) {
                if (1 < 2) {
                    logger!("nested-true-true")
                } else {
                    logger!("nested-true-false")
                }
            } else {
                if (3 > 4) {
                    logger!("nested-false-true")
                } else {
                    logger!("nested-false-false")
                }
            } |
            
            // Section 7: Collections - lists
            collector!([1, 2, 3]) |
            collector!([4, 5, 6, 7, 8]) |
            collector!(["a", "b", "c"]) |
            collector!([true, false, true]) |
            collector!([1, "mixed", true]) |
            collector!([[1, 2], [3, 4]]) |
            
            // Section 8: Collections - tuples
            collector!((1, 2)) |
            collector!((1, 2, 3)) |
            collector!(("x", "y", "z")) |
            collector!((true, 42, "hello")) |
            collector!(((1, 2), (3, 4))) |
            
            // Section 9: Complex nested expressions
            if ((1 + 2) == 3) {
                if ((4 * 5) > 15) {
                    collector!([(1 + 1), (2 + 2), (3 + 3)])
                } else {
                    collector!((100 - 50, 200 / 4))
                }
            } else {
                collector!(0)
            } |
            
            // Section 10: Nested new with more operations
            new inner1, inner2 in {
                inner1!(1) |
                inner2!(2) |
                for (i <- inner1) {
                    collector!(999)
                } |
                new deepNested in {
                    deepNested!("deep") |
                    for (d <- deepNested) {
                        logger!("received-deep")
                    }
                }
            } |
            
            // Section 11: Long chain of parallel compositions
            Nil | Nil | Nil | Nil | Nil |
            main!(100) | main!(101) | main!(102) | main!(103) |
            
            // Section 12: More complex boolean chains
            if ((1 < 2) and (2 < 3) and (3 < 4)) {
                collector!("chain-true")
            } else {
                collector!("chain-false")
            } |
            
            if ((1 > 2) or (2 > 3) or (3 < 4)) {
                collector!("or-chain-true")
            } else {
                collector!("or-chain-false")
            } |
            
            // Section 13: Edge cases
            collector!(0) |
            collector!(-1) |
            collector!(-32768) |
            collector!(32767) |
            collector!("") |
            collector!([]) |
            
            // Final marker
            logger!("complete")
        }
    "#;

    // Test passes if it compiles and runs without error
    let result = compile_and_run(code);
    assert!(
        result.is_ok(),
        "Maximum complexity example should execute: {:?}",
        result
    );
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_vm_edge_zero() {
    // Code: 0
    let result = compile_and_run("0").unwrap();
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_vm_edge_negative_one() {
    // Code: -1
    let result = compile_and_run("-1").unwrap();
    assert_eq!(result, Value::Int(-1));
}

#[test]
fn test_vm_edge_max_i16() {
    // Code: 32767
    let result = compile_and_run("32767").unwrap();
    assert_eq!(result, Value::Int(32767));
}

#[test]
fn test_vm_edge_min_i16() {
    // Code: -32768
    let result = compile_and_run("-32768").unwrap();
    assert_eq!(result, Value::Int(-32768));
}

#[test]
fn test_vm_empty_string() {
    // Code: ""
    let result = compile_and_run(r#""""#).unwrap();
    assert_eq!(result, Value::Str("".to_string()));
}

#[test]
fn test_vm_long_string() {
    // Code: "this is a longer string with spaces and punctuation!"
    let result =
        compile_and_run(r#""this is a longer string with spaces and punctuation!""#).unwrap();
    assert_eq!(
        result,
        Value::Str("this is a longer string with spaces and punctuation!".to_string())
    );
}
