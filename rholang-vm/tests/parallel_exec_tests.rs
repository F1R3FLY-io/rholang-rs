use rholang_bytecode::core::instructions::Instruction;
use rholang_bytecode::core::Opcode;
use rholang_process::{execute_ready_processes, Process, ProcessEvent, ProcessState};
use std::sync::{Arc, Mutex};

#[test]
fn test_execute_ready_processes_emits_events() {
    let ready_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "ready_proc");
    let wait_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "wait_proc")
        .with_state(ProcessState::Wait);

    let events: Arc<Mutex<Vec<ProcessEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let handler_events = events.clone();
    let handler = move |event: ProcessEvent| {
        if let Ok(mut guard) = handler_events.lock() {
            guard.push(event);
        }
    };
    let handler = Arc::new(handler);

    let (updated, results) = execute_ready_processes(vec![ready_proc, wait_proc], Some(handler));

    assert_eq!(updated.len(), 2);
    assert_eq!(results.len(), 2);

    let ready = updated
        .iter()
        .find(|p| p.source_ref == "ready_proc")
        .unwrap();
    assert!(matches!(ready.state, ProcessState::Value(_)));

    let waiting = updated
        .iter()
        .find(|p| p.source_ref == "wait_proc")
        .unwrap();
    assert!(matches!(waiting.state, ProcessState::Wait));

    let captured = events.lock().unwrap();
    assert_eq!(captured.len(), 1);
    assert!(matches!(captured[0], ProcessEvent::Value(_)));
}
