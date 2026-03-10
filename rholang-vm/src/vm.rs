//! VM structure and execution context.

use anyhow::Result;
use rholang_bytecode::core::instructions::Instruction as CoreInst;
use std::sync::{Arc, Mutex};

use crate::execute::{self, StepResult};
use rholang_rspace::{ExecError, InMemoryRSpace, RSpace, SharedRSpace, Value};

/// Virtual Machine for Rholang bytecode execution.
///
/// The VM maintains:
/// - A value stack for operand storage
/// - A shared RSpace for tuple space operations
/// - Continuation state for async operations
/// - A name counter for fresh channel generation
#[derive(Clone)]
pub struct VM {
    /// Value stack for operand storage during execution.
    pub stack: Vec<Value>,
    /// Shared RSpace implementation for tuple space operations.
    pub rspace: SharedRSpace,
    /// Single-slot continuation storage (id, value).
    pub(crate) cont_last: Option<(u32, Value)>,
    /// Counter for generating unique continuation IDs.
    pub(crate) next_cont_id: u32,
    /// Monotonic counter for generating fresh channel names.
    pub(crate) next_name_id: u64,
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
    /// Create a new VM with default InMemoryRSpace.
    ///
    /// For production use with hierarchical channel names, use
    /// `VM::with_rspace()` with a PathMapRSpace instance.
    pub fn new() -> Self {
        VM {
            stack: Vec::new(),
            rspace: Arc::new(Mutex::new(Box::new(InMemoryRSpace::new()))),
            cont_last: None,
            next_cont_id: 1,
            next_name_id: 1,
        }
    }

    /// Create a VM with a custom RSpace implementation.
    ///
    /// The RSpace will be wrapped in Arc<Mutex<>> for concurrent access.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rholang_rspace::PathMapRSpace;
    /// use rholang_vm::VM;
    ///
    /// let vm = VM::with_rspace(Box::new(PathMapRSpace::new()));
    /// ```
    pub fn with_rspace(rspace: Box<dyn RSpace>) -> Self {
        VM {
            stack: Vec::new(),
            rspace: Arc::new(Mutex::new(rspace)),
            cont_last: None,
            next_cont_id: 1,
            next_name_id: 1,
        }
    }

    /// Create a VM with a pre-shared RSpace.
    ///
    /// Use this when you need to share RSpace access outside the VM,
    /// such as when multiple processes share the same tuple space.
    pub fn with_shared_rspace(rspace: SharedRSpace) -> Self {
        VM {
            stack: Vec::new(),
            rspace,
            cont_last: None,
            next_cont_id: 1,
            next_name_id: 1,
        }
    }

    /// Clear the RSpace store (useful for test isolation).
    pub fn reset_rspace(&mut self) {
        if let Ok(mut rspace) = self.rspace.lock() {
            rspace.reset();
        }
    }

    /// Clear the value stack.
    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    /// Execute a single instruction.
    ///
    /// # Arguments
    ///
    /// * `locals` - Process local variable slots
    /// * `names` - Process string pool for PUSH_STR
    /// * `inst` - The instruction to execute
    ///
    /// # Returns
    ///
    /// `StepResult` indicating whether execution should continue or stop.
    pub fn execute(
        &mut self,
        locals: &mut Vec<Value>,
        names: &[Value],
        inst: CoreInst,
    ) -> Result<StepResult, ExecError> {
        execute::step(self, locals, names, inst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rholang_rspace::ProcessState;

    // =========================================================================
    // VM tests
    // =========================================================================

    #[test]
    fn test_vm_new_creates_empty_rspace() {
        let vm = VM::new();
        assert!(vm.stack.is_empty());
        assert!(vm.cont_last.is_none());
        assert_eq!(vm.next_cont_id, 1);
        assert_eq!(vm.next_name_id, 1);
    }

    #[test]
    fn test_vm_with_custom_rspace() -> Result<()> {
        let mut custom_rspace = InMemoryRSpace::new();
        custom_rspace.tell("pre_loaded", Value::Int(42))?;

        let vm = VM::with_rspace(Box::new(custom_rspace));

        let rspace = vm.rspace.lock().unwrap();
        assert_eq!(rspace.peek("pre_loaded")?, Some(Value::Int(42)));

        Ok(())
    }

    #[test]
    fn test_vm_reset_rspace() -> Result<()> {
        let mut vm = VM::new();

        {
            let mut rspace = vm.rspace.lock().unwrap();
            rspace.tell("test", Value::Int(1))?;
        }

        vm.reset_rspace();

        {
            let rspace = vm.rspace.lock().unwrap();
            assert!(rspace.get_entry("test").is_none());
        }

        Ok(())
    }

    #[test]
    fn test_vm_reset_stack() {
        let mut vm = VM::new();
        vm.stack.push(Value::Int(1));
        vm.stack.push(Value::Int(2));

        vm.reset_stack();

        assert!(vm.stack.is_empty());
    }

    #[test]
    fn test_vm_rspace_operations() -> Result<()> {
        let vm = VM::new();

        {
            let mut rspace = vm.rspace.lock().unwrap();

            // Channel operations
            rspace.tell("inbox", Value::Int(42))?;
            assert_eq!(rspace.peek("inbox")?, Some(Value::Int(42)));

            // Process operations
            rspace.register_process("worker", ProcessState::Ready)?;
            assert_eq!(
                rspace.get_process_state("worker"),
                Some(ProcessState::Ready)
            );

            // Value operations
            rspace.set_value("config", Value::Str("test".into()))?;
            assert_eq!(rspace.get_value("config"), Some(Value::Str("test".into())));
        }

        Ok(())
    }
}
