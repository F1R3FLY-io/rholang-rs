use anyhow::Result;

use rholang_shell::providers::{
    FakeInterpreterProvider, InterpreterProvider, RholangCompilerInterpreterProvider,
    RholangParserInterpreterProvider,
};

// Basic smoke tests for disassemble() across providers.

#[test]
fn disassemble_unsupported_in_fake_provider() {
    let fake = FakeInterpreterProvider;
    let err = fake.disassemble("Nil").unwrap_err();
    assert!(format!("{}", err).contains("Disassembly not available in FakeInterpreterProvider"));
}

#[test]
fn disassemble_unsupported_in_parser_provider() -> Result<()> {
    let parser = RholangParserInterpreterProvider::new()?;
    let err = parser.disassemble("Nil").unwrap_err();
    assert!(format!("{}", err)
        .contains("Disassembly not available in RholangParserInterpreterProvider"));
    Ok(())
}

#[test]
fn disassemble_compiler_empty_input() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("")?;
    assert_eq!(out, "No code to disassemble (empty AST)");
    Ok(())
}

#[test]
fn disassemble_compiler_parse_error() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    // Use an obviously malformed fragment that the parser cannot recover from
    let out = compiler.disassemble("(")?;
    assert!(out.starts_with("Parsing failed: unable to build AST"));
    Ok(())
}

#[test]
fn disassemble_compiler_success_nil() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("Nil")?;
    // We don't assert the exact disassembly to avoid brittleness across versions.
    // Just ensure it is non-empty and does not look like an error marker used by provider.
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Semantic errors:"));
    assert!(!out.starts_with("Compilation error:"));
    Ok(())
}

// Regression: calling disassemble inside a running Tokio runtime should not panic
#[tokio::test]
async fn disassemble_inside_tokio_runtime() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("Nil")?;
    assert!(!out.trim().is_empty());
    Ok(())
}

// Test disassembly of various Rholang constructs covered by the compiler

#[test]
fn disassemble_compiler_bool_true() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("true")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain PUSH_BOOL instruction (true is value 1)
    assert!(out.contains("PUSH_BOOL"));
    Ok(())
}

#[test]
fn disassemble_compiler_bool_false() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("false")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain PUSH_BOOL instruction (false is value 0)
    assert!(out.contains("PUSH_BOOL"));
    Ok(())
}

#[test]
fn disassemble_compiler_integer() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("42")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain PUSH_INT instruction
    assert!(out.contains("PUSH_INT"));
    Ok(())
}

#[test]
fn disassemble_compiler_negative_integer() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("-123")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain PUSH_INT instruction
    assert!(out.contains("PUSH_INT"));
    Ok(())
}

#[test]
fn disassemble_compiler_string() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble(r#""hello world""#)?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain PUSH_STR instruction and string in pool
    assert!(out.contains("PUSH_STR") || out.contains("hello world"));
    Ok(())
}

#[test]
fn disassemble_compiler_new_channel() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("new x in { Nil }")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain NAME_CREATE instruction for new channel
    assert!(out.contains("NAME_CREATE"));
    Ok(())
}

#[test]
fn disassemble_compiler_send() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("new x in { x!(42) }")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain TELL instruction for send
    assert!(out.contains("TELL"));
    Ok(())
}

#[test]
fn disassemble_compiler_receive() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("new x in { for (y <- x) { Nil } }")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain ASK instruction for receive
    assert!(out.contains("ASK"));
    Ok(())
}

#[test]
fn disassemble_compiler_par() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    // Note: Nil | Nil is optimized by the compiler to sequential PUSH_NIL instructions
    // The par construct doesn't generate SPAWN_ASYNC for trivial Nil processes
    let out = compiler.disassemble("Nil | Nil")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain PUSH_NIL instructions for both sides of par
    assert!(out.contains("PUSH_NIL"));
    Ok(())
}

#[test]
fn disassemble_compiler_arithmetic() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("1 + 2")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain ADD instruction
    assert!(out.contains("ADD"));
    Ok(())
}

#[test]
fn disassemble_compiler_multiplication() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("3 * 4")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain MUL instruction
    assert!(out.contains("MUL"));
    Ok(())
}

#[test]
fn disassemble_compiler_if_then_else() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("if (true) { Nil } else { Nil }")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain BRANCH_FALSE or JUMP instruction for control flow
    assert!(out.contains("BRANCH_FALSE") || out.contains("JUMP"));
    Ok(())
}

#[test]
fn disassemble_compiler_list() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    let out = compiler.disassemble("[1, 2, 3]")?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"));
    // Should contain CREATE_LIST instruction
    assert!(out.contains("CREATE_LIST"));
    Ok(())
}

