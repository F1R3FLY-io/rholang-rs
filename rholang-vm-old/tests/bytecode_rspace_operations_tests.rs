use anyhow::Result;
use tokio::runtime::Runtime;
use rholang_vm_old::{RholangVM, bytecode::{Instruction, RSpaceType, Label}};

#[test]
fn test_name_creation_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Top-level names (use persistent concurrent storage)
    let program_top_level = vec![
        // Create x
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        // Create y
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(1),
        // x!("hello")
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
    assert_eq!(result, "Bool(true)");

    // Local concurrent name used twice
    let program_concurrent_local = vec![
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
    ];
    let result = rt.block_on(async { vm.execute(&program_concurrent_local).await })?;
    assert_eq!(result, "Bool(true)");

    // Sequential local name single use
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

#[test]
fn test_send_operation_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Top-level send
    let program_top_level = vec![
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        Instruction::LoadLocal(0),
        Instruction::PushInt(1),
        Instruction::PushInt(2),
        Instruction::PushInt(3),
        Instruction::Mul,
        Instruction::Add,
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program_top_level).await })?;
    assert_eq!(result, "Bool(true)");

    // Local send
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

#[test]
fn test_receive_operation_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Top-level receive
    let program_top_level_receive = vec![
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        Instruction::LoadLocal(0),
        Instruction::PushInt(5),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsume(RSpaceType::StoreConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program_top_level_receive).await })?;
    assert_eq!(result, "List([Int(5)])");

    // Local receive
    let program_local_receive = vec![
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        Instruction::LoadLocal(0),
        Instruction::PushInt(10),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsume(RSpaceType::MemoryConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program_local_receive).await })?;
    assert_eq!(result, "List([Int(10)])");

    Ok(())
}

#[test]
fn test_let_binding_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    let program = vec![
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        Instruction::AllocLocal,
        Instruction::PushInt(5),
        Instruction::StoreLocal(1),
        Instruction::LoadLocal(0),
        Instruction::LoadLocal(1),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
    ];
    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Bool(true)");

    Ok(())
}

#[test]
fn test_parallel_composition_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Top-level parallel composition
    let program_top_level_parallel = vec![
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        Instruction::LoadLocal(0),
        Instruction::PushStr("hello".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(1),
        Instruction::LoadLocal(1),
        Instruction::PushStr("world".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        Instruction::Jump(Label("after_spawn_top".to_string())),
        Instruction::Label(Label("spawn_top".to_string())),
        Instruction::SpawnAsync(RSpaceType::MemoryConcurrent),
        Instruction::Label(Label("after_spawn_top".to_string())),
    ];
    let result = rt.block_on(async { vm.execute(&program_top_level_parallel).await })?;
    assert_eq!(result, "Bool(true)");

    // Local parallel composition
    let program_local_parallel = vec![
        Instruction::NameCreate(RSpaceType::MemoryConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        Instruction::LoadLocal(0),
        Instruction::PushStr("hello".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        Instruction::LoadLocal(0),
        Instruction::PushStr("world".to_string()),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::MemoryConcurrent),
        Instruction::Jump(Label("after_spawn_local".to_string())),
        Instruction::Label(Label("spawn_local".to_string())),
        Instruction::SpawnAsync(RSpaceType::MemoryConcurrent),
        Instruction::Label(Label("after_spawn_local".to_string())),
    ];
    let result = rt.block_on(async { vm.execute(&program_local_parallel).await })?;
    assert_eq!(result, "Bool(true)");

    Ok(())
}
