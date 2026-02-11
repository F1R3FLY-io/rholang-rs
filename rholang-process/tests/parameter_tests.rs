// Tests for Process parameters
// Parameters are named bindings that relate processes to entries in RSpace.
// A parameter is solved when its entry is in a resolved state:
// - Channel: non-empty queue
// - Process: in Value state
// - Value: always solved

use rholang_bytecode::core::instructions::Instruction;
use rholang_bytecode::core::Opcode;
use rholang_process::{
    Parameter, Process, ProcessHolder, ProcessState, RSpace, SharedRSpace, Value, VM,
};
use rholang_rspace::PathMapRSpace;
use std::sync::{Arc, Mutex};

/// Helper to create a simple HALT process
fn halt_process(name: &str) -> Process {
    Process::new(vec![Instruction::nullary(Opcode::HALT)], name)
}

/// Helper to box a Process into a ProcessHolder
fn box_process(p: Process) -> Box<dyn ProcessHolder> {
    Box::new(p)
}

/// Helper to create a shared RSpace
fn shared_rspace() -> SharedRSpace {
    Arc::new(Mutex::new(Box::new(PathMapRSpace::new()) as Box<dyn RSpace>))
}

/// Helper to create a VM with a shared RSpace
fn vm_with_shared_rspace(rspace: SharedRSpace) -> VM {
    VM::with_shared_rspace(rspace)
}

// ============================================================================
// Test: Parameter creation and basic properties
// ============================================================================

#[test]
fn test_parameter_new() {
    let param = Parameter::new("input");
    assert_eq!(param.name(), "input");
}

#[test]
fn test_parameter_new_with_string() {
    let name = String::from("channel");
    let param = Parameter::new(name);
    assert_eq!(param.name(), "channel");
}

// ============================================================================
// Test: Process with zero parameters
// ============================================================================

#[test]
fn test_process_with_zero_parameters_is_ready() {
    let process = halt_process("no_params");
    assert!(process.is_ready());
    assert!(process.parameters().is_empty());
}

#[test]
fn test_process_with_zero_parameters_executes() {
    let mut process = halt_process("no_params");
    let result = process.execute();
    assert!(result.is_ok());
    assert!(matches!(process.state, ProcessState::Value(_)));
}

// ============================================================================
// Test: Parameter is_solved with empty channel
// ============================================================================

#[test]
fn test_parameter_unsolved_when_entry_not_exists() {
    let rspace = shared_rspace();
    let param = Parameter::new("empty");

    // Entry doesn't exist, so parameter is unsolved
    let rspace_guard = rspace.lock().unwrap();
    assert!(!param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_unsolved_when_channel_empty() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        // Create channel and immediately consume, leaving it empty
        rspace_guard.tell("inbox", Value::Int(42)).unwrap();
        rspace_guard.ask("inbox").unwrap();
    }
    let param = Parameter::new("inbox");

    let rspace_guard = rspace.lock().unwrap();
    assert!(!param.is_solved(rspace_guard.as_ref()));
}

// ============================================================================
// Test: Parameter is_solved with channel values (various types)
// ============================================================================

#[test]
fn test_parameter_solved_with_int_in_channel() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("input", Value::Int(42)).unwrap();
    }

    let param = Parameter::new("input");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_solved_with_bool_in_channel() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("flag", Value::Bool(true)).unwrap();
    }

    let param = Parameter::new("flag");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_solved_with_string_in_channel() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .tell("msg", Value::Str("hello".to_string()))
            .unwrap();
    }

    let param = Parameter::new("msg");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_solved_with_nil_in_channel() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("nil_chan", Value::Nil).unwrap();
    }

    let param = Parameter::new("nil_chan");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_solved_with_list_in_channel() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .tell("list", Value::List(vec![Value::Int(1), Value::Int(2)]))
            .unwrap();
    }

    let param = Parameter::new("list");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_solved_with_tuple_in_channel() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .tell("tuple", Value::Tuple(vec![Value::Int(1)]))
            .unwrap();
    }

    let param = Parameter::new("tuple");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_solved_with_map_in_channel() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .tell("map", Value::Map(vec![(Value::Int(1), Value::Int(2))]))
            .unwrap();
    }

    let param = Parameter::new("map");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_solved_with_name_in_channel() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .tell("name_chan", Value::Name("other".to_string()))
            .unwrap();
    }

    let param = Parameter::new("name_chan");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

