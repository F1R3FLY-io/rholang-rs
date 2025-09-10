// Example: Contract implementation using atomic bytecode operations (per BYTECODE_DESIGN.md)
// This example demonstrates how a contract install sequence maps to bytecode.
// Notes:
// - Some instructions from the design doc (PATTERN, EXTRACT_BINDINGS, ASK persistent)
//   are not implemented in the current VM. We include them under a label that is
//   skipped via an unconditional jump to keep the example faithful to the documentation
//   without triggering unimplemented paths at runtime.
// - We then demonstrate an "atomic" sequence of implemented instructions that approximate
//   a contract behavior using persistent RSpace operations.

use anyhow::Result;
use rholang_vm::{
    bytecode::{Instruction, Label, RSpaceType},
    RholangVM,
};
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    // Desugaring reminder from docs:
    // contract MyContract(x) = { P }  =>  for (x <= MyContract) { P }
    // Documented bytecode (not fully implemented yet) is present but skipped.

    let program = vec![
        // Create top-level contract channel in persistent concurrent RSpace
        Instruction::NameCreate(RSpaceType::StoreConcurrent), // -> Name
        Instruction::AllocLocal,
        Instruction::StoreLocal(0), // local 0 = MyContract

        // Documented install sequence (skipped)
        Instruction::Jump(Label("after_install".to_string())),
        Instruction::Label(Label("install".to_string())),
            // LOAD_VAR MyContract ~ LoadLocal(0) here
            Instruction::LoadLocal(0),
            Instruction::AllocLocal,
            Instruction::Pattern("x".to_string()),              // unimplemented
            Instruction::PushProc("P".to_string()),
            Instruction::ContinuationStore(RSpaceType::StoreConcurrent),
            Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent), // as placeholder for persistent ask
            Instruction::ExtractBindings,                       // unimplemented
            Instruction::ContinuationResume(RSpaceType::StoreConcurrent),
        Instruction::Label(Label("after_install".to_string())),

        // Implemented subset approximating contract semantics (atomic steps):
        // 1) Store a continuation representing P and drop its id
        Instruction::PushProc("P".to_string()),
        Instruction::ContinuationStore(RSpaceType::StoreConcurrent), // -> Int(id)
        Instruction::Pop,

        // 2) Send data to the contract channel: MyContract!([42])
        Instruction::LoadLocal(0),         // push channel name
        Instruction::PushInt(42),          // data
        Instruction::CreateList(1),        // wrap as singleton list, matches RSpace examples
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),

        // 3) Persistent receive semantics: peek without consuming
        Instruction::LoadLocal(0),
        Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent), // -> List([Int(42)])
    ];

    let result = rt.block_on(async { vm.execute(&program).await })?;

    println!("Result: {result}");
    // Expect: List([Int(42)])
    Ok(())
}
