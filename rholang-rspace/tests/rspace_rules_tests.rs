//! Comprehensive tests for RSpace rules as defined in rspace.md.
//!
//! This test module verifies:
//! - RSpace Interface (tell, ask, peek, reset)
//! - Entry types (Channel, Process, Value)
//! - Stored Values (all Value variants)
//! - Process Storage (Value::Par)
//! - Process States (wait, ready, value, error)
//! - Execution Flow with RSpace
//! - Both RSpace Implementations (InMemoryRSpace, PathMapRSpace)
//! - FIFO ordering

use anyhow::Result;
use rholang_bytecode::core::instructions::Instruction;
use rholang_bytecode::core::Opcode;
use rholang_process::{Process, ProcessEvent};
use rholang_rspace::{
    Entry, InMemoryRSpace, PathMapRSpace, ProcessHolder, ProcessState, RSpace, Value,
};
use std::sync::Arc;

// =============================================================================
// RSpace Interface Tests (tell, ask, peek, reset)
// =============================================================================

macro_rules! rspace_interface_tests {
    ($rspace_type:ty, $mod_name:ident) => {
        mod $mod_name {
            use super::*;

            fn make_rspace() -> Box<dyn RSpace> {
                Box::new(<$rspace_type>::new())
            }

            #[test]
            fn test_tell_stores_value() -> Result<()> {
                let mut rspace = make_rspace();

                rspace.tell("test", Value::Int(42))?;

                // Verify value was stored
                let result = rspace.peek("test")?;
                assert_eq!(result, Some(Value::Int(42)));
                Ok(())
            }

            #[test]
            fn test_ask_removes_value() -> Result<()> {
                let mut rspace = make_rspace();

                rspace.tell("test", Value::Int(42))?;

                // First ask should return the value
                let result = rspace.ask("test")?;
                assert_eq!(result, Some(Value::Int(42)));

                // Second ask should return None (value was removed)
                let result = rspace.ask("test")?;
                assert_eq!(result, None);
                Ok(())
            }

            #[test]
            fn test_peek_does_not_remove_value() -> Result<()> {
                let mut rspace = make_rspace();

                rspace.tell("test", Value::Int(42))?;

                // Multiple peeks should return the same value
                assert_eq!(rspace.peek("test")?, Some(Value::Int(42)));
                assert_eq!(rspace.peek("test")?, Some(Value::Int(42)));
                assert_eq!(rspace.peek("test")?, Some(Value::Int(42)));
                Ok(())
            }

            #[test]
            fn test_reset_clears_all_storage() -> Result<()> {
                let mut rspace = make_rspace();

                rspace.tell("a", Value::Int(1))?;
                rspace.tell("b", Value::Int(2))?;

                rspace.reset();

                assert_eq!(rspace.ask("a")?, None);
                assert_eq!(rspace.ask("b")?, None);
                Ok(())
            }

            #[test]
            fn test_ask_empty_channel_returns_none() -> Result<()> {
                let mut rspace = make_rspace();

                assert_eq!(rspace.ask("empty")?, None);
                Ok(())
            }

            #[test]
            fn test_peek_empty_channel_returns_none() -> Result<()> {
                let rspace = make_rspace();

                assert_eq!(rspace.peek("empty")?, None);
                Ok(())
            }

            // =============================================================================
            // FIFO Ordering Tests
            // =============================================================================

            #[test]
            fn test_fifo_ordering_basic() -> Result<()> {
                let mut rspace = make_rspace();

                // Insert in order
                rspace.tell("fifo", Value::Int(1))?;
                rspace.tell("fifo", Value::Int(2))?;
                rspace.tell("fifo", Value::Int(3))?;

                // Should come out in the same order (FIFO)
                assert_eq!(rspace.ask("fifo")?, Some(Value::Int(1)));
                assert_eq!(rspace.ask("fifo")?, Some(Value::Int(2)));
                assert_eq!(rspace.ask("fifo")?, Some(Value::Int(3)));
                assert_eq!(rspace.ask("fifo")?, None);
                Ok(())
            }

            #[test]
            fn test_fifo_ordering_many_values() -> Result<()> {
                let mut rspace = make_rspace();

                // Insert many values
                for i in 0..100 {
                    rspace.tell("many", Value::Int(i))?;
                }

                // Should come out in order
                for i in 0..100 {
                    assert_eq!(rspace.ask("many")?, Some(Value::Int(i)));
                }
                Ok(())
            }

            #[test]
            fn test_peek_returns_oldest_value() -> Result<()> {
                let mut rspace = make_rspace();

                rspace.tell("peek", Value::Int(100))?;
                rspace.tell("peek", Value::Int(200))?;

                // Peek should return the oldest (first inserted)
                assert_eq!(rspace.peek("peek")?, Some(Value::Int(100)));

                // After ask removes oldest, peek should return next oldest
                rspace.ask("peek")?;
                assert_eq!(rspace.peek("peek")?, Some(Value::Int(200)));
                Ok(())
            }

            // =============================================================================
            // Stored Values Tests (all Value variants)
            // =============================================================================

            #[test]
            fn test_store_primitive_values() -> Result<()> {
                let mut rspace = make_rspace();

                // Int
                rspace.tell("prim", Value::Int(i64::MAX))?;
                assert_eq!(rspace.ask("prim")?, Some(Value::Int(i64::MAX)));

                // Bool
                rspace.tell("prim", Value::Bool(true))?;
                assert_eq!(rspace.ask("prim")?, Some(Value::Bool(true)));

                // Str
                rspace.tell("prim", Value::Str("hello".into()))?;
                assert_eq!(rspace.ask("prim")?, Some(Value::Str("hello".into())));

                // Name
                rspace.tell("prim", Value::Name("@test".into()))?;
                assert_eq!(rspace.ask("prim")?, Some(Value::Name("@test".into())));

                // Nil
                rspace.tell("prim", Value::Nil)?;
                assert_eq!(rspace.ask("prim")?, Some(Value::Nil));

                Ok(())
            }

            #[test]
            fn test_store_collection_values() -> Result<()> {
                let mut rspace = make_rspace();

                // List
                let list = Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
                rspace.tell("coll", list.clone())?;
                assert_eq!(rspace.ask("coll")?, Some(list));

                // Tuple
                let tuple = Value::Tuple(vec![Value::Str("a".into()), Value::Bool(false)]);
                rspace.tell("coll", tuple.clone())?;
                assert_eq!(rspace.ask("coll")?, Some(tuple));

                // Map
                let map = Value::Map(vec![
                    (Value::Str("key1".into()), Value::Int(10)),
                    (Value::Str("key2".into()), Value::Int(20)),
                ]);
                rspace.tell("coll", map.clone())?;
                assert_eq!(rspace.ask("coll")?, Some(map));

                Ok(())
            }

            #[test]
            fn test_store_nested_collections() -> Result<()> {
                let mut rspace = make_rspace();

                // Deeply nested structure
                let nested = Value::List(vec![
                    Value::Map(vec![(
                        Value::Str("inner".into()),
                        Value::Tuple(vec![Value::Int(1), Value::Int(2)]),
                    )]),
                    Value::List(vec![Value::Bool(true), Value::Nil]),
                ]);

                rspace.tell("nested", nested.clone())?;
                assert_eq!(rspace.ask("nested")?, Some(nested));
                Ok(())
            }

            // =============================================================================
            // Process Storage Tests (Value::Par)
            // =============================================================================

            #[test]
            fn test_store_process_in_par() -> Result<()> {
                let mut rspace = make_rspace();

                let process = Process::new(vec![Instruction::nullary(Opcode::HALT)], "test_proc");

                rspace.tell("proc", Value::Par(vec![process.clone().boxed()]))?;

                match rspace.ask("proc")? {
                    Some(Value::Par(procs)) => {
                        assert_eq!(procs.len(), 1);
                        assert_eq!(procs[0].source_ref(), "test_proc");
                    }
                    other => panic!("Expected Par, got {:?}", other),
                }
                Ok(())
            }

            #[test]
            fn test_store_multiple_processes_in_par() -> Result<()> {
                let mut rspace = make_rspace();

                let proc1 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "proc1");
                let proc2 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "proc2");
                let proc3 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "proc3");

                rspace.tell(
                    "multi",
                    Value::Par(vec![proc1.boxed(), proc2.boxed(), proc3.boxed()]),
                )?;

                match rspace.ask("multi")? {
                    Some(Value::Par(procs)) => {
                        assert_eq!(procs.len(), 3);
                        assert_eq!(procs[0].source_ref(), "proc1");
                        assert_eq!(procs[1].source_ref(), "proc2");
                        assert_eq!(procs[2].source_ref(), "proc3");
                    }
                    other => panic!("Expected Par with 3 processes, got {:?}", other),
                }
                Ok(())
            }

            #[test]
            fn test_process_source_ref_preserved() -> Result<()> {
                let mut rspace = make_rspace();

                let process = Process::new(
                    vec![Instruction::nullary(Opcode::HALT)],
                    "my_unique_ref_123",
                );

                rspace.tell("ref", Value::Par(vec![process.boxed()]))?;

                match rspace.ask("ref")? {
                    Some(Value::Par(procs)) => {
                        assert_eq!(procs[0].source_ref(), "my_unique_ref_123");
                    }
                    _ => panic!("Expected Par"),
                }
                Ok(())
            }

            // =============================================================================
            // Entry Type Tests
            // =============================================================================

            #[test]
            fn test_get_entry_returns_channel() -> Result<()> {
                let mut rspace = make_rspace();
                rspace.tell("channel", Value::Int(42))?;

                let entry = rspace.get_entry("channel").unwrap();
                assert!(entry.is_channel());
                Ok(())
            }

            #[test]
            fn test_get_entry_returns_process() -> Result<()> {
                let mut rspace = make_rspace();
                rspace.register_process("worker", ProcessState::Ready)?;

                let entry = rspace.get_entry("worker").unwrap();
                assert!(entry.is_process());
                Ok(())
            }

            #[test]
            fn test_get_entry_returns_value() -> Result<()> {
                let mut rspace = make_rspace();
                rspace.set_value("config", Value::Str("prod".into()))?;

                let entry = rspace.get_entry("config").unwrap();
                assert!(entry.is_value());
                Ok(())
            }

            #[test]
            fn test_get_entry_nonexistent_returns_none() {
                let rspace = make_rspace();
                assert!(rspace.get_entry("nonexistent").is_none());
            }

            // =============================================================================
            // is_solved Tests
            // =============================================================================

            #[test]
            fn test_is_solved_empty_channel() -> Result<()> {
                let mut rspace = make_rspace();
                rspace.tell("chan", Value::Int(1))?;
                rspace.ask("chan")?; // Drain it

                assert!(!rspace.is_solved("chan"));
                Ok(())
            }

            #[test]
            fn test_is_solved_nonempty_channel() -> Result<()> {
                let mut rspace = make_rspace();
                rspace.tell("chan", Value::Int(1))?;

                assert!(rspace.is_solved("chan"));
                Ok(())
            }

            #[test]
            fn test_is_solved_process_ready() -> Result<()> {
                let mut rspace = make_rspace();
                rspace.register_process("worker", ProcessState::Ready)?;

                assert!(!rspace.is_solved("worker"));
                Ok(())
            }

            #[test]
            fn test_is_solved_process_value() -> Result<()> {
                let mut rspace = make_rspace();
                rspace.register_process("worker", ProcessState::Value(Value::Int(42)))?;

                assert!(rspace.is_solved("worker"));
                Ok(())
            }

            #[test]
            fn test_is_solved_value_entry() -> Result<()> {
                let mut rspace = make_rspace();
                rspace.set_value("config", Value::Int(1))?;

                assert!(rspace.is_solved("config"));
                Ok(())
            }

            #[test]
            fn test_is_solved_nonexistent() {
                let rspace = make_rspace();
                assert!(!rspace.is_solved("missing"));
            }
        }
    };
}