// ============================================================================
// Test: Parameter is_solved with Process entries
// ============================================================================

#[test]
fn test_parameter_solved_when_process_in_value_state() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .register_process("worker", ProcessState::Ready)
            .unwrap();
        rspace_guard
            .update_process("worker", ProcessState::Value(Value::Int(100)))
            .unwrap();
    }

    let param = Parameter::new("worker");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_unsolved_when_process_in_ready_state() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .register_process("worker", ProcessState::Ready)
            .unwrap();
    }

    let param = Parameter::new("worker");
    let rspace_guard = rspace.lock().unwrap();
    assert!(!param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_unsolved_when_process_in_wait_state() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .register_process("worker", ProcessState::Wait)
            .unwrap();
    }

    let param = Parameter::new("worker");
    let rspace_guard = rspace.lock().unwrap();
    assert!(!param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_unsolved_when_process_in_error_state() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .register_process("worker", ProcessState::Ready)
            .unwrap();
        rspace_guard
            .update_process("worker", ProcessState::Error("failed".to_string()))
            .unwrap();
    }

    let param = Parameter::new("worker");
    let rspace_guard = rspace.lock().unwrap();
    assert!(!param.is_solved(rspace_guard.as_ref()));
}

// ============================================================================
// Test: Parameter is_solved with Value entries
// ============================================================================

#[test]
fn test_parameter_solved_when_value_entry_exists() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .set_value("config", Value::Str("production".to_string()))
            .unwrap();
    }

    let param = Parameter::new("config");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

// ============================================================================
// Test: Parameter is_solved with Par values in channel
// ============================================================================

#[test]
fn test_parameter_solved_with_par_all_in_value_state() {
    let rspace = shared_rspace();

    // Create processes that are already in Value state (terminal)
    let proc1 = halt_process("p1").with_state(ProcessState::Value(Value::Int(1)));
    let proc2 = halt_process("p2").with_state(ProcessState::Value(Value::Int(2)));

    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .tell(
                "procs",
                Value::Par(vec![box_process(proc1), box_process(proc2)]),
            )
            .unwrap();
    }

    let param = Parameter::new("procs");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_unsolved_with_par_some_ready() {
    let rspace = shared_rspace();

    // One process in Value state, one still Ready
    let proc1 = halt_process("p1").with_state(ProcessState::Value(Value::Int(1)));
    let proc2 = halt_process("p2"); // Ready state

    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .tell(
                "procs",
                Value::Par(vec![box_process(proc1), box_process(proc2)]),
            )
            .unwrap();
    }

    let param = Parameter::new("procs");
    let rspace_guard = rspace.lock().unwrap();
    assert!(!param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_unsolved_with_par_some_wait() {
    let rspace = shared_rspace();

    // One process in Value state, one in Wait
    let proc1 = halt_process("p1").with_state(ProcessState::Value(Value::Int(1)));
    let proc2 = halt_process("p2").with_state(ProcessState::Wait);

    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .tell(
                "procs",
                Value::Par(vec![box_process(proc1), box_process(proc2)]),
            )
            .unwrap();
    }

    let param = Parameter::new("procs");
    let rspace_guard = rspace.lock().unwrap();
    assert!(!param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_unsolved_with_par_some_error() {
    let rspace = shared_rspace();

    // One process in Value state, one in Error
    let proc1 = halt_process("p1").with_state(ProcessState::Value(Value::Int(1)));
    let proc2 = halt_process("p2").with_state(ProcessState::Error("failed".to_string()));

    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .tell(
                "procs",
                Value::Par(vec![box_process(proc1), box_process(proc2)]),
            )
            .unwrap();
    }

    let param = Parameter::new("procs");
    let rspace_guard = rspace.lock().unwrap();
    assert!(!param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_unsolved_with_par_all_ready() {
    let rspace = shared_rspace();

    // All processes are Ready (not yet executed)
    let proc1 = halt_process("p1");
    let proc2 = halt_process("p2");

    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .tell(
                "procs",
                Value::Par(vec![box_process(proc1), box_process(proc2)]),
            )
            .unwrap();
    }

    let param = Parameter::new("procs");
    let rspace_guard = rspace.lock().unwrap();
    assert!(!param.is_solved(rspace_guard.as_ref()));
}

#[test]
fn test_parameter_solved_with_empty_par() {
    let rspace = shared_rspace();

    // Empty Par is considered solved (no pending processes)
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("procs", Value::Par(vec![])).unwrap();
    }

    let param = Parameter::new("procs");
    let rspace_guard = rspace.lock().unwrap();
    assert!(param.is_solved(rspace_guard.as_ref()));
}

// ============================================================================
// Test: Process with one parameter
// ============================================================================

#[test]
fn test_process_with_one_unsolved_parameter_is_not_ready() {
    let rspace = shared_rspace();
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("input");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "param_proc", vm)
        .with_parameters(vec![param]);

    assert!(!process.is_ready());
}

