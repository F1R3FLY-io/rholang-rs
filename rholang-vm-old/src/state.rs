use crate::bytecode::{RSpaceType, Value};
use crate::vm::{ContinuationRecord, ExecutionContext, VmMemory};
use std::collections::BTreeMap;

// Note: Core types don’t implement serde; we provide non-serialized snapshots for now.
// If/when serde is required, introduce local serializable wrappers.

/// Snapshot of an RSpace instance (guest-visible aspects only)
#[derive(Debug, Clone)]
pub struct RSpaceSnapshot {
    /// The RSpace backend type
    pub rspace_type: RSpaceType,
}

/// Canonical, serializable snapshot of VM memory
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub constant_pool: Vec<Value>,
    pub continuation_table: BTreeMap<u32, ContinuationRecord>,
    pub pattern_cache: BTreeMap<String, crate::vm::PatternCompiled>,
    pub name_registry: BTreeMap<String, Value>,
}

/// Canonical, serializable snapshot of the full VM state required for GSLT/JSON
#[derive(Debug, Clone)]
pub struct VmStateSnapshot {
    pub stack: Vec<Value>,
    pub locals: Vec<Value>,
    pub ip: usize,
    /// Map label name -> instruction index (canonical: sorted keys)
    pub labels: BTreeMap<String, usize>,
    /// Memory snapshot (canonical: sorted maps)
    pub memory: MemorySnapshot,
    /// RSpace snapshots present in this VM (guest-visible aspects only)
    pub rspaces: Vec<RSpaceSnapshot>,
}

impl From<&VmMemory> for MemorySnapshot {
    fn from(mem: &VmMemory) -> Self {
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
            continuation_table,
            pattern_cache,
            name_registry,
        }
    }
}

/// Build a canonical, serializable snapshot from a live execution context
pub fn snapshot_from_context(ctx: &ExecutionContext) -> VmStateSnapshot {
    // Labels and rspaces are not tracked in the simplified ExecutionContext anymore.
    let labels: BTreeMap<String, usize> = BTreeMap::new();
    let rspaces: Vec<RSpaceSnapshot> = Vec::new();

    VmStateSnapshot {
        stack: ctx.stack.clone(),
        locals: ctx.locals.clone(),
        ip: ctx.ip,
        labels,
        memory: MemorySnapshot::from(&ctx.memory),
        rspaces,
    }
}

