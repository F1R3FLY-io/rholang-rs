use anyhow::Result;
use tokio::runtime::Runtime;
use rholang_vm::{RholangVM, bytecode::Instruction};

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
