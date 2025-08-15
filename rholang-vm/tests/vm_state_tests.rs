use rholang_vm::bytecode::{Label, Value};
use rholang_vm::state::{deserialize_state_from_json, serialize_state_to_json, snapshot_from_context};
use rholang_vm::vm::{ContinuationRecord, ExecutionContext, PatternCompiled};

fn make_ctx_variant(order_variant: u8) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();

    // Stack and locals
    ctx.stack.push(Value::Int(1));
    ctx.stack.push(Value::String("x".into()));
    ctx.locals.push(Value::Bool(true));
    ctx.locals.push(Value::Nil);

    // IP and labels
    ctx.ip = 7;
    match order_variant {
        0 => {
            ctx.labels.insert(Label("start".into()), 0);
            ctx.labels.insert(Label("loop".into()), 3);
        }
        _ => {
            // Insert in different order to test canonicalization
            ctx.labels.insert(Label("loop".into()), 3);
            ctx.labels.insert(Label("start".into()), 0);
        }
    }

    // Memory: constant pool, process heap, continuation table, pattern cache, name registry
    ctx.memory.constant_pool.push(Value::Int(42));

    match order_variant {
        0 => {
            ctx.memory.process_heap.insert(2, Value::String("proc2".into()));
            ctx.memory.process_heap.insert(1, Value::String("proc1".into()));
            ctx
                .memory
                .continuation_table
                .insert(10, ContinuationRecord { proc_ref: "k10".into(), env_id: Some(1) });
            ctx
                .memory
                .continuation_table
                .insert(5, ContinuationRecord { proc_ref: "k5".into(), env_id: None });
            ctx
                .memory
                .pattern_cache
                .insert("pX".into(), PatternCompiled { key: "patX".into() });
            ctx
                .memory
                .pattern_cache
                .insert("aA".into(), PatternCompiled { key: "patA".into() });
            ctx
                .memory
                .name_registry
                .insert("alpha".into(), Value::Name("@0".into()));
            ctx
                .memory
                .name_registry
                .insert("zeta".into(), Value::Name("@1".into()));
        }
        _ => {
            // Insert in opposite order
            ctx
                .memory
                .name_registry
                .insert("zeta".into(), Value::Name("@1".into()));
            ctx
                .memory
                .name_registry
                .insert("alpha".into(), Value::Name("@0".into()));
            ctx
                .memory
                .pattern_cache
                .insert("aA".into(), PatternCompiled { key: "patA".into() });
            ctx
                .memory
                .pattern_cache
                .insert("pX".into(), PatternCompiled { key: "patX".into() });
            ctx
                .memory
                .continuation_table
                .insert(5, ContinuationRecord { proc_ref: "k5".into(), env_id: None });
            ctx
                .memory
                .continuation_table
                .insert(10, ContinuationRecord { proc_ref: "k10".into(), env_id: Some(1) });
            ctx.memory.process_heap.insert(1, Value::String("proc1".into()));
            ctx.memory.process_heap.insert(2, Value::String("proc2".into()));
        }
    }

    ctx
}

#[test]
fn test_snapshot_round_trip_and_canonicalization() {
    let ctx_a = make_ctx_variant(0);
    let ctx_b = make_ctx_variant(1);

    // JSON should be identical due to canonical ordering
    let json_a = serialize_state_to_json(&ctx_a).expect("serialize A");
    let json_b = serialize_state_to_json(&ctx_b).expect("serialize B");
    assert_eq!(json_a, json_b, "Canonical JSON must be identical across insertion orders");

    // Round-trip via schema-validated deserialization
    let snap = deserialize_state_from_json(&json_a).expect("deserialize via schema");

    // Sanity checks on snapshot content
    assert_eq!(snap.ip, 7);
    assert_eq!(snap.stack.len(), 2);
    assert_eq!(snap.locals.len(), 2);
    assert_eq!(snap.labels.get("start"), Some(&0));
    assert_eq!(snap.labels.get("loop"), Some(&3));
    assert_eq!(snap.memory.constant_pool, vec![Value::Int(42)]);
    assert_eq!(snap.memory.process_heap.len(), 2);
    assert_eq!(snap.memory.continuation_table.len(), 2);
    assert_eq!(snap.memory.pattern_cache.len(), 2);
    assert_eq!(snap.memory.name_registry.len(), 2);

    // Ensure snapshot_from_context yields equivalent JSON when re-serialized
    let json_from_snap = serde_json::to_string(&snap).expect("serialize snapshot");
    assert_eq!(json_from_snap, json_a);

    // Also compare direct snapshot against snapshot_from_context
    let snap_direct = snapshot_from_context(&ctx_a);
    let json_direct = serde_json::to_string(&snap_direct).unwrap();
    assert_eq!(json_direct, json_a);
}