#[test]
fn disassemble_compiler_complex_example() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    // A complex example using only supported constructs in the MVP compiler:
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
    let out = compiler.disassemble(code)?;
    assert!(!out.trim().is_empty());
    assert!(!out.starts_with("Parsing failed:"));
    assert!(!out.starts_with("Compilation error:"), "Got: {}", out);
    assert!(!out.starts_with("Semantic errors:"), "Got: {}", out);
    // Verify key instructions are present
    assert!(
        out.contains("NAME_CREATE"),
        "Expected NAME_CREATE for new channels"
    );
    assert!(out.contains("TELL"), "Expected TELL for send operations");
    Ok(())
}

/// Maximum complexity disassembly test - exercises all supported compiler features
/// This test creates the longest possible bytecode output with the current MVP compiler
#[test]
fn disassemble_compiler_maximum_complexity() -> Result<()> {
    let compiler = RholangCompilerInterpreterProvider::new()?;
    // Comprehensive example exercising ALL supported MVP compiler constructs:
    // 1. Multiple nested new declarations
    // 2. Multiple send operations with different data types
    // 3. Multiple for comprehensions (receives)
    // 4. Parallel composition at multiple levels
    // 5. All arithmetic operators: +, -, *, /, %
    // 6. All comparison operators: ==, !=, <, >, <=, >=
    // 7. Boolean operators: and, or, not
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
            
            // Section 3: Arithmetic expressions (Note: % modulo not supported in MVP)
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
            
            // Section 5: Boolean operations (Note: 'not' unary operator is not supported in MVP)
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
            
            // Section 13: Edge cases (integers limited to i16 range: -32768 to 32767)
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

    let out = compiler.disassemble(code)?;

    // Print full disassembly for inspection
    eprintln!("=== MAXIMUM COMPLEXITY DISASSEMBLY ===");
    eprintln!("{}", out);
    eprintln!("=== END DISASSEMBLY ===");

    // Verify successful compilation
    assert!(!out.trim().is_empty(), "Output should not be empty");
    assert!(
        !out.starts_with("Parsing failed:"),
        "Should parse successfully"
    );
    assert!(
        !out.starts_with("Compilation error:"),
        "Should compile successfully: {}",
        out
    );
    assert!(
        !out.starts_with("Semantic errors:"),
        "Should have no semantic errors: {}",
        out
    );

    // Verify all major instruction types are present
    assert!(
        out.contains("NAME_CREATE"),
        "Expected NAME_CREATE for new channels"
    );
    assert!(out.contains("TELL"), "Expected TELL for send operations");
    assert!(out.contains("ASK"), "Expected ASK for receive operations");
    assert!(out.contains("PUSH_INT"), "Expected PUSH_INT for integers");
    assert!(out.contains("PUSH_STR"), "Expected PUSH_STR for strings");
    assert!(out.contains("PUSH_BOOL"), "Expected PUSH_BOOL for booleans");
    assert!(out.contains("PUSH_NIL"), "Expected PUSH_NIL for Nil");
    assert!(out.contains("ADD"), "Expected ADD for addition");
    assert!(out.contains("SUB"), "Expected SUB for subtraction");
    assert!(out.contains("MUL"), "Expected MUL for multiplication");
    assert!(out.contains("DIV"), "Expected DIV for division");
    // Note: MOD (modulo) is not supported in MVP compiler
    assert!(out.contains("CMP_EQ"), "Expected CMP_EQ for equality");
    assert!(out.contains("CMP_NEQ"), "Expected CMP_NEQ for inequality");
    assert!(out.contains("CMP_LT"), "Expected CMP_LT for less than");
    assert!(out.contains("CMP_GT"), "Expected CMP_GT for greater than");
    assert!(
        out.contains("CMP_LTE"),
        "Expected CMP_LTE for less or equal"
    );
    assert!(
        out.contains("CMP_GTE"),
        "Expected CMP_GTE for greater or equal"
    );
    assert!(out.contains("AND"), "Expected AND for boolean and");
    assert!(out.contains("OR"), "Expected OR for boolean or");
    // Note: NOT (unary not) is not supported in MVP compiler
    assert!(
        out.contains("CREATE_LIST"),
        "Expected CREATE_LIST for lists"
    );
    assert!(
        out.contains("CREATE_TUPLE"),
        "Expected CREATE_TUPLE for tuples"
    );
    assert!(
        out.contains("BRANCH_FALSE"),
        "Expected BRANCH_FALSE for conditionals"
    );
    assert!(out.contains("JUMP"), "Expected JUMP for control flow");
    assert!(out.contains("HALT"), "Expected HALT at end");

    // Verify instruction count is substantial (should be > 200 instructions)
    let instruction_count_line = out
        .lines()
        .find(|line| line.starts_with("Instructions:"))
        .expect("Should have instruction count");
    let count: usize = instruction_count_line
        .trim_start_matches("Instructions: ")
        .trim()
        .parse()
        .expect("Should parse instruction count");
    assert!(count > 200, "Expected > 200 instructions, got {}", count);

    eprintln!("Total instructions generated: {}", count);

    Ok(())
}
