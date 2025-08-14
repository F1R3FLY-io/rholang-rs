use rholang_vm::vm::ExecutionContext;
use rholang_vm::state::{snapshot_from_context};
use rholang_vm::vm::VM;

#[test]
fn test_execution_context_pause_resume_roundtrip() {
    // Prepare a context with some state
    let mut ctx = ExecutionContext::new();
    ctx.stack.push(rholang_vm::bytecode::Value::Int(7));
    ctx.locals.push(rholang_vm::bytecode::Value::Bool(true));
    ctx.ip = 3;
    ctx.labels.insert(rholang_vm::bytecode::Label("L1".into()), 1);
    ctx.memory.constant_pool.push(rholang_vm::bytecode::Value::String("const".into()));

    // Take snapshot via method
    let snap = ctx.pause_and_snapshot();

    // Mutate the context to ensure we actually restore from snapshot
    ctx.stack.clear();
    ctx.locals.clear();
    ctx.ip = 0;
    ctx.labels.clear();
    ctx.memory.constant_pool.clear();

    // Resume from snapshot
    ctx.resume_from_snapshot(&snap).expect("resume ok");

    // Snapshot from restored context must equal original snapshot
    let snap2 = snapshot_from_context(&ctx);
    let json1 = serde_json::to_string(&snap).unwrap();
    let json2 = serde_json::to_string(&snap2).unwrap();
    assert_eq!(json1, json2);
}

#[test]
fn test_vm_pause_resume_convenience_methods() {
    let vm = VM::new().expect("vm new");
    let mut ctx = ExecutionContext::new();
    ctx.stack.push(rholang_vm::bytecode::Value::Int(99));
    let snap = vm.pause_and_snapshot(&ctx);

    let ctx2 = vm.resume_from_snapshot(&snap).expect("resume ctx");

    // Compare snapshots of both contexts
    let s1 = snapshot_from_context(&ctx);
    let s2 = snapshot_from_context(&ctx2);
    let j1 = serde_json::to_string(&s1).unwrap();
    let j2 = serde_json::to_string(&s2).unwrap();
    assert_eq!(j1, j2);
}