#[test]
fn test_process_with_one_solved_channel_parameter_is_ready() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("input", Value::Int(42)).unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("input");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "param_proc", vm)
        .with_parameters(vec![param]);

    assert!(process.is_ready());
}

#[test]
fn test_process_with_one_solved_value_parameter_is_ready() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.set_value("config", Value::Int(42)).unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("config");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "param_proc", vm)
        .with_parameters(vec![param]);

    assert!(process.is_ready());
}

#[test]
fn test_process_with_one_solved_process_parameter_is_ready() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .register_process("worker", ProcessState::Value(Value::Nil))
            .unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("worker");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "param_proc", vm)
        .with_parameters(vec![param]);

    assert!(process.is_ready());
}

// ============================================================================
// Test: Process with multiple parameters
// ============================================================================

#[test]
fn test_process_with_multiple_solved_parameters_is_ready() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("a", Value::Int(1)).unwrap();
        rspace_guard.tell("b", Value::Int(2)).unwrap();
        rspace_guard.tell("c", Value::Int(3)).unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let params = vec![
        Parameter::new("a"),
        Parameter::new("b"),
        Parameter::new("c"),
    ];
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "multi_param", vm)
        .with_parameters(params);

    assert!(process.is_ready());
}

#[test]
fn test_process_with_mixed_entry_type_parameters_is_ready() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        // Channel with value
        rspace_guard.tell("channel", Value::Int(1)).unwrap();
        // Process in Value state
        rspace_guard
            .register_process("proc", ProcessState::Value(Value::Int(2)))
            .unwrap();
        // Direct value
        rspace_guard.set_value("value", Value::Int(3)).unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let params = vec![
        Parameter::new("channel"),
        Parameter::new("proc"),
        Parameter::new("value"),
    ];
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "mixed_param", vm)
        .with_parameters(params);

    assert!(process.is_ready());
}

#[test]
fn test_process_with_some_unsolved_parameters_is_not_ready() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("a", Value::Int(1)).unwrap();
        // "b" is missing/unsolved
        rspace_guard.tell("c", Value::Int(3)).unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let params = vec![
        Parameter::new("a"),
        Parameter::new("b"), // unsolved
        Parameter::new("c"),
    ];
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "multi_param", vm)
        .with_parameters(params);

    assert!(!process.is_ready());
}

#[test]
fn test_process_with_all_unsolved_parameters_is_not_ready() {
    let rspace = shared_rspace();
    let vm = vm_with_shared_rspace(rspace);

    let params = vec![Parameter::new("a"), Parameter::new("b")];
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "multi_param", vm)
        .with_parameters(params);

    assert!(!process.is_ready());
}

// ============================================================================
// Test: Process execution blocked by unsolved parameters
// ============================================================================

#[test]
fn test_process_with_unsolved_parameters_cannot_execute() {
    let rspace = shared_rspace();
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("input");
    let mut process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "blocked", vm)
        .with_parameters(vec![param]);

    // Process is not ready due to unsolved parameter
    assert!(!process.is_ready());

    // Attempting to execute should fail or be blocked
    let result = process.execute();
    assert!(result.is_err());
}

#[test]
fn test_process_with_solved_parameters_can_execute() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("input", Value::Int(42)).unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("input");
    let mut process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "ready", vm)
        .with_parameters(vec![param]);

    assert!(process.is_ready());

    let result = process.execute();
    assert!(result.is_ok());
    assert!(matches!(process.state, ProcessState::Value(_)));
}

// ============================================================================
// Test: Parameter state changes dynamically
// ============================================================================

