use anyhow::Result;
use tokio::runtime::Runtime;
use rholang_vm::{RholangVM, bytecode::{Instruction, RSpaceType, Label}};
mod test_utils;
use test_utils::run_and_expect_err;

#[test]
fn test_bytecode_arithmetic_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // 5 + 3 => 8
    let program = vec![
        Instruction::PushInt(5),
        Instruction::PushInt(3),
        Instruction::Add,
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Int(8)");

    // (10 + 5) * (20 - 15) / 5 => 15
    let program = vec![
        Instruction::PushInt(10),
        Instruction::PushInt(5),
        Instruction::Add,
        Instruction::PushInt(20),
        Instruction::PushInt(15),
        Instruction::Sub,
        Instruction::Mul,
        Instruction::PushInt(5),
        Instruction::Div,
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Int(15)");

    Ok(())
}

#[test]
fn test_bytecode_comparison_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // 5 == 5 => true
    let program = vec![
        Instruction::PushInt(5),
        Instruction::PushInt(5),
        Instruction::CmpEq,
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Bool(true)");

    // 5 < 10 => true
    let program = vec![
        Instruction::PushInt(5),
        Instruction::PushInt(10),
        Instruction::CmpLt,
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Bool(true)");

    Ok(())
}

#[test]
fn test_bytecode_conditional_branching() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // if (true) { "true branch" } else { "false branch" }
    let else_lbl = Instruction::Label(rholang_vm::bytecode::Label("else".to_string()));
    let end_lbl = Instruction::Label(rholang_vm::bytecode::Label("end".to_string()));

    let program = vec![
        Instruction::PushBool(true),
        Instruction::BranchFalse(rholang_vm::bytecode::Label("else".to_string())),
        Instruction::PushStr("true branch".to_string()),
        Instruction::Jump(rholang_vm::bytecode::Label("end".to_string())),
        else_lbl,
        Instruction::PushStr("false branch".to_string()),
        end_lbl,
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "String(\"true branch\")");

    Ok(())
}

#[test]
fn test_bytecode_data_structures() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // List [1,2,3]
    let program = vec![
        Instruction::PushInt(1),
        Instruction::PushInt(2),
        Instruction::PushInt(3),
        Instruction::CreateList(3),
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "List([Int(1), Int(2), Int(3)])");

    // Tuple (1, "hello", true)
    let program = vec![
        Instruction::PushInt(1),
        Instruction::PushStr("hello".to_string()),
        Instruction::PushBool(true),
        Instruction::CreateTuple(3),
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Tuple([Int(1), String(\"hello\"), Bool(true)])");

    // Map {"a": 1, "b": 2}
    let program = vec![
        Instruction::PushStr("a".to_string()),
        Instruction::PushInt(1),
        Instruction::PushStr("b".to_string()),
        Instruction::PushInt(2),
        Instruction::CreateMap(2),
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Map([(String(\"a\"), Int(1)), (String(\"b\"), Int(2))])");

    Ok(())
}

/// Test name creation examples
#[test]
fn test_name_creation_examples() -> Result<()> {
    // Create a runtime for executing async code
    let rt = Runtime::new()?;
    // Create a VM instance
    let vm = RholangVM::new()?;

    // Test top-level contract names
    // Rholang: new x, y in P
    let top_level_names_code = "new x, y in { x!(\"hello\") | y!(\"world\") }";
    let result = rt.block_on(async {
        vm.compile_and_execute(top_level_names_code).await
    })?;
    println!("Top-level names result: {}", result);

    // Test local names with concurrent access
    // Rholang: let x = <expression> in (P1(x) | P2(x))
    let concurrent_local_names_code = "new x in { let y = x in { y!(\"hello\") | y!(\"world\") } }";
    let result = rt.block_on(async {
        vm.compile_and_execute(concurrent_local_names_code).await
    })?;
    println!("Concurrent local names result: {}", result);

    // Test sequential local names
    // Rholang: let x = <expression> in P(x)
    let sequential_local_names_code = "new x in { let y = x in { y!(\"hello\") } }";
    let result = rt.block_on(async {
        vm.compile_and_execute(sequential_local_names_code).await
    })?;
    println!("Sequential local names result: {}", result);

    Ok(())
}

/// Test send operation examples
#[test]
fn test_send_operation_examples() -> Result<()> {
    // Create a runtime for executing async code
    let rt = Runtime::new()?;
    // Create a VM instance
    let vm = RholangVM::new()?;

    // Test top-level channel with lazy evaluation
    // Rholang: chan!(complex_process)
    let top_level_send_code = "new chan in { chan!(1 + 2 * 3) }";
    let result = rt.block_on(async {
        vm.compile_and_execute(top_level_send_code).await
    })?;
    println!("Top-level send result: {}", result);

    // Test local channel with explicit evaluation
    // Rholang: localChan!(*arithmetic_expr)
    // Note: Rholang 1.0 doesn't have the star syntax yet, so we'll use a simpler example
    let local_send_code = "new localChan in { localChan!(1 + 2 * 3) }";
    let result = rt.block_on(async {
        vm.compile_and_execute(local_send_code).await
    })?;
    println!("Local send result: {}", result);

    Ok(())
}

/// Test receive operation examples
#[test]
fn test_receive_operation_examples() -> Result<()> {
    // Create a runtime for executing async code
    let rt = Runtime::new()?;
    // Create a VM instance
    let vm = RholangVM::new()?;

    // Test contract reception (persistent)
    // Rholang: for(x <- publicChannel) P
    let contract_reception_code = "new publicChannel in {
        for(x <- publicChannel) { publicChannel!(x + 1) } |
        publicChannel!(5)
    }";
    let result = rt.block_on(async {
        vm.compile_and_execute(contract_reception_code).await
    })?;
    println!("Contract reception result: {}", result);

    // Test local scope reception
    // Rholang: { new local in { for(x <- local) P } }
    let local_reception_code = "new local in {
        for(x <- local) { local!(x + 1) } |
        local!(10)
    }";
    let result = rt.block_on(async {
        vm.compile_and_execute(local_reception_code).await
    })?;
    println!("Local reception result: {}", result);

    Ok(())
}

/// Test let binding examples
#[test]
fn test_let_binding_examples() -> Result<()> {
    // Create a runtime for executing async code
    let rt = Runtime::new()?;
    // Create a VM instance
    let vm = RholangVM::new()?;

    // Test sequential let
    // Rholang: let x = P; y = Q in R
    // Note: Rholang 1.0 doesn't have this exact syntax, so we'll use a simpler example
    let sequential_let_code = "new x in {
        let y = 5 in { x!(y) }
    }";
    let result = rt.block_on(async {
        vm.compile_and_execute(sequential_let_code).await
    })?;
    println!("Sequential let result: {}", result);

    Ok(())
}

/// Test parallel composition examples
#[test]
fn test_parallel_composition_examples() -> Result<()> {
    // Create a runtime for executing async code
    let rt = Runtime::new()?;
    // Create a VM instance
    let vm = RholangVM::new()?;

    // Test top-level parallel composition
    // Rholang: P | Q
    let top_level_par_code = "new x in { x!(\"hello\") } | new y in { y!(\"world\") }";
    let result = rt.block_on(async {
        vm.compile_and_execute(top_level_par_code).await
    })?;
    println!("Top-level parallel composition result: {}", result);

    // Test local parallel composition
    // Rholang: { new x in { P(x) | Q(x) } }
    let local_par_code = "new x in { x!(\"hello\") | x!(\"world\") }";
    let result = rt.block_on(async {
        vm.compile_and_execute(local_par_code).await
    })?;
    println!("Local parallel composition result: {}", result);

    Ok(())
}


#[test]
fn test_arithmetic_and_logical() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Modulo and negation
    let program = vec![Instruction::PushInt(10), Instruction::PushInt(3), Instruction::Mod];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Int(1)");

    let program = vec![Instruction::PushInt(5), Instruction::Neg];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Int(-5)");

    // Logical NOT on bool
    let program = vec![Instruction::PushBool(false), Instruction::Not];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Bool(true)");

    // String concatenation
    let program = vec![
        Instruction::PushStr("hello ".to_string()),
        Instruction::PushStr("world".to_string()),
        Instruction::Concat,
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "String(\"hello world\")");

    // List concatenation
    let program = vec![
        // Build [1,2]
        Instruction::PushInt(1),
        Instruction::PushInt(2),
        Instruction::CreateList(2),
        // Build [3,4]
        Instruction::PushInt(3),
        Instruction::PushInt(4),
        Instruction::CreateList(2),
        // Concat -> [1,2,3,4]
        Instruction::Concat,
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "List([Int(1), Int(2), Int(3), Int(4)])");

    // More comparisons
    let program = vec![Instruction::PushInt(5), Instruction::PushInt(5), Instruction::CmpNeq];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Bool(false)");

    let program = vec![Instruction::PushInt(5), Instruction::PushInt(4), Instruction::CmpGt];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Bool(true)");

    let program = vec![Instruction::PushInt(5), Instruction::PushInt(5), Instruction::CmpGte];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Bool(true)");

    let program = vec![Instruction::PushInt(5), Instruction::PushInt(6), Instruction::CmpLte];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Bool(true)");

    // Error paths: division by zero, modulo by zero
    test_utils::run_and_expect_err(&rt, &vm, vec![Instruction::PushInt(1), Instruction::PushInt(0), Instruction::Div], "Division by zero");
    test_utils::run_and_expect_err(&rt, &vm, vec![Instruction::PushInt(1), Instruction::PushInt(0), Instruction::Mod], "Modulo by zero");

    Ok(())
}

// All not implemented yet instructions
#[test]
fn test_unimplemented_instructions() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Unimplemented evaluation/control flow per design doc
    run_and_expect_err(&rt, &vm, vec![Instruction::Eval], "Eval not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::EvalBool], "EvalBool not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::EvalStar], "EvalStar not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::EvalWithLocals], "EvalWithLocals not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::EvalInBundle], "EvalInBundle not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::EvalToRSpace], "EvalToRSpace not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::Exec], "Exec not implemented yet");

    // Unimplemented pattern matching
    run_and_expect_err(&rt, &vm, vec![Instruction::Pattern("x".to_string())], "Pattern not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::MatchTest], "MatchTest not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::ExtractBindings], "ExtractBindings not implemented yet");

    // Process logic controls
    run_and_expect_err(&rt, &vm, vec![Instruction::SpawnAsync(RSpaceType::MemoryConcurrent)], "SpawnAsync not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::ProcNeg], "ProcNeg not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::Conj], "Conj not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::Disj], "Disj not implemented yet");

    // Reference and method invocation
    run_and_expect_err(&rt, &vm, vec![Instruction::Copy], "Copy not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::Move], "Move not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::Ref], "Ref not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::LoadMethod("m".to_string())], "LoadMethod not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::InvokeMethod], "InvokeMethod not implemented yet");

    // RSpace/Name placeholders that remain unimplemented
    run_and_expect_err(&rt, &vm, vec![Instruction::RSpaceMatch(RSpaceType::MemorySequential)], "RSpaceMatch not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::RSpaceSelectBegin(RSpaceType::MemoryConcurrent)], "RSpaceSelectBegin not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::RSpaceSelectAdd(RSpaceType::MemoryConcurrent)], "RSpaceSelectAdd not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::RSpaceSelectWait(RSpaceType::MemoryConcurrent)], "RSpaceSelectWait not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::PatternCompile(RSpaceType::MemorySequential)], "PatternCompile not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::PatternBind(RSpaceType::MemorySequential)], "PatternBind not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::ContinuationStore(RSpaceType::MemorySequential)], "ContinuationStore not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::ContinuationResume(RSpaceType::MemorySequential)], "ContinuationResume not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::RSpaceBundleBegin(RSpaceType::MemorySequential, rholang_vm::bytecode::BundleOp::Read)], "RSpaceBundleBegin not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::RSpaceBundleEnd(RSpaceType::MemorySequential)], "RSpaceBundleEnd not implemented yet");

    // BranchSuccess also unimplemented
    run_and_expect_err(&rt, &vm, vec![Instruction::BranchSuccess(Label("L".to_string()))], "BranchSuccess not implemented yet");

    Ok(())
}