use anyhow::Result;

use crate::error::ExecError;
use crate::execute::{self, StepResult};
use crate::process::Process;
use crate::rspace::{InMemoryRSpace, RSpace};
use crate::value::Value;

pub struct VM {
    pub(crate) stack: Vec<Value>,
    // Abstract RSpace implementation
    pub(crate) rspace: Box<dyn RSpace>,
    // Simple continuation table: id -> stored value
    pub(crate) cont_table: std::collections::HashMap<u32, Value>,
    pub(crate) next_cont_id: u32,
    // Monotonic counter for fresh channel names
    pub(crate) next_name_id: u64,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            rspace: Box::new(InMemoryRSpace::new()),
            cont_table: std::collections::HashMap::new(),
            next_cont_id: 1,
            next_name_id: 1,
        }
    }

    pub fn with_rspace(rspace: Box<dyn RSpace>) -> Self {
        VM {
            stack: Vec::new(),
            rspace,
            cont_table: std::collections::HashMap::new(),
            next_cont_id: 1,
            next_name_id: 1,
        }
    }

    // Helper to clear in-VM RSpace store (useful for test isolation)
    pub fn reset_rspace(&mut self) {
        self.rspace.reset();
    }

    // Execute a provided Process (the only entry point)
    pub fn execute(&mut self, process: &mut Process) -> Result<Value> {
        // Reset VM stack per process execution to avoid contamination between runs
        self.stack.clear();
        let mut pc = 0usize;
        while pc < process.code.len() {
            let inst = process.code[pc];
            match execute::step(self, process, inst)? {
                StepResult::Next => {
                    pc += 1;
                }
                StepResult::Stop => {
                    break;
                }
                StepResult::Jump(label) => {
                    if let Some(&target) = process.labels.get(&label) {
                        pc = target;
                    } else {
                        // Label not found is an execution error
                        return Err(ExecError::LabelNotFound {
                            label,
                            source: process.source_ref.clone(),
                        }
                        .into());
                    }
                }
            }
        }
        Ok(self.stack.last().cloned().unwrap_or(Value::Nil))
    }
}
