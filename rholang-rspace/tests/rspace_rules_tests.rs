//! Comprehensive tests for RSpace rules as defined in rspace.md.
//!
//! This test module verifies:
//! - RSpace Interface (tell, ask, peek, reset)
//! - Channel Naming and Kinds (@<kind>:<name> format)
//! - Stored Values (all Value variants)
//! - Process Storage (Value::Par)
//! - Process States (wait, ready, value, error)
//! - Ready-Queue Drain semantics
//! - Execution Flow with RSpace
//! - Both RSpace Implementations (InMemoryRSpace, PathMapRSpace)
//! - FIFO ordering and channel-kind validation

use anyhow::Result;
use rholang_bytecode::core::instructions::Instruction;
use rholang_bytecode::core::Opcode;
use rholang_rspace::{
    drain_ready_processes, InMemoryRSpace, PathMapRSpace, Process, ProcessEvent, ProcessState,
    RSpace, Value,
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
                let channel = "@0:test".to_string();

                rspace.tell(0, channel.clone(), Value::Int(42))?;

                // Verify value was stored
                let result = rspace.peek(0, channel)?;
                assert_eq!(result, Some(Value::Int(42)));
                Ok(())
            }

            #[test]
            fn test_ask_removes_value() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:test".to_string();

                rspace.tell(0, channel.clone(), Value::Int(42))?;

                // First ask should return the value
                let result = rspace.ask(0, channel.clone())?;
                assert_eq!(result, Some(Value::Int(42)));

                // Second ask should return None (value was removed)
                let result = rspace.ask(0, channel)?;
                assert_eq!(result, None);
                Ok(())
            }

            #[test]
            fn test_peek_does_not_remove_value() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:test".to_string();

                rspace.tell(0, channel.clone(), Value::Int(42))?;

                // Multiple peeks should return the same value
                assert_eq!(rspace.peek(0, channel.clone())?, Some(Value::Int(42)));
                assert_eq!(rspace.peek(0, channel.clone())?, Some(Value::Int(42)));
                assert_eq!(rspace.peek(0, channel)?, Some(Value::Int(42)));
                Ok(())
            }

            #[test]
            fn test_reset_clears_all_storage() -> Result<()> {
                let mut rspace = make_rspace();
                let channel1 = "@0:a".to_string();
                let channel2 = "@1:b".to_string();

                rspace.tell(0, channel1.clone(), Value::Int(1))?;
                rspace.tell(1, channel2.clone(), Value::Int(2))?;

                rspace.reset();

                assert_eq!(rspace.ask(0, channel1)?, None);
                assert_eq!(rspace.ask(1, channel2)?, None);
                Ok(())
            }

            #[test]
            fn test_ask_empty_channel_returns_none() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:empty".to_string();

                assert_eq!(rspace.ask(0, channel)?, None);
                Ok(())
            }

            #[test]
            fn test_peek_empty_channel_returns_none() -> Result<()> {
                let rspace = make_rspace();
                let channel = "@0:empty".to_string();

                assert_eq!(rspace.peek(0, channel)?, None);
                Ok(())
            }

            // =============================================================================
            // Channel Naming and Kind Validation Tests
            // =============================================================================

            #[test]
            fn test_channel_kind_must_match_prefix() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:test".to_string();

                // kind 0 should work with @0:test
                assert!(rspace.tell(0, channel.clone(), Value::Int(42)).is_ok());

                // kind 1 should fail with @0:test
                assert!(rspace.tell(1, channel.clone(), Value::Int(42)).is_err());
                assert!(rspace.ask(1, channel.clone()).is_err());
                assert!(rspace.peek(1, channel).is_err());
                Ok(())
            }

            #[test]
            fn test_channel_without_proper_prefix_fails() -> Result<()> {
                let mut rspace = make_rspace();

                // Missing @ prefix
                assert!(rspace
                    .tell(0, "0:test".to_string(), Value::Int(42))
                    .is_err());

                // Wrong kind number
                assert!(rspace
                    .tell(0, "@5:test".to_string(), Value::Int(42))
                    .is_err());
                Ok(())
            }

            #[test]
            fn test_various_valid_channel_kinds() -> Result<()> {
                let mut rspace = make_rspace();

                // Test various kind values
                for kind in [0u16, 1, 100, 1000, u16::MAX] {
                    let channel = format!("@{}:channel", kind);
                    rspace.tell(kind, channel.clone(), Value::Int(kind as i64))?;
                    assert_eq!(rspace.ask(kind, channel)?, Some(Value::Int(kind as i64)));
                }
                Ok(())
            }

            // =============================================================================
            // FIFO Ordering Tests
            // =============================================================================

            #[test]
            fn test_fifo_ordering_basic() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:fifo".to_string();

                // Insert in order
                rspace.tell(0, channel.clone(), Value::Int(1))?;
                rspace.tell(0, channel.clone(), Value::Int(2))?;
                rspace.tell(0, channel.clone(), Value::Int(3))?;

                // Should come out in the same order (FIFO)
                assert_eq!(rspace.ask(0, channel.clone())?, Some(Value::Int(1)));
                assert_eq!(rspace.ask(0, channel.clone())?, Some(Value::Int(2)));
                assert_eq!(rspace.ask(0, channel.clone())?, Some(Value::Int(3)));
                assert_eq!(rspace.ask(0, channel)?, None);
                Ok(())
            }

            #[test]
            fn test_fifo_ordering_many_values() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:many".to_string();

                // Insert many values
                for i in 0..100 {
                    rspace.tell(0, channel.clone(), Value::Int(i))?;
                }

                // Should come out in order
                for i in 0..100 {
                    assert_eq!(rspace.ask(0, channel.clone())?, Some(Value::Int(i)));
                }
                Ok(())
            }

            #[test]
            fn test_peek_returns_oldest_value() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:peek".to_string();

                rspace.tell(0, channel.clone(), Value::Int(100))?;
                rspace.tell(0, channel.clone(), Value::Int(200))?;

                // Peek should return the oldest (first inserted)
                assert_eq!(rspace.peek(0, channel.clone())?, Some(Value::Int(100)));

                // After ask removes oldest, peek should return next oldest
                rspace.ask(0, channel.clone())?;
                assert_eq!(rspace.peek(0, channel)?, Some(Value::Int(200)));
                Ok(())
            }

            // =============================================================================
            // Stored Values Tests (all Value variants)
            // =============================================================================

            #[test]
            fn test_store_primitive_values() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:prim".to_string();

                // Int
                rspace.tell(0, channel.clone(), Value::Int(i64::MAX))?;
                assert_eq!(rspace.ask(0, channel.clone())?, Some(Value::Int(i64::MAX)));

                // Bool
                rspace.tell(0, channel.clone(), Value::Bool(true))?;
                assert_eq!(rspace.ask(0, channel.clone())?, Some(Value::Bool(true)));

                // Str
                rspace.tell(0, channel.clone(), Value::Str("hello".into()))?;
                assert_eq!(
                    rspace.ask(0, channel.clone())?,
                    Some(Value::Str("hello".into()))
                );

                // Name
                rspace.tell(0, channel.clone(), Value::Name("@test".into()))?;
                assert_eq!(
                    rspace.ask(0, channel.clone())?,
                    Some(Value::Name("@test".into()))
                );

                // Nil
                rspace.tell(0, channel.clone(), Value::Nil)?;
                assert_eq!(rspace.ask(0, channel)?, Some(Value::Nil));

                Ok(())
            }

            #[test]
            fn test_store_collection_values() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:coll".to_string();

                // List
                let list = Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
                rspace.tell(0, channel.clone(), list.clone())?;
                assert_eq!(rspace.ask(0, channel.clone())?, Some(list));

                // Tuple
                let tuple = Value::Tuple(vec![Value::Str("a".into()), Value::Bool(false)]);
                rspace.tell(0, channel.clone(), tuple.clone())?;
                assert_eq!(rspace.ask(0, channel.clone())?, Some(tuple));

                // Map
                let map = Value::Map(vec![
                    (Value::Str("key1".into()), Value::Int(10)),
                    (Value::Str("key2".into()), Value::Int(20)),
                ]);
                rspace.tell(0, channel.clone(), map.clone())?;
                assert_eq!(rspace.ask(0, channel)?, Some(map));

                Ok(())
            }

            #[test]
            fn test_store_nested_collections() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:nested".to_string();

                // Deeply nested structure
                let nested = Value::List(vec![
                    Value::Map(vec![(
                        Value::Str("inner".into()),
                        Value::Tuple(vec![Value::Int(1), Value::Int(2)]),
                    )]),
                    Value::List(vec![Value::Bool(true), Value::Nil]),
                ]);

                rspace.tell(0, channel.clone(), nested.clone())?;
                assert_eq!(rspace.ask(0, channel)?, Some(nested));
                Ok(())
            }

            // =============================================================================
            // Process Storage Tests (Value::Par)
            // =============================================================================

            #[test]
            fn test_store_process_in_par() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:proc".to_string();

                let process = Process::new(vec![Instruction::nullary(Opcode::HALT)], "test_proc");

                rspace.tell(0, channel.clone(), Value::Par(vec![process.clone()]))?;

                match rspace.ask(0, channel)? {
                    Some(Value::Par(procs)) => {
                        assert_eq!(procs.len(), 1);
                        assert_eq!(procs[0].source_ref, "test_proc");
                    }
                    other => panic!("Expected Par, got {:?}", other),
                }
                Ok(())
            }

            #[test]
            fn test_store_multiple_processes_in_par() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:multi".to_string();

                let proc1 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "proc1");
                let proc2 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "proc2");
                let proc3 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "proc3");

                rspace.tell(0, channel.clone(), Value::Par(vec![proc1, proc2, proc3]))?;

                match rspace.ask(0, channel)? {
                    Some(Value::Par(procs)) => {
                        assert_eq!(procs.len(), 3);
                        assert_eq!(procs[0].source_ref, "proc1");
                        assert_eq!(procs[1].source_ref, "proc2");
                        assert_eq!(procs[2].source_ref, "proc3");
                    }
                    other => panic!("Expected Par with 3 processes, got {:?}", other),
                }
                Ok(())
            }

            #[test]
            fn test_process_source_ref_preserved() -> Result<()> {
                let mut rspace = make_rspace();
                let channel = "@0:ref".to_string();

                let process = Process::new(
                    vec![Instruction::nullary(Opcode::HALT)],
                    "my_unique_ref_123",
                );

                rspace.tell(0, channel.clone(), Value::Par(vec![process]))?;

                match rspace.ask(0, channel)? {
                    Some(Value::Par(procs)) => {
                        assert_eq!(procs[0].source_ref, "my_unique_ref_123");
                    }
                    _ => panic!("Expected Par"),
                }
                Ok(())
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
        let channel = "@0:states".to_string();

        let ready_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "ready")
            .with_state(ProcessState::Ready);
        let wait_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "wait")
            .with_state(ProcessState::Wait);
        let value_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "value")
            .with_state(ProcessState::Value(Value::Int(99)));
        let error_proc = Process::new(vec![Instruction::nullary(Opcode::HALT)], "error")
            .with_state(ProcessState::Error("fail".to_string()));

        rspace.tell(
            0,
            channel.clone(),
            Value::Par(vec![ready_proc, wait_proc, value_proc, error_proc]),
        )?;

        match rspace.ask(0, channel)? {
            Some(Value::Par(procs)) => {
                assert_eq!(procs.len(), 4);
                assert!(matches!(procs[0].state, ProcessState::Ready));
                assert!(matches!(procs[1].state, ProcessState::Wait));
                assert!(matches!(procs[2].state, ProcessState::Value(_)));
                assert!(matches!(procs[3].state, ProcessState::Error(_)));
            }
            other => panic!("Expected Par, got {:?}", other),
        }
        Ok(())
    }
}