#[test]
fn test_parameter_becomes_solved_when_value_added_to_channel() {
    // Create an empty shared RSpace
    let rspace = shared_rspace();
    let vm = vm_with_shared_rspace(rspace.clone());

    let param = Parameter::new("input");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "dynamic", vm)
        .with_parameters(vec![param]);

    // Initially not ready (entry doesn't exist)
    assert!(!process.is_ready());

    // Add value to RSpace through the shared reference
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("input", Value::Int(42)).unwrap();
    }

    // Now should be ready
    assert!(process.is_ready());
}

#[test]
fn test_parameter_becomes_solved_when_process_completes() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .register_process("worker", ProcessState::Ready)
            .unwrap();
    }
    let vm = vm_with_shared_rspace(rspace.clone());

    let param = Parameter::new("worker");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "dynamic", vm)
        .with_parameters(vec![param]);

    // Initially not ready (worker is in Ready state, not Value)
    assert!(!process.is_ready());

    // Simulate worker completing
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .update_process("worker", ProcessState::Value(Value::Int(100)))
            .unwrap();
    }

    // Now should be ready
    assert!(process.is_ready());
}

#[test]
fn test_execute_processes_linked_by_parameters_wait_for_values() {
    let rspace = shared_rspace();

    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .register_process("proc_b", ProcessState::Wait)
            .unwrap();
        rspace_guard
            .register_process("proc_c", ProcessState::Wait)
            .unwrap();
    }

    let vm_a = vm_with_shared_rspace(rspace.clone());
    let params = vec![Parameter::new("proc_b"), Parameter::new("proc_c")];
    let mut proc_a = Process::with_vm(
        vec![
            Instruction::unary(Opcode::PUSH_BOOL, 1),
            Instruction::nullary(Opcode::HALT),
        ],
        "proc_a",
        vm_a,
    )
    .with_parameters(params);

    assert!(!proc_a.is_ready());
    assert!(proc_a.execute().is_err());

    let vm_b = vm_with_shared_rspace(rspace.clone());
    let mut proc_b = Process::with_vm(
        vec![
            Instruction::unary(Opcode::PUSH_INT, 10),
            Instruction::nullary(Opcode::HALT),
        ],
        "proc_b",
        vm_b,
    );
    let value_b = proc_b.execute().expect("proc_b executes");

    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .update_process("proc_b", ProcessState::Value(value_b))
            .unwrap();
    }

    assert!(!proc_a.is_ready());

    let vm_c = vm_with_shared_rspace(rspace.clone());
    let mut proc_c = Process::with_vm(
        vec![
            Instruction::unary(Opcode::PUSH_INT, 20),
            Instruction::nullary(Opcode::HALT),
        ],
        "proc_c",
        vm_c,
    );
    let value_c = proc_c.execute().expect("proc_c executes");

    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .update_process("proc_c", ProcessState::Value(value_c))
            .unwrap();
    }

    assert!(proc_a.is_ready());

    let result = proc_a.execute().expect("proc_a executes");
    assert_eq!(result, Value::Bool(true));
    assert!(matches!(proc_a.state, ProcessState::Value(_)));
}

// ============================================================================
// Test: ProcessHolder trait is_ready reflects parameter state
// ============================================================================

#[test]
fn test_process_holder_is_ready_with_parameters() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("input", Value::Int(42)).unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("input");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "holder_test", vm)
        .with_parameters(vec![param]);

    let holder: Box<dyn ProcessHolder> = Box::new(process);
    assert!(holder.is_ready());
}

#[test]
fn test_process_holder_not_ready_with_unsolved_parameters() {
    let rspace = shared_rspace();
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("input");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "holder_test", vm)
        .with_parameters(vec![param]);

    let holder: Box<dyn ProcessHolder> = Box::new(process);
    assert!(!holder.is_ready());
}

// ============================================================================
// Test: Parameter equality and cloning
// ============================================================================

#[test]
fn test_parameter_equality() {
    let p1 = Parameter::new("a");
    let p2 = Parameter::new("a");
    let p3 = Parameter::new("b");

    assert_eq!(p1, p2);
    assert_ne!(p1, p3);
}

#[test]
fn test_parameter_clone() {
    let p1 = Parameter::new("test");
    let p2 = p1.clone();

    assert_eq!(p1, p2);
    assert_eq!(p2.name(), "test");
}

#[test]
fn test_parameter_debug() {
    let param = Parameter::new("test");
    let debug_str = format!("{:?}", param);
    assert!(debug_str.contains("test"));
}