// Generate tests for both implementations
rspace_interface_tests!(InMemoryRSpace, in_memory_rspace_tests);
rspace_interface_tests!(PathMapRSpace, path_map_rspace_tests);

// =============================================================================
// Process State Tests (wait, ready, value, error)
// =============================================================================

mod process_state_tests {
    use super::*;

    #[test]
    fn test_process_default_state_is_ready() {
        let process = Process::new(vec![Instruction::nullary(Opcode::HALT)], "test");
        assert!(matches!(process.state, ProcessState::Ready));
        assert!(process.is_ready());
    }

    #[test]
    fn test_process_wait_state() {
        let process = Process::new(vec![Instruction::nullary(Opcode::HALT)], "wait_test")
            .with_state(ProcessState::Wait);

        assert!(matches!(process.state, ProcessState::Wait));
        assert!(!process.is_ready());
    }

    #[test]
    fn test_process_value_state_is_terminal() {
        let process = Process::new(vec![Instruction::nullary(Opcode::HALT)], "value_test")
            .with_state(ProcessState::Value(Value::Int(42)));

        assert!(matches!(process.state, ProcessState::Value(_)));
        assert!(!process.is_ready()); // Terminal states are not "ready"
    }

    #[test]
    fn test_process_error_state_is_terminal() {
        let process = Process::new(vec![Instruction::nullary(Opcode::HALT)], "error_test")
            .with_state(ProcessState::Error("test error".to_string()));

        assert!(matches!(process.state, ProcessState::Error(_)));
        assert!(!process.is_ready()); // Terminal states are not "ready"
    }