// =============================================================================
// Ready-Queue Drain Tests
// =============================================================================

mod drain_ready_tests {
    use super::*;

    #[test]
    fn test_drain_ready_processes_returns_only_ready() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        let channel = "@0:drain".to_string();

        let ready1 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "ready1");
        let ready2 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "ready2");
        let wait = Process::new(vec![Instruction::nullary(Opcode::HALT)], "wait")
            .with_state(ProcessState::Wait);

        rspace.tell(0, channel.clone(), Value::Par(vec![ready1, wait, ready2]))?;

        let ready = drain_ready_processes(rspace.as_mut(), 0, channel.clone())?;

        // Should return 2 ready processes
        assert_eq!(ready.len(), 2);
        assert!(ready.iter().all(|p| p.is_ready()));
        Ok(())
    }

    #[test]
    fn test_drain_restores_pending_processes() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        let channel = "@0:pending".to_string();

        let ready = Process::new(vec![Instruction::nullary(Opcode::HALT)], "ready");
        let wait = Process::new(vec![Instruction::nullary(Opcode::HALT)], "wait")
            .with_state(ProcessState::Wait);
        let terminal = Process::new(vec![Instruction::nullary(Opcode::HALT)], "terminal")
            .with_state(ProcessState::Value(Value::Int(1)));

        rspace.tell(0, channel.clone(), Value::Par(vec![ready, wait, terminal]))?;

        let _ = drain_ready_processes(rspace.as_mut(), 0, channel.clone())?;

        // Pending processes (wait and terminal) should be re-stored
        match rspace.ask(0, channel)? {
            Some(Value::Par(procs)) => {
                assert_eq!(procs.len(), 2);
                // Verify wait and terminal are stored back
                let has_wait = procs.iter().any(|p| matches!(p.state, ProcessState::Wait));
                let has_terminal = procs
                    .iter()
                    .any(|p| matches!(p.state, ProcessState::Value(_)));
                assert!(has_wait, "Wait process should be re-stored");
                assert!(has_terminal, "Terminal process should be re-stored");
            }
            other => panic!("Expected Par with pending processes, got {:?}", other),
        }
        Ok(())
    }

    #[test]
    fn test_drain_empty_channel() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        let channel = "@0:empty".to_string();

        let ready = drain_ready_processes(rspace.as_mut(), 0, channel)?;
        assert!(ready.is_empty());
        Ok(())
    }

    #[test]
    fn test_drain_non_par_value() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        let channel = "@0:nonpar".to_string();

        // Store a non-Par value
        rspace.tell(0, channel.clone(), Value::Int(42))?;

        let ready = drain_ready_processes(rspace.as_mut(), 0, channel.clone())?;

        // Should return empty (no processes)
        assert!(ready.is_empty());

        // Original value should be re-stored
        assert_eq!(rspace.ask(0, channel)?, Some(Value::Int(42)));
        Ok(())
    }

    #[test]
    fn test_drain_all_ready_empties_channel() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        let channel = "@0:allready".to_string();

        let p1 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "p1");
        let p2 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "p2");

        rspace.tell(0, channel.clone(), Value::Par(vec![p1, p2]))?;

        let ready = drain_ready_processes(rspace.as_mut(), 0, channel.clone())?;
        assert_eq!(ready.len(), 2);

        // Channel should be empty now (no pending to re-store)
        assert_eq!(rspace.ask(0, channel)?, None);
        Ok(())
    }

    #[test]
    fn test_drain_all_pending_returns_empty() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        let channel = "@0:allpending".to_string();

        let wait1 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "w1")
            .with_state(ProcessState::Wait);
        let wait2 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "w2")
            .with_state(ProcessState::Wait);

        rspace.tell(0, channel.clone(), Value::Par(vec![wait1, wait2]))?;

        let ready = drain_ready_processes(rspace.as_mut(), 0, channel.clone())?;
        assert!(ready.is_empty());

        // All pending should be re-stored
        match rspace.ask(0, channel)? {
            Some(Value::Par(procs)) => assert_eq!(procs.len(), 2),
            other => panic!("Expected Par, got {:?}", other),
        }
        Ok(())
    }

    #[test]
    fn test_drain_preserves_process_order() -> Result<()> {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        let channel = "@0:order".to_string();

        // Mix of ready and wait processes
        let r1 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "r1");
        let w1 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "w1")
            .with_state(ProcessState::Wait);
        let r2 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "r2");
        let w2 = Process::new(vec![Instruction::nullary(Opcode::HALT)], "w2")
            .with_state(ProcessState::Wait);

        rspace.tell(0, channel.clone(), Value::Par(vec![r1, w1, r2, w2]))?;

        let ready = drain_ready_processes(rspace.as_mut(), 0, channel.clone())?;

        // Ready processes should maintain relative order
        assert_eq!(ready.len(), 2);
        assert_eq!(ready[0].source_ref, "r1");
        assert_eq!(ready[1].source_ref, "r2");

        // Pending should also maintain order
        match rspace.ask(0, channel)? {
            Some(Value::Par(procs)) => {
                assert_eq!(procs.len(), 2);
                assert_eq!(procs[0].source_ref, "w1");
                assert_eq!(procs[1].source_ref, "w2");
            }
            other => panic!("Expected Par, got {:?}", other),
        }
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

        assert!(process.vm.is_none()); // No VM initially

        process.execute()?;

        // VM should be preserved after execution
        assert!(process.vm.is_some());
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

        let channel = "@0:concurrent".to_string();

        // Write from one "thread" context
        {
            let mut guard = rspace.lock().unwrap();
            guard.tell(0, channel.clone(), Value::Int(42))?;
        }

        // Read from another
        {
            let mut guard = rspace.lock().unwrap();
            assert_eq!(guard.ask(0, channel)?, Some(Value::Int(42)));
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
                    let channel = format!("@0:ch{}", i);
                    let mut guard = rspace_clone.lock().unwrap();
                    guard.tell(0, channel, Value::Int(i)).unwrap();
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // Verify all writes succeeded
        let mut guard = rspace.lock().unwrap();
        for i in 0..10 {
            let channel = format!("@0:ch{}", i);
            assert_eq!(guard.ask(0, channel)?, Some(Value::Int(i)));
        }

        Ok(())
    }

    #[test]
    fn test_concurrent_writes_to_same_channel() -> Result<()> {
        let rspace: Arc<Mutex<Box<dyn RSpace>>> =
            Arc::new(Mutex::new(Box::new(InMemoryRSpace::new())));
        let channel = "@0:shared".to_string();

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let rspace_clone = rspace.clone();
                let ch = channel.clone();
                thread::spawn(move || {
                    let mut guard = rspace_clone.lock().unwrap();
                    guard.tell(0, ch, Value::Int(i)).unwrap();
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        // All values should be stored
        let mut guard = rspace.lock().unwrap();
        let mut values = Vec::new();
        while let Some(val) = guard.ask(0, channel.clone())? {
            values.push(val);
        }
        assert_eq!(values.len(), 10);

        Ok(())
    }
}

// =============================================================================
// Error Case Tests
// =============================================================================

mod error_tests {
    use super::*;

    #[test]
    fn test_channel_kind_mismatch_error_message() {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        let channel = "@0:test".to_string();

        let err = rspace.tell(99, channel, Value::Int(1)).unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("channel-kind mismatch"));
        assert!(msg.contains("99"));
        assert!(msg.contains("@0:test"));
    }

    #[test]
    fn test_drain_with_kind_mismatch() {
        let mut rspace: Box<dyn RSpace> = Box::new(InMemoryRSpace::new());
        let channel = "@0:drain".to_string();

        // Try to drain with wrong kind
        let result = drain_ready_processes(rspace.as_mut(), 99, channel);
        assert!(result.is_err());
    }
}