#[test]
fn test_schema_validation_rejects_invalid() {
    // Missing required field "ip"
    let invalid = r#"{
        "stack": [],
        "locals": [],
        "labels": {},
        "memory": {
            "constant_pool": [],
            "process_heap": {},
            "continuation_table": {},
            "pattern_cache": {},
            "name_registry": {}
        },
        "rspaces": []
    }"#;

    let err = deserialize_state_from_json(invalid).expect_err("schema should reject missing ip");
    let msg = err.to_string();
    assert!(msg.contains("vm_state_schema.json") || msg.contains("does not conform"));

    // Bad Value encoding (using {"Int": "not int"})
    let invalid_value = r#"{
        "stack": [{"Int": "NaN"}],
        "locals": [],
        "ip": 0,
        "labels": {},
        "memory": {
            "constant_pool": [],
            "process_heap": {},
            "continuation_table": {},
            "pattern_cache": {},
            "name_registry": {}
        },
        "rspaces": []
    }"#;

    let err2 = deserialize_state_from_json(invalid_value).expect_err("schema should reject wrong Value shape");
    let msg2 = err2.to_string();
    assert!(msg2.contains("vm_state_schema.json") || msg2.contains("does not conform"));
}

use tokio::runtime::Runtime;
use rholang_vm::rspace::{RSpaceFactory, RSpace, ChannelName, Pattern, Continuation};

#[test]
fn test_rspace_snapshot_provider_populates_channels() {
    let mut ctx = rholang_vm::vm::ExecutionContext::new();
    let rt = Runtime::new().expect("tokio rt");

    // Create a MemorySequential RSpace and attach to context
    let mut rs = RSpaceFactory::create(rholang_vm::bytecode::RSpaceType::MemorySequential).expect("factory");

    // Prepare channels
    let ch1 = ChannelName { name: "ch1".into(), rspace_type: rholang_vm::bytecode::RSpaceType::MemorySequential };
    let other = ChannelName { name: "other".into(), rspace_type: rholang_vm::bytecode::RSpaceType::MemorySequential };

    // Produce some data to ch1
    rt.block_on(async {
        rs.produce(ch1.clone(), Value::Int(1)).await.unwrap();
        rs.produce(ch1.clone(), Value::Int(2)).await.unwrap();
        // Store a continuation on a different channel by consuming with no data present
        let pat = Pattern { pattern: "x".into(), bindings: vec!["x".into()] };
        let kont = Continuation { process: "P".into(), environment: std::collections::HashMap::new() };
        let res = rs.consume(other.clone(), pat, kont).await.unwrap();
        assert!(res.is_none());
    });

    // Insert into ctx
    ctx.rspaces.insert(rholang_vm::bytecode::RSpaceType::MemorySequential, rs);

    // Snapshot
    let snap = snapshot_from_context(&ctx);
    // There should be one rspace
    assert_eq!(snap.rspaces.len(), 1);
    let channels = &snap.rspaces[0].channels;
    // ch1 should have data [1,2]
    let ch1_entry = channels.get("ch1").expect("ch1 entry present");
    // Continuations for ch1 should be empty
    let data = ch1_entry.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    assert_eq!(data.len(), 2);
    // other should have a continuation
    let other_entry = channels.get("other").expect("other entry present");
    let conts = other_entry.get("continuations").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    assert_eq!(conts.len(), 1);
}