    #[test]
    fn test_process_states_stored_correctly() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        let ready_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "ready")
            .with_state(ProcessState::Ready);
        let wait_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "wait")
            .with_state(ProcessState::Wait);
        let value_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "value")
            .with_state(ProcessState::Value(Value::Int(99)));
        let error_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "error")
            .with_state(ProcessState::Error("fail".to_string()));

        rspace.tell(
            "states",
            Value::Par(vec![
                ready_proc.boxed(),
                wait_proc.boxed(),
                value_proc.boxed(),
                error_proc.boxed(),
            ]),
        )?;

        match rspace.ask("states")? {
            Some(Value::Par(procs)) => {
                assert_eq!(procs.len(), 4);
                assert!(matches!(procs[0].state(), ProcessState::Ready));
                assert!(matches!(procs[1].state(), ProcessState::Wait));
                assert!(matches!(procs[2].state(), ProcessState::Value(_)));
                assert!(matches!(procs[3].state(), ProcessState::Error(_)));
            }
            other => panic!("Expected Par, got {:?}", other),
        }
        Ok(())
    }
}

// =============================================================================
// Process Registration and Update Tests
// =============================================================================

mod process_registration_tests {
    use super::*;

    #[test]
    fn test_register_process() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.register_process("worker", ProcessState::Ready)?;

