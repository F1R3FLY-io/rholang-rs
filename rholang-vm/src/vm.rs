use anyhow::Result;
use rholang_bytecode::core::instructions::Instruction as CoreInst;

use crate::execute::{self, StepResult};
use crate::process::Process;
use crate::value::Value;
use crate::error::ExecError;

pub struct VM {
    pub(crate) stack: Vec<Value>,
    // Simple in-VM RSpace storage: (kind_code, channel_name) -> queue of values
    pub(crate) rspace: std::collections::HashMap<(u16, String), Vec<Value>>,
    // Simple continuation table: id -> stored value
    pub(crate) cont_table: std::collections::HashMap<u32, Value>,
    pub(crate) next_cont_id: u32,
    // Monotonic counter for fresh channel names
    pub(crate) next_name_id: u64,
}

impl VM {
    pub fn new() -> Self { VM { stack: Vec::new(), rspace: std::collections::HashMap::new(), cont_table: std::collections::HashMap::new(), next_cont_id: 1, next_name_id: 1 } }

    // Helper to clear in-VM RSpace store (useful for test isolation)
    pub fn reset_rspace(&mut self) { self.rspace.clear(); }

    // Execute a provided Process (the only entry point)
    pub fn execute(&mut self, process: &mut Process) -> Result<Value> {
        // Reset VM stack per process execution to avoid contamination between runs
        self.stack.clear();
        let mut pc = 0usize;
        while pc < process.code.len() {
            let inst = process.code[pc].clone();
            match execute::step(self, process, inst)? {
                StepResult::Next => { pc += 1; }
                StepResult::Stop => { break; }
                StepResult::Jump(label) => {
                    if let Some(&target) = process.labels.get(&label) {
                        pc = target;
                    } else {
                        // Label not found is an execution error
                        return Err(ExecError::LabelNotFound { label, source: process.source_ref.clone() }.into());
                    }
                }
            }
        }
        Ok(self.stack.last().cloned().unwrap_or(Value::Nil))
    }
}
