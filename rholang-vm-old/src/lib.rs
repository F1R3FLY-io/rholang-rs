// Rholang Virtual Machine Implementation
// Based on the design in BYTECODE_DESIGN.md

pub mod bytecode;
pub mod compiler;
pub mod interpreter;
pub mod rspace;
pub mod vm;
pub mod state;

use anyhow::Result;

/// The main Rholang VM interface
pub struct RholangVM {
    /// The VM instance
    vm: vm::VM,
}

impl RholangVM {
    /// Create a new Rholang VM instance
    pub fn new() -> Result<Self> {
        Ok(RholangVM { vm: vm::VM::new()? })
    }

    /// Execute bytecode in the VM
    pub async fn execute(&self, bytecode: &[bytecode::Instruction]) -> Result<String> {
        self.vm.execute(bytecode).await
    }

    /// Compile Rholang code to bytecode
    pub fn compile(&self, code: &str) -> Result<Vec<bytecode::Instruction>> {
        let mut compiler = compiler::RholangCompiler::new();
        compiler.compile(code)
    }

    /// Compile and execute Rholang code
    pub async fn compile_and_execute(&self, code: &str) -> Result<String> {
        let bytecode = self.compile(code)?;
        self.execute(&bytecode).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_vm_creation() -> Result<()> {
        let _vm = RholangVM::new()?;
        Ok(())
    }
}