        assert_eq!(
            rspace.get_process_state("worker"),
            Some(ProcessState::Ready)
        );
        Ok(())
    }

    #[test]
    fn test_update_process_state() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.register_process("worker", ProcessState::Ready)?;
        rspace.update_process("worker", ProcessState::Value(Value::Int(42)))?;

        assert_eq!(
            rspace.get_process_state("worker"),
            Some(ProcessState::Value(Value::Int(42)))
        );
        Ok(())
    }

    #[test]
    fn test_register_duplicate_fails() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.register_process("worker", ProcessState::Ready)?;
        assert!(rspace
            .register_process("worker", ProcessState::Ready)
            .is_err());
        Ok(())
    }

    #[test]
    fn test_update_nonexistent_fails() {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        assert!(rspace
            .update_process("missing", ProcessState::Ready)
            .is_err());
    }

    #[test]
    fn test_get_process_state_nonexistent() {
        let rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        assert!(rspace.get_process_state("missing").is_none());
    }
}

// =============================================================================
// Value Entry Tests
// =============================================================================

mod value_entry_tests {
    use super::*;

    #[test]
    fn test_set_value() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.set_value("config", Value::Str("prod".into()))?;

        assert_eq!(rspace.get_value("config"), Some(Value::Str("prod".into())));
        Ok(())
    }

    #[test]
    fn test_set_value_duplicate_fails() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.set_value("config", Value::Int(1))?;
        assert!(rspace.set_value("config", Value::Int(2)).is_err());
        Ok(())
    }

    #[test]
    fn test_get_value_nonexistent() {
        let rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        assert!(rspace.get_value("missing").is_none());
    }

    #[test]
    fn test_value_always_solved() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.set_value("config", Value::Int(1))?;

        assert!(rspace.is_solved("config"));
        Ok(())
    }
}

// =============================================================================
// Entry Type Conflict Tests
// =============================================================================

mod entry_conflict_tests {
    use super::*;

    #[test]
    fn test_tell_on_process_entry_fails() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.register_process("worker", ProcessState::Ready)?;
        assert!(rspace.tell("worker", Value::Int(1)).is_err());
        Ok(())
    }

    #[test]
    fn test_tell_on_value_entry_fails() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.set_value("config", Value::Int(1))?;
        assert!(rspace.tell("config", Value::Int(2)).is_err());
        Ok(())
    }

    #[test]
    fn test_ask_on_process_entry_fails() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.register_process("worker", ProcessState::Ready)?;
        assert!(rspace.ask("worker").is_err());
        Ok(())
    }

    #[test]
    fn test_ask_on_value_entry_fails() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.set_value("config", Value::Int(1))?;
        assert!(rspace.ask("config").is_err());
        Ok(())
    }

    #[test]
    fn test_peek_on_process_entry_fails() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.register_process("worker", ProcessState::Ready)?;
        assert!(rspace.peek("worker").is_err());
        Ok(())
    }

    #[test]
    fn test_peek_on_value_entry_fails() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.set_value("config", Value::Int(1))?;
        assert!(rspace.peek("config").is_err());
        Ok(())
    }

    #[test]
    fn test_register_process_on_channel_fails() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.tell("chan", Value::Int(1))?;
        assert!(rspace
            .register_process("chan", ProcessState::Ready)
            .is_err());
        Ok(())
    }

    #[test]
    fn test_set_value_on_channel_fails() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());

        rspace.tell("chan", Value::Int(1))?;
        assert!(rspace.set_value("chan", Value::Int(2)).is_err());
        Ok(())
    }
}

