// Example: Greeter contract bytecode approximation
// Rholang:
// new greeter, stdout(`rho:io:stdout`) in {
//   // Persistent contract installation
//   new greeter, stdout in {
//     greeter!("Alice", *stdout) |
//     greeter!("Bob", *stdout)
//   } |
//   // Runnable approximation:
//   contract greeter(name, return) = {
//     return!("Hello, " ++ *name)
//   } |
//   greeter!("Alice", *stdout) |
//   greeter!("Bob", *stdout)
// }
//
// Notes:
// - The VM currently lacks full support for PATTERN/EXTRACT_BINDINGS/ASK persistent and star evaluation.
// - We include a doc-faithful install sequence under a label and skip it via Jump so the program runs.
// - The runnable part approximates behavior by directly producing greeting messages to the stdout channel.
// - We then demonstrate reading: consume removes the first greeting, and a persistent consume (peek)
//   shows the second greeting as the final result.

use anyhow::Result;
use rholang_vm::{
    bytecode::{Instruction, Label, RSpaceType},
    RholangVM,
};
use tokio::runtime::Runtime;

fn main() -> Result<()> {
    let rt = Runtime::new()?;
    let vm = RholangVM::new()?;

    let program = vec![
        // Create top-level channels (persistent concurrent RSpace)
        // new greeter, stdout in { ... }
        Instruction::NameCreate(RSpaceType::StoreConcurrent), // -> Name (greeter)
        Instruction::AllocLocal,
        Instruction::StoreLocal(0), // local[0] = greeter

        Instruction::NameCreate(RSpaceType::StoreConcurrent), // -> Name (stdout)
        Instruction::AllocLocal,
        Instruction::StoreLocal(1), // local[1] = stdout

        // Doc-faithful contract installation (skipped)
        Instruction::Jump(Label("after_install".to_string())),
        Instruction::Label(Label("install".to_string())),
            // LOAD_VAR greeter  (use LoadLocal(0) here)
            Instruction::LoadLocal(0),
            Instruction::AllocLocal,                  // slots for pattern vars
            // PATTERN (name, return)  (unimplemented)
            Instruction::Pattern("(name, return)".to_string()),
            // CONT_STORE body: return!("Hello, " ++ *name)
            Instruction::PushProc("return!(\"Hello, \" ++ *name)".to_string()),
            Instruction::ContinuationStore(RSpaceType::StoreConcurrent),
            // ASK persistent greeter  (placeholder with persistent consume)
            Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent),
            // Extract and resume (unimplemented)
            Instruction::ExtractBindings,
            Instruction::ContinuationResume(RSpaceType::StoreConcurrent),
        Instruction::Label(Label("after_install".to_string())),

        // Runnable approximation:
        // Simulate greeter!("Alice", *stdout)
        // Body would do: return!("Hello, " ++ *name)
        // We build the greeting and send to stdout channel.
        Instruction::LoadLocal(1),                      // stdout channel
        Instruction::PushStr("Hello, ".to_string()),
        Instruction::PushStr("Alice".to_string()),
        Instruction::Concat,                            // "Hello, " ++ "Alice"
        Instruction::CreateList(1),                     // wrap as [greeting]
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),

        // Simulate greeter!("Bob", *stdout)
        Instruction::LoadLocal(1),                      // stdout channel
        Instruction::PushStr("Hello, ".to_string()),
        Instruction::PushStr("Bob".to_string()),
        Instruction::Concat,                            // "Hello, " ++ "Bob"
        Instruction::CreateList(1),
        Instruction::RSpaceProduce(RSpaceType::StoreConcurrent),

        // Demonstrate reading from stdout: remove first, then peek second
        Instruction::LoadLocal(1),
        Instruction::RSpaceConsume(RSpaceType::StoreConcurrent),     // -> ["Hello, Alice"] (consumed)
        Instruction::Pop,                                            // discard consumed result
        Instruction::LoadLocal(1),
        Instruction::RSpaceConsumePersistent(RSpaceType::StoreConcurrent), // -> ["Hello, Bob"] (peek)
    ];

    let result = rt.block_on(async { vm.execute(&program).await })?;
    println!("Result: {result}");
    // Expect final: List([String("Hello, Bob")])
    Ok(())
}
