use anyhow::Result;
use std::collections::HashMap;

use crate::execute::{self, StepResult};
use crate::process::Process;
use crate::{ExecError, RSpace, Value};

#[derive(Clone)]
pub struct VM {
    pub stack: Vec<Value>,
    // Abstract RSpace implementation
    pub rspace: std::sync::Arc<std::sync::Mutex<Box<dyn RSpace>>>,
    // Single-slot continuation storage (id, value)
    pub(crate) cont_last: Option<(u32, Value)>,
    pub(crate) next_cont_id: u32,
    // Monotonic counter for fresh channel names
    pub(crate) next_name_id: u64,
}

#[derive(Default)]
struct DefaultRSpace {
    store: HashMap<(u16, String), Vec<Value>>,
}

impl DefaultRSpace {
    fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }
}

impl RSpace for DefaultRSpace {
    fn tell(&mut self, kind: u16, channel: String, data: Value) -> Result<()> {
        ensure_kind_matches_channel(kind, &channel)?;
        let key = (kind, channel);
        self.store.entry(key).or_default().push(data);
        Ok(())
    }

    fn ask(&mut self, kind: u16, channel: String) -> Result<Option<Value>> {
        ensure_kind_matches_channel(kind, &channel)?;
        let key = (kind, channel);
        Ok(self.store.get_mut(&key).and_then(|q| {
            if q.is_empty() {
                None
            } else {
                Some(q.remove(0))
            }
        }))
    }

    fn peek(&self, kind: u16, channel: String) -> Result<Option<Value>> {
        ensure_kind_matches_channel(kind, &channel)?;
        let key = (kind, channel);
        Ok(self.store.get(&key).and_then(|q| q.first()).cloned())
    }

    fn reset(&mut self) {
        self.store.clear();
    }
}

fn ensure_kind_matches_channel(kind: u16, channel: &str) -> anyhow::Result<()> {
    if !channel.starts_with(&format!("@{}:", kind)) {
        anyhow::bail!(
            "channel-kind mismatch: kind {} does not match channel {}",
            kind,
            channel
        );
    }
    Ok(())
}

impl std::fmt::Debug for VM {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VM")
            .field("stack", &self.stack)
            .field("cont_last", &self.cont_last)
            .field("next_cont_id", &self.next_cont_id)
            .field("next_name_id", &self.next_name_id)
            .finish()
    }
}

impl PartialEq for VM {
    fn eq(&self, other: &Self) -> bool {
        self.stack == other.stack
            && self.cont_last == other.cont_last
            && self.next_cont_id == other.next_cont_id
            && self.next_name_id == other.next_name_id
        // We skip RSpace for equality as it's a shared resource
    }
}

impl Eq for VM {}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            rspace: std::sync::Arc::new(std::sync::Mutex::new(Box::new(DefaultRSpace::new()))),
            cont_last: None,
            next_cont_id: 1,
            next_name_id: 1,
        }
    }

    pub fn with_rspace(rspace: Box<dyn RSpace>) -> Self {
        VM {
            stack: Vec::new(),
            rspace: std::sync::Arc::new(std::sync::Mutex::new(rspace)),
            cont_last: None,
            next_cont_id: 1,
            next_name_id: 1,
        }
    }

    // Helper to clear in-VM RSpace store (useful for test isolation)
    pub fn reset_rspace(&mut self) {
        if let Ok(mut rspace) = self.rspace.lock() {
            rspace.reset();
        }
    }

    // Execute a provided Process (the only entry point)
    pub fn execute(&mut self, process: &mut Process) -> Result<Value, ExecError> {
        // Reset VM stack per process execution to avoid contamination between runs
        self.stack.clear();
        let mut pc = 0usize;
        let code = process.code.clone(); // Clone code to avoid borrowing process as whole
        while pc < code.len() {
            let inst = code[pc];
            match execute::step(self, process, inst)? {
                StepResult::Next => {
                    pc += 1;
                }
                StepResult::Stop => {
                    break;
                }
                StepResult::Jump(target) => {
                    pc = target;
                }
            }
        }
        Ok(self.stack.last().cloned().unwrap_or(Value::Nil))
    }
}