// =============================================================================
// Execution Flow Tests
// =============================================================================

mod execution_flow_tests {
    use super::*;

    #[test]
    fn test_process_execution_updates_state_to_value() -> Result<()> {
        // Create a simple process that returns a value
        let mut process = Process::new(
            vec![
                Instruction::unary(Opcode::PUSH_INT, 42),
                Instruction::nullary(Opcode::HALT),
            ],
            "test_exec",
        );

        assert!(matches!(process.state, ProcessState::Ready));

        // Execute the process
        let result = process.execute();
        assert!(result.is_ok());

        // State should transition to Value
        assert!(matches!(process.state, ProcessState::Value(_)));
        Ok(())
    }

    #[test]
    fn test_process_vm_preserved_across_executions() -> Result<()> {
        let mut process = Process::new(
            vec![
                Instruction::unary(Opcode::PUSH_INT, 100),
                Instruction::nullary(Opcode::HALT),
            ],
            "vm_test",
        );

        // VM is always present (non-optional)
        let stack_before = process.vm.stack.len();

        process.execute()?;

        // VM should still be present after execution
        // Stack may have changed but VM is preserved
        let _ = stack_before; // Just verify VM access works
        Ok(())
    }

    #[test]
    fn test_process_event_callback_on_success() -> Result<()> {
        use std::sync::atomic::{AtomicBool, Ordering};

        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_called_clone = callback_called.clone();

        let handler: Arc<dyn Fn(ProcessEvent) + Send + Sync> =
            Arc::new(move |event: ProcessEvent| {
                if let ProcessEvent::Value(_) = event {
                    callback_called_clone.store(true, Ordering::SeqCst);
                }
            });

        let mut process = Process::new(
            vec![
                Instruction::unary(Opcode::PUSH_INT, 1),
                Instruction::nullary(Opcode::HALT),
            ],
            "callback_test",
        );

        process.execute_with_event(Some(&handler))?;

        assert!(callback_called.load(Ordering::SeqCst));
        Ok(())
    }

    #[test]
    fn test_process_event_callback_on_error() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_called_clone = callback_called.clone();

        let handler: Arc<dyn Fn(ProcessEvent) + Send + Sync> =
            Arc::new(move |event: ProcessEvent| {
                if let ProcessEvent::Error(_) = event {
                    callback_called_clone.store(true, Ordering::SeqCst);
                }
            });

        // Create a process that will fail (division by zero)
        let mut process = Process::new(
            vec![
                Instruction::unary(Opcode::PUSH_INT, 1), // Push dividend
                Instruction::unary(Opcode::PUSH_INT, 0), // Push divisor (zero)
                Instruction::nullary(Opcode::DIV),       // Division by zero will error
                Instruction::nullary(Opcode::HALT),
            ],
            "error_callback_test",
        );

        let _ = process.execute_with_event(Some(&handler));

        // Error callback should have been called
        assert!(callback_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_terminal_state_process_should_not_be_ready() {
        let value_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "v")
            .with_state(ProcessState::Value(Value::Nil));

        let error_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "e")
            .with_state(ProcessState::Error("err".to_string()));

