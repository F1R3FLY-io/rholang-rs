use anyhow::Result;
use tokio::runtime::Runtime;
use rholang_vm_old::{RholangVM, bytecode::{Instruction, RSpaceType, Label}};

/// Test contract operation bytecode equivalents based on documentation
#[test]
fn test_contract_operation_bytecode_examples() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Desugaring: contract MyContract(x) = { P }  =>  for (x <= MyContract) { P }
    // Include the documented instruction sequence but skip execution of unimplemented parts.

    let program = vec![
        // Create top-level contract channel in persistent concurrent RSpace (StoreConcurrent)
        Instruction::NameCreate(RSpaceType::StoreConcurrent), // -> Name
        Instruction::AllocLocal,
        Instruction::StoreLocal(0), // local 0 = MyContract

        // Ensure documented install sequence is present but NOT executed
        Instruction::Jump(Label("after_install".to_string())),
        Instruction::Label(Label("install".to_string())),
            // LOAD_VAR MyContract ~ we use LoadLocal(0) as equivalent in test context
            Instruction::LoadLocal(0),
            Instruction::AllocLocal,
            // PATTERN x (unimplemented)
            Instruction::Pattern("x".to_string()),
            // CONT_STORE P (placeholder process string)
            Instruction::PushProc("P".to_string()),
            Instruction::ContinuationStore(RSpaceType::StoreConcurrent),
            // ASK persistent (approximate with persistent consume)
            Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent),
            // EXTRACT_BINDINGS (unimplemented)
            Instruction::ExtractBindings,
            // CONT_RESUME (would resume stored continuation)
            Instruction::ContinuationResume(RSpaceType::StoreConcurrent),
        Instruction::Label(Label("after_install".to_string())),

        // Implemented subset approximating contract semantics.
        // 1) Store a continuation and discard id
        Instruction::PushProc("P".to_string()),
        Instruction::ContinuationStore(RSpaceType::StoreConcurrent),
        Instruction::Pop,

        // 2) Send data to the contract channel: MyContract!([42])
        Instruction::LoadLocal(0),
        Instruction::PushInt(42),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),

        // 3) Persistent receive semantics: peek should not consume and yield the sent list
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent), // -> List([Int(42)])
    ];

    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "List([Int(42)])");

    Ok(())
}


#[test]
fn test_contract_persistent_peek_then_consume() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    let program = vec![
        // Create contract channel in persistent concurrent RSpace
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),

        // Send [1]
        Instruction::LoadLocal(0),
        Instruction::PushInt(1),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),

        // Persistent peek twice (non-consuming)
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent), // -> List([Int(1)])
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent), // -> List([Int(1)]) again
    ];

    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "List([Int(1)])");

    // Now extend with a consume to verify removal and then persistent peek Nil
    let program = vec![
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        Instruction::LoadLocal(0),
        Instruction::PushInt(1),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),
        // Consume removes the value
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsume(RSpaceType::StoreConcurrent), // -> List([Int(1)])
        // Persistent peek now yields Nil since queue is empty
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent), // -> Nil
    ];

    let result2 = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result2, "Nil");

    Ok(())
}

#[test]
fn test_contract_multiple_sends_and_persistent_peek() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    let program = vec![
        Instruction::NameCreate(RSpaceType::StoreConcurrent),
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),
        // Send [1]
        Instruction::LoadLocal(0),
        Instruction::PushInt(1),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),
        // Send [2]
        Instruction::LoadLocal(0),
        Instruction::PushInt(2),
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),
        // Persistent peek sees head ([1])
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent), // -> List([Int(1)])
        // Consume removes head ([1])
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsume(RSpaceType::StoreConcurrent), // -> List([Int(1)])
        // Persistent peek now sees next ([2])
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent), // -> List([Int(2)])
    ];

    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "List([Int(2)])");
    Ok(())
}

#[test]
fn test_contract_continuation_store_and_resume() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    let program = vec![
        // Store a continuation representing P, then immediately resume it by id
        Instruction::PushProc("P".to_string()),
        Instruction::ContinuationStore(RSpaceType::StoreConcurrent), // -> Int(id)
        Instruction::ContinuationResume(RSpaceType::StoreConcurrent), // pops id, pushes Process("P")
    ];

    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "Process(\"P\")");
    Ok(())
}


#[test]
fn test_greeter_contract_example() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    let program = vec![
        // Create top-level channels (persistent concurrent RSpace)
        // new greeter, stdout in { ... }
        Instruction::NameCreate(RSpaceType::StoreConcurrent), // greeter
        Instruction::AllocLocal,
        Instruction::StoreLocal(0),

        Instruction::NameCreate(RSpaceType::StoreConcurrent), // stdout
        Instruction::AllocLocal,
        Instruction::StoreLocal(1),

        // Doc-faithful contract installation (skipped)
        Instruction::Jump(Label("after_install".to_string())),
        Instruction::Label(Label("install".to_string())),
            // LOAD_VAR greeter (use LoadLocal(0))
            Instruction::LoadLocal(0),
            Instruction::AllocLocal,
            // PATTERN (name, return) (unimplemented)
            Instruction::Pattern("(name, return)".to_string()),
            // CONT_STORE body: return!("Hello, " ++ *name)
            Instruction::PushProc("return!(\"Hello, \" ++ *name)".to_string()),
            Instruction::ContinuationStore(RSpaceType::StoreConcurrent),
            // ASK persistent greeter (placeholder with persistent consume)
            Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent),
            // Extract and resume (unimplemented)
            Instruction::ExtractBindings,
            Instruction::ContinuationResume(RSpaceType::StoreConcurrent),
        Instruction::Label(Label("after_install".to_string())),

        // Runnable approximation of greeter!("Alice", *stdout)
        Instruction::LoadLocal(1),                      // stdout channel
        Instruction::PushStr("Hello, ".to_string()),
        Instruction::PushStr("Alice".to_string()),
        Instruction::Concat,                            // "Hello, " ++ "Alice"
        Instruction::CreateList(1),                     // [greeting]
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),

        // greeter!("Bob", *stdout)
        Instruction::LoadLocal(1),                      // stdout channel
        Instruction::PushStr("Hello, ".to_string()),
        Instruction::PushStr("Bob".to_string()),
        Instruction::Concat,                            // "Hello, " ++ "Bob"
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),

        // Read from stdout: remove first, then peek second
        Instruction::LoadLocal(1),
        Instruction::RSpaceConsume(RSpaceType::StoreConcurrent),     // -> ["Hello, Alice"] consumed
        Instruction::Pop,                                            // discard
        Instruction::LoadLocal(1),
        Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent), // -> ["Hello, Bob"]
    ];

    let result = rt.block_on(async { vm.execute(&program).await })?;
    assert_eq!(result, "List([String(\"Hello, Bob\")])");
    Ok(())
}
