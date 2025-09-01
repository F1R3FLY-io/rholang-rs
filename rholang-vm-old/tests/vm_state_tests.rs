use rholang_vm_old::bytecode::{Label, Value};
use rholang_vm_old::state::snapshot_from_context;
use rholang_vm_old::vm::{ContinuationRecord, ExecutionContext, PatternCompiled};

fn build_ctx(order_variant: u8) -> ExecutionContext {
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
            ctx.labels.insert(Label("loop".into()), 3);
            ctx.labels.insert(Label("start".into()), 0);
        }
    }

    // Memory (without process_heap): constant_pool, continuation_table, pattern_cache, name_registry
    ctx.memory.constant_pool.push(Value::Int(42));

    match order_variant {
        0 => {
            ctx.memory
                .continuation_table
                .insert(10, ContinuationRecord { proc_ref: "k10".into(), env_id: Some(1) });
            ctx.memory
                .continuation_table
                .insert(5, ContinuationRecord { proc_ref: "k5".into(), env_id: None });
            ctx.memory
                .pattern_cache
                .insert("pX".into(), PatternCompiled { key: "patX".into() });
            ctx.memory
                .pattern_cache
                .insert("aA".into(), PatternCompiled { key: "patA".into() });
            ctx.memory
                .name_registry
                .insert("alpha".into(), Value::Name("@0".into()));
            ctx.memory
                .name_registry
                .insert("zeta".into(), Value::Name("@1".into()));
        }
        _ => {
            ctx.memory
                .name_registry
                .insert("zeta".into(), Value::Name("@1".into()));
            ctx.memory
                .name_registry
                .insert("alpha".into(), Value::Name("@0".into()));
            ctx.memory
                .pattern_cache
                .insert("aA".into(), PatternCompiled { key: "patA".into() });
            ctx.memory
                .pattern_cache
                .insert("pX".into(), PatternCompiled { key: "patX".into() });
            ctx.memory
                .continuation_table
                .insert(5, ContinuationRecord { proc_ref: "k5".into(), env_id: None });
            ctx.memory
                .continuation_table
                .insert(10, ContinuationRecord { proc_ref: "k10".into(), env_id: Some(1) });
        }
    }

    ctx
}

#[test]
fn test_snapshot_basic_and_canonical() {
    let ctx_a = build_ctx(0);
    let ctx_b = build_ctx(1);

    let snap_a = snapshot_from_context(&ctx_a);
    let snap_b = snapshot_from_context(&ctx_b);

    // Basic assertions
    assert_eq!(snap_a.ip, 7);
    assert_eq!(snap_a.stack.len(), 2);
    assert_eq!(snap_a.locals.len(), 2);
    assert_eq!(snap_a.memory.constant_pool, vec![Value::Int(42)]);
    assert_eq!(snap_a.memory.continuation_table.len(), 2);
    assert_eq!(snap_a.memory.pattern_cache.len(), 2);
    assert_eq!(snap_a.memory.name_registry.len(), 2);

    // Canonical labels stored in BTreeMap -> order-independent
    assert_eq!(snap_a.labels.get("start"), Some(&0));
    assert_eq!(snap_a.labels.get("loop"), Some(&3));
    assert_eq!(snap_b.labels.get("start"), Some(&0));
    assert_eq!(snap_b.labels.get("loop"), Some(&3));

    // RSpaces snapshot is currently empty in this simplified VM snapshot
    assert!(snap_a.rspaces.is_empty());
}