        // Terminal states should not be considered "ready"
        assert!(!value_proc.is_ready());
        assert!(!error_proc.is_ready());
    }

    #[test]
    fn test_reexecute_value_state_returns_cached_value() {
        // A process in Value state should return its cached value when re-executed
        let mut process = Process::new(
            vec![
                Instruction::unary(Opcode::PUSH_INT, 42),
                Instruction::nullary(Opcode::HALT),
            ],
            "cached_value",
        )
        .with_state(ProcessState::Value(Value::Int(99)));

        // Should return the cached value (99), not execute the code (which would push 42)
        let result = process.execute();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(99));
    }

    #[test]
    fn test_reexecute_error_state_returns_error() {
        let mut process = Process::new(
            vec![
                Instruction::unary(Opcode::PUSH_INT, 42),
                Instruction::nullary(Opcode::HALT),
            ],
            "error_state",
        )
        .with_state(ProcessState::Error("previous error".to_string()));

        // Should return error, not execute the code
        let result = process.execute();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("cannot re-execute"));
    }

    #[test]
    fn test_execute_wait_state_returns_error() {
        let mut process = Process::new(
            vec![
                Instruction::unary(Opcode::PUSH_INT, 42),
                Instruction::nullary(Opcode::HALT),
            ],
            "wait_state",
        )
        .with_state(ProcessState::Wait);

        // Should return error, not execute the code
        let result = process.execute();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("cannot execute"));
        assert!(err_msg.contains("wait state"));
    }
}

// =============================================================================
// Concurrency Tests
// =============================================================================

mod concurrency_tests {
    use super::*;
    use std::sync::Mutex;
    use std::thread;

    #[test]
    fn test_rspace_behind_arc_mutex() -> Result<()> {
        let rspace: Arc<Mutex<Box<dyn RSpace>>> =
            Arc::new(Mutex::new(Box::new(InMemoryRSpace::new())));

        // Write from one "thread" context
        {
            let mut guard = rspace.lock().unwrap();
            guard.tell("concurrent", Value::Int(42))?;
        }

        // Read from another
        {
            let mut guard = rspace.lock().unwrap();
            assert_eq!(guard.ask("concurrent")?, Some(Value::Int(42)));
        }

        Ok(())
    }

    #[test]
    fn test_concurrent_writes_to_different_channels() -> Result<()> {
        let rspace: Arc<Mutex<Box<dyn RSpace>>> =
            Arc::new(Mutex::new(Box::new(InMemoryRSpace::new())));

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let rspace_clone = rspace.clone();
                thread::spawn(move || {
                    let channel = format!("ch{}", i);
                    let mut guard = rspace_clone.lock().unwrap();
                    guard.tell(&channel, Value::Int(i)).unwrap();
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // Verify all writes succeeded
        let mut guard = rspace.lock().unwrap();
        for i in 0..10 {
            let channel = format!("ch{}", i);
            assert_eq!(guard.ask(&channel)?, Some(Value::Int(i)));
        }

        Ok(())
    }

    #[test]
    fn test_concurrent_writes_to_same_channel() -> Result<()> {
        let rspace: Arc<Mutex<Box<dyn RSpace>>> =
            Arc::new(Mutex::new(Box::new(InMemoryRSpace::new())));

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let rspace_clone = rspace.clone();
                thread::spawn(move || {
                    let mut guard = rspace_clone.lock().unwrap();
                    guard.tell("shared", Value::Int(i)).unwrap();
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // All values should be stored
        let mut guard = rspace.lock().unwrap();
        let mut values = Vec::new();
        while let Some(val) = guard.ask("shared")? {
            values.push(val);
        }
        assert_eq!(values.len(), 10);

        Ok(())
    }
}

// =============================================================================
// Entry Solved State Tests
// =============================================================================

mod entry_solved_tests {
    use super::*;

    #[test]
    fn test_entry_channel_solved_when_nonempty() {
        let entry = Entry::Channel(vec![Value::Int(1)]);
        assert!(entry.is_solved());
    }

    #[test]
    fn test_entry_channel_unsolved_when_empty() {
        let entry = Entry::Channel(vec![]);
        assert!(!entry.is_solved());
    }

    #[test]
    fn test_entry_process_solved_when_value_state() {
        let entry = Entry::Process {
            state: ProcessState::Value(Value::Int(42)),
        };
        assert!(entry.is_solved());
    }

    #[test]
    fn test_entry_process_unsolved_when_ready() {
        let entry = Entry::Process {
            state: ProcessState::Ready,
        };
        assert!(!entry.is_solved());
    }

    #[test]
    fn test_entry_process_unsolved_when_wait() {
        let entry = Entry::Process {
            state: ProcessState::Wait,
        };
        assert!(!entry.is_solved());
    }

    #[test]
    fn test_entry_process_unsolved_when_error() {
        let entry = Entry::Process {
            state: ProcessState::Error("err".into()),
        };
        assert!(!entry.is_solved());
    }

    #[test]
    fn test_entry_value_always_solved() {
        let entry = Entry::Value(Value::Nil);
        assert!(entry.is_solved());
    }
}
