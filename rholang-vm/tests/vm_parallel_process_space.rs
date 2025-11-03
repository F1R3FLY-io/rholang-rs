use std::sync::Arc;

#[cfg(feature = "parallel-exec")]
use rholang_vm::api::VmBuilder;
use rholang_vm::api::Process;
use rholang_vm::api::Instruction;

#[cfg(feature = "parallel-exec")]
#[test]
fn spawn_process_stores_in_process_space_and_retrievable() {
    // Build a minimal process (no-op) to spawn
    let code: Vec<Instruction> = vec![];
    let proc = Arc::new(Process::new(code, "test:spawn"));

    let mut vm = VmBuilder::new().threads(2).default_budget(10).build();
    let pid = vm.spawn_process(proc.clone());

    // Ensure the process can be retrieved by pid via the exposed API
    let found = vm.get_process(pid).expect("process should be stored");
    assert_eq!(found.source_ref, "test:spawn");

    // Verify canonical path formatting is stable
    let path = vm.process_path(pid);
    assert!(path.starts_with("/process/"));
}

// When the parallel-exec feature is disabled, provide a smoke test that compiles
#[cfg(not(feature = "parallel-exec"))]
#[test]
fn parallel_api_unavailable_compiles() {
    // This test intentionally does nothing; it ensures the integration test file compiles
    // without requiring the parallel-exec feature.
    assert!(true);
}
