use anyhow::Result;
use tokio::runtime::Runtime;
use rholang_vm::{RholangVM, bytecode::{Instruction, Label}};
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
        Instruction::PushInt(1),
        Instruction::PushInt(2),
        Instruction::CreateList(2),
        Instruction::PushInt(3),
        Instruction::PushInt(4),
        Instruction::CreateList(2),
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
    run_and_expect_err(&rt, &vm, vec![Instruction::PushInt(1), Instruction::PushInt(0), Instruction::Div], "Division by zero");
    run_and_expect_err(&rt, &vm, vec![Instruction::PushInt(1), Instruction::PushInt(0), Instruction::Mod], "Modulo by zero");

    Ok(())
}

// Not implemented yet instructions (excluding implemented RSpace/Name ops)
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
    run_and_expect_err(&rt, &vm, vec![Instruction::SpawnAsync(rholang_vm::bytecode::RSpaceType::MemoryConcurrent)], "SpawnAsync not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::ProcNeg], "ProcNeg not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::Conj], "Conj not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::Disj], "Disj not implemented yet");

    // Reference and method invocation
    run_and_expect_err(&rt, &vm, vec![Instruction::Copy], "Copy not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::Move], "Move not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::Ref], "Ref not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::LoadMethod("m".to_string())], "LoadMethod not implemented yet");
    run_and_expect_err(&rt, &vm, vec![Instruction::InvokeMethod], "InvokeMethod not implemented yet");

    // BranchSuccess also unimplemented
    run_and_expect_err(&rt, &vm, vec![Instruction::BranchSuccess(Label("L".to_string()))], "BranchSuccess not implemented yet");

    Ok(())
}