// ============================================================================
// Test: Process parameters() accessor
// ============================================================================

#[test]
fn test_process_parameters_returns_empty_by_default() {
    let process = halt_process("no_params");
    assert!(process.parameters().is_empty());
}

#[test]
fn test_process_parameters_returns_set_parameters() {
    let rspace = shared_rspace();
    let vm = vm_with_shared_rspace(rspace);

    let params = vec![Parameter::new("a"), Parameter::new("b")];
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "test", vm)
        .with_parameters(params.clone());

    assert_eq!(process.parameters().len(), 2);
    assert_eq!(process.parameters()[0].name(), "a");
    assert_eq!(process.parameters()[1].name(), "b");
}

// ============================================================================
// Test: Process state overrides parameter readiness
// ============================================================================

#[test]
fn test_process_in_wait_state_not_ready_even_with_solved_params() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("input", Value::Int(42)).unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("input");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "wait_test", vm)
        .with_parameters(vec![param])
        .with_state(ProcessState::Wait);

    // Even though parameter is solved, Wait state overrides readiness
    assert!(!process.is_ready());
}

#[test]
fn test_process_in_value_state_not_ready() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("input", Value::Int(42)).unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("input");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "value_test", vm)
        .with_parameters(vec![param])
        .with_state(ProcessState::Value(Value::Nil));

    // Terminal Value state means not ready (already executed)
    assert!(!process.is_ready());
}

#[test]
fn test_process_in_error_state_not_ready() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("input", Value::Int(42)).unwrap();
    }
    let vm = vm_with_shared_rspace(rspace);

    let param = Parameter::new("input");
    let process = Process::with_vm(vec![Instruction::nullary(Opcode::HALT)], "error_test", vm)
        .with_parameters(vec![param])
        .with_state(ProcessState::Error("failed".to_string()));

    // Terminal Error state means not ready
    assert!(!process.is_ready());
}

// ============================================================================
// Test: Entry types and their solved state
// ============================================================================

#[test]
fn test_channel_entry_solved_state() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("chan", Value::Int(1)).unwrap();
    }

    let rspace_guard = rspace.lock().unwrap();
    let entry = rspace_guard.get_entry("chan").unwrap();
    assert!(entry.is_channel());
    assert!(entry.is_solved()); // Non-empty channel is solved
}

#[test]
fn test_process_entry_solved_state() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard
            .register_process("proc", ProcessState::Ready)
            .unwrap();
    }

    let rspace_guard = rspace.lock().unwrap();
    let entry = rspace_guard.get_entry("proc").unwrap();
    assert!(entry.is_process());
    assert!(!entry.is_solved()); // Ready state is not solved
}

#[test]
fn test_value_entry_always_solved() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.set_value("val", Value::Int(42)).unwrap();
    }

    let rspace_guard = rspace.lock().unwrap();
    let entry = rspace_guard.get_entry("val").unwrap();
    assert!(entry.is_value());
    assert!(entry.is_solved()); // Value is always solved
}

// ============================================================================
// Test: FIFO behavior of channel entries
// ============================================================================

#[test]
fn test_channel_fifo_order() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("queue", Value::Int(1)).unwrap();
        rspace_guard.tell("queue", Value::Int(2)).unwrap();
        rspace_guard.tell("queue", Value::Int(3)).unwrap();
    }

    {
        let mut rspace_guard = rspace.lock().unwrap();
        assert_eq!(rspace_guard.ask("queue").unwrap(), Some(Value::Int(1)));
        assert_eq!(rspace_guard.ask("queue").unwrap(), Some(Value::Int(2)));
        assert_eq!(rspace_guard.ask("queue").unwrap(), Some(Value::Int(3)));
        assert_eq!(rspace_guard.ask("queue").unwrap(), None);
    }
}

#[test]
fn test_parameter_solved_with_multiple_values_in_channel() {
    let rspace = shared_rspace();
    {
        let mut rspace_guard = rspace.lock().unwrap();
        rspace_guard.tell("queue", Value::Int(1)).unwrap();
        rspace_guard.tell("queue", Value::Int(2)).unwrap();
    }

    let param = Parameter::new("queue");
    let rspace_guard = rspace.lock().unwrap();
    // Channel is non-empty, so parameter is solved
    assert!(param.is_solved(rspace_guard.as_ref()));
}
