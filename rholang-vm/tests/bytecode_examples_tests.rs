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
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Top-level names (use persistent concurrent storage)
    // Equivalent Rholang: new x, y in { x!("hello") | y!("world") }
    // Bytecode steps:
    // - Create two fresh names in StoreConcurrent, store in locals 0 and 1
    // - Produce ["hello"] to x and ["world"] to y
    let program_top_level = vec![
        // Create x
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        // Create y
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(1),
        // x!("hello") -> push channel then data list
        Instruction::LoadLocal(0),
        Instruction::PushStr("hello".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),
        // y!("world")
        Instruction::LoadLocal(1),
        Instruction::PushStr("world".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program_top_level).await })?;
    // RSpaceProduce leaves Bool(true) on stack; after two produces the last is true
    assert_eq!(result, "Bool(true)");

    // Local name with concurrent access (alias and use twice)
    // Equivalent Rholang: new x in { let y = x in { y!("hello") | y!("world") } }
    let program_concurrent_local = vec![
        // Create x in MemoryConcurrent and store as local 0; y is alias via LoadLocal(0)
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        // y!("hello")
        Instruction::LoadLocal(0),
        Instruction::PushStr("hello".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        // y!("world")
        Instruction::LoadLocal(0),
        Instruction::PushStr("world".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program_concurrent_local).await })?;
    assert_eq!(result, "Bool(true)");

    // Sequential local name (single use)
    // Equivalent Rholang: new x in { let y = x in { y!("hello") } }
    let program_sequential_local = vec![
        Instruction::NameCreate(RSpaceType::MemorySequential),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        Instruction::LoadLocal(0),
        Instruction::PushStr("hello".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemorySequential),
    ];
    let result = rt.block_on(async { vm.execute(&program_sequential_local).await })?;
    assert_eq!(result, "Bool(true)");

    Ok(())
}

/// Test send operation examples
#[test]
fn test_send_operation_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Top-level send: new chan in { chan!(1 + 2 * 3) }
    let program_top_level = vec![
        // Create top-level channel in persistent concurrent store
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        // Prepare send: chan!(1 + 2 * 3)
        Instruction::LoadLocal(0),
        Instruction::PushInt(1),
        Instruction::PushInt(2),
        Instruction::PushInt(3),
        Instruction::Mul, // 2 * 3 = 6
        Instruction::Add, // 1 + 6 = 7
        Instruction::CreateList(1), // data list [7]
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program_top_level).await })?;
    assert_eq!(result, "Bool(true)");

    // Local send: new localChan in { localChan!(1 + 2 * 3) }
    let program_local = vec![
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        Instruction::LoadLocal(0),
        Instruction::PushInt(1),
        Instruction::PushInt(2),
        Instruction::PushInt(3),
        Instruction::Mul,
        Instruction::Add,
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program_local).await })?;
    assert_eq!(result, "Bool(true)");

    Ok(())
}

/// Test receive operation examples
#[test]
fn test_receive_operation_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Top-level receive: new publicChannel in { publicChannel!(5) | for(x <- publicChannel) { x }
    // Bytecode: create channel in StoreConcurrent, produce [5], then consume value from channel.
    let program_top_level_receive = vec![
        // Create channel
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        // Produce [5] on channel
        Instruction::LoadLocal(0),
        Instruction::PushInt(5),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),
        // Consume from channel (should get List([Int(5)]))
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsume(RSpaceType::StoreConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program_top_level_receive).await })?;
    assert_eq!(result, "List([Int(5)])");

    // Local receive: new local in { local!(10) | for(x <- local) { x }
    // Use MemoryConcurrent for local channels
    let program_local_receive = vec![
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        // Produce [10]
        Instruction::LoadLocal(0),
        Instruction::PushInt(10),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        // Consume => List([Int(10)])
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsume(RSpaceType::MemoryConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program_local_receive).await })?;
    assert_eq!(result, "List([Int(10)])");

    Ok(())
}

/// Test let binding examples
#[test]
fn test_let_binding_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Model: new x in { let y = 5 in { x!(y) } }
    // Using bytecode with locals:
    // - local 0: channel x
    // - local 1: bound value y = 5
    let program = vec![
        // new x in ... (local channel)
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        // let y = 5
        Instruction::AllocLocal,
        Instruction::PushInt(5),
        Instruction::StoreLocal(1),
        // x!(y)
        Instruction::LoadLocal(0), // x
        Instruction::LoadLocal(1), // y
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Bool(true)");

    Ok(())
}

/// Test parallel composition examples
#[test]
// currently ignored because of not checking real paralleling code execution
// TODO: fix this test to really check parallel execution and enable it
#[ignore]
fn test_parallel_composition_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Top-level parallel composition
    // Rholang: new x in { x!("hello") } | new y in { y!("world") }
    // Bytecode model: create two independent local concurrent channels and produce on each
    let program_top_level_parallel = vec![
        // new x in { x!("hello") }
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        Instruction::LoadLocal(0),
        Instruction::PushStr("hello".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        // new y in { y!("world") }
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(1),
        Instruction::LoadLocal(1),
        Instruction::PushStr("world".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        // Ensure SPAWN_ASYNC appears in bytecode but is not executed
        Instruction::Jump(Label("after_spawn_top".to_string())),
        Instruction::Label(Label("spawn_top".to_string())),
        Instruction::SpawnAsync(RSpaceType::MemoryConcurrent),
        Instruction::Label(Label("after_spawn_top".to_string())),
    ];
    let result = rt.block_on(async { vm.execute(&program_top_level_parallel).await })?;
    // Each RSpaceProduce leaves Bool(true); the final result should be Bool(true)
    assert_eq!(result, "Bool(true)");

    // Local parallel composition
    // Rholang: new x in { x!("hello") | x!("world") }
    // Bytecode model: one local concurrent channel used by two sends
    let program_local_parallel = vec![
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        // x!("hello")
        Instruction::LoadLocal(0),
        Instruction::PushStr("hello".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        // x!("world")
        Instruction::LoadLocal(0),
        Instruction::PushStr("world".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        // Include SPAWN_ASYNC presence, but skip it via jump
        Instruction::Jump(Label("after_spawn_local".to_string())),
        Instruction::Label(Label("spawn_local".to_string())),
        Instruction::SpawnAsync(RSpaceType::MemoryConcurrent),
        Instruction::Label(Label("after_spawn_local".to_string())),
    ];
    let result = rt.block_on(async { vm.execute(&program_local_parallel).await })?;
    assert_eq!(result, "Bool(true)");

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
    run_and_expect_err(&rt, &vm, vec![Instruction::PushInt(1), Instruction::PushInt(0), Instruction::Div], "Division by zero");
    run_and_expect_err(&rt, &vm, vec![Instruction::PushInt(1), Instruction::PushInt(0), Instruction::Mod], "Modulo by zero");

    Ok(())
}

// All not implemented yet instructions (excluding newly implemented RSpace/Name ops)
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


    // BranchSuccess also unimplemented
    run_and_expect_err(&rt, &vm, vec![Instruction::BranchSuccess(Label("L".to_string()))], "BranchSuccess not implemented yet");

    Ok(())
}