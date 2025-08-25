use crate::bytecode::{Label, RSpaceType, Value};
use crate::rspace::RSpaceSnapshotProvider;
use crate::vm::{ContinuationRecord, ExecutionContext, VmMemory};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

/// Snapshot of an RSpace instance (guest-visible aspects only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSpaceSnapshot {
    /// The RSpace backend type
    pub rspace_type: RSpaceType,
    /// Placeholder for future: channels and data; represented canonically
    #[serde(default)]
    pub channels: BTreeMap<String, JsonValue>,
}

/// Canonical, serializable snapshot of VM memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub constant_pool: Vec<Value>,
    pub process_heap: BTreeMap<u32, Value>,
    pub continuation_table: BTreeMap<u32, ContinuationRecord>,
    pub pattern_cache: BTreeMap<String, crate::vm::PatternCompiled>,
    pub name_registry: BTreeMap<String, Value>,
}

/// Canonical, serializable snapshot of the full VM state required for GSLT/JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStateSnapshot {
    pub stack: Vec<Value>,
    pub locals: Vec<Value>,
    pub ip: usize,
    /// Map label name -> instruction index (canonical: sorted keys)
    pub labels: BTreeMap<String, usize>,
    /// Memory snapshot (canonical: sorted maps)
    pub memory: MemorySnapshot,
    /// RSpace snapshots present in this VM (guest-visible aspects only)
    #[serde(default)]
    pub rspaces: Vec<RSpaceSnapshot>,
}

impl From<&VmMemory> for MemorySnapshot {
    fn from(mem: &VmMemory) -> Self {
        let mut process_heap = BTreeMap::new();
        for (k, v) in &mem.process_heap {
            process_heap.insert(*k, v.clone());
        }
        let mut continuation_table = BTreeMap::new();
        for (k, v) in &mem.continuation_table {
            continuation_table.insert(*k, v.clone());
        }
        let mut pattern_cache = BTreeMap::new();
        for (k, v) in &mem.pattern_cache {
            pattern_cache.insert(k.clone(), v.clone());
        }
        let mut name_registry = BTreeMap::new();
        for (k, v) in &mem.name_registry {
            name_registry.insert(k.clone(), v.clone());
        }
        MemorySnapshot {
            constant_pool: mem.constant_pool.clone(),
            process_heap,
            continuation_table,
            pattern_cache,
            name_registry,
        }
    }
}

/// Build a canonical, serializable snapshot from a live execution context
pub fn snapshot_from_context(ctx: &ExecutionContext) -> VmStateSnapshot {
    let mut labels = BTreeMap::new();
    for (label, idx) in &ctx.labels {
        labels.insert(label.0.clone(), *idx);
    }

    // RSpace snapshots: we currently include only the type; further details
    // can be added by extracting guest-visible channel/data state from concrete impls.
    let mut rspaces: Vec<RSpaceSnapshot> = ctx
        .rspaces
        .iter()
        .map(|(t, rs)| {
            // Attempt to obtain channel snapshots via RSpaceSnapshotProvider if implemented
            let channels = {
                // downcast to known concrete types
                let any = rs.as_any();
                if let Some(mem_seq) = any.downcast_ref::<crate::rspace::MemorySequentialRSpace>() {
                    mem_seq.snapshot_channels()
                } else if let Some(mem_conc) = any.downcast_ref::<crate::rspace::MemoryConcurrentRSpace>() {
                    mem_conc.snapshot_channels()
                } else {
                    BTreeMap::new()
                }
            };
            RSpaceSnapshot {
                rspace_type: *t,
                channels,
            }
        })
        .collect();
    // Keep deterministic order by sorting by rspace_type discriminant name
    rspaces.sort_by_key(|r| format!("{}", r.rspace_type));

    VmStateSnapshot {
        stack: ctx.stack.clone(),
        locals: ctx.locals.clone(),
        ip: ctx.ip,
        labels,
        memory: MemorySnapshot::from(&ctx.memory),
        rspaces,
    }
}

/// Serialize the VM state to a canonical JSON string (sorted maps ensure canonical key order)
pub fn serialize_state_to_json(ctx: &ExecutionContext) -> Result<String> {
    let snapshot = snapshot_from_context(ctx);
    let json = serde_json::to_string(&snapshot)?;
    Ok(json)
}

static VM_STATE_SCHEMA_STR: &str = include_str!("../vm_state_schema.json");

/// Two-stage deserialization and validation:
/// 1) Parse JSON syntactically
/// 2) Validate against vm_state_schema.json
pub fn deserialize_state_from_json(json_str: &str) -> Result<VmStateSnapshot> {
    // Stage 1: syntactic parse
    let value: JsonValue = serde_json::from_str(json_str)?;

    // Stage 2: schema validation
    let schema_json: JsonValue = serde_json::from_str(VM_STATE_SCHEMA_STR)?;
    let compiled = jsonschema::JSONSchema::compile(&schema_json)
        .map_err(|e| anyhow!("Schema compile error: {e}"))?;
    if let Err(errors) = compiled.validate(&value) {
        let mut msg = String::from("JSON does not conform to vm_state_schema.json:\n");
        for err in errors {
            msg.push_str(&format!("- {err}\n"));
        }
        return Err(anyhow!(msg));
    }

    // If valid, deserialize into snapshot
    let snapshot: VmStateSnapshot = serde_json::from_value(value)?;
    Ok(snapshot)
}
