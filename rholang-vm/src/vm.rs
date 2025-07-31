// Rholang Virtual Machine Implementation
// Based on the design in BYTECODE_DESIGN.md

use crate::bytecode::{Instruction, Label, Value};
use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;

/// The VM execution context
pub struct ExecutionContext {
    /// The stack for computational operations
    pub stack: Vec<Value>,
    /// Local variables
    pub locals: Vec<Value>,
    /// Current instruction pointer
    pub ip: usize,
    /// Label to instruction index mapping
    pub labels: HashMap<Label, usize>,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new() -> Self {
        ExecutionContext {
            stack: Vec::new(),
            locals: Vec::new(),
            ip: 0,
            labels: HashMap::new(),
        }
    }

    /// Push a value onto the stack
    pub fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    /// Pop a value from the stack
    pub fn pop(&mut self) -> Result<Value> {
        self.stack.pop().ok_or_else(|| anyhow!("Stack underflow"))
    }

    /// Peek at the top value on the stack without removing it
    pub fn peek(&self) -> Result<&Value> {
        self.stack.last().ok_or_else(|| anyhow!("Stack empty"))
    }

    /// Allocate a new local variable
    pub fn alloc_local(&mut self) -> usize {
        let index = self.locals.len();
        self.locals.push(Value::Nil);
        index
    }

    /// Load a local variable
    pub fn load_local(&self, index: usize) -> Result<Value> {
        self.locals
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!("Local variable index out of bounds: {}", index))
    }

    /// Store a value to a local variable
    pub fn store_local(&mut self, index: usize, value: Value) -> Result<()> {
        if index >= self.locals.len() {
            bail!("Local variable index out of bounds: {}", index);
        }
        self.locals[index] = value;
        Ok(())
    }

    /// Jump to a label
    pub fn jump(&mut self, label: &Label) -> Result<()> {
        let target = self
            .labels
            .get(label)
            .ok_or_else(|| anyhow!("Label not found: {:?}", label))?;
        self.ip = *target;
        Ok(())
    }

    /// Jump to a label if the top of the stack is true
    pub fn branch_true(&mut self, label: &Label) -> Result<()> {
        let condition = self.pop()?;
        match condition {
            Value::Bool(true) => self.jump(label),
            Value::Bool(false) => {
                // Just continue to the next instruction
                Ok(())
            }
            _ => bail!("Expected boolean value for branch_true"),
        }
    }

    /// Jump to a label if the top of the stack is false
    pub fn branch_false(&mut self, label: &Label) -> Result<()> {
        let condition = self.pop()?;
        match condition {
            Value::Bool(false) => self.jump(label),
            Value::Bool(true) => {
                // Just continue to the next instruction
                Ok(())
            }
            _ => bail!("Expected boolean value for branch_false"),
        }
    }
}

/// The Rholang Virtual Machine
pub struct VM {
    // VM is currently stateless, but this struct is kept for future extensions
}

impl VM {
    /// Create a new VM instance
    pub fn new() -> Result<Self> {
        Ok(VM {})
    }

    /// Execute a bytecode program
    pub async fn execute(&self, program: &[Instruction]) -> Result<String> {
        let mut context = ExecutionContext::new();

        // First pass: collect all labels
        for (i, instruction) in program.iter().enumerate() {
            if let Instruction::Label(label) = instruction {
                context.labels.insert(label.clone(), i);
            }
        }

        // Second pass: execute instructions
        while context.ip < program.len() {
            let instruction = &program[context.ip];
            self.execute_instruction(&mut context, instruction).await?;
            context.ip += 1;
        }

        // Return the top of the stack as the result
        match context.pop() {
            Ok(value) => Ok(format!("{value:?}")),
            Err(_) => Ok("(no result)".to_string()),
        }
    }

    /// Execute a single instruction
    async fn execute_instruction(
        &self,
        context: &mut ExecutionContext,
        instruction: &Instruction,
    ) -> Result<()> {
        match instruction {
            // Computational Instructions
            Instruction::Nop => {
                // No operation
            }
            Instruction::PushInt(n) => {
                context.push(Value::Int(*n));
            }
            Instruction::PushStr(s) => {
                context.push(Value::String(s.clone()));
            }
            Instruction::PushBool(b) => {
                context.push(Value::Bool(*b));
            }
            Instruction::PushProc(proc) => {
                context.push(Value::Process(proc.clone()));
            }
            Instruction::Pop => {
                context.pop()?;
            }
            Instruction::Dup => {
                let value = context.peek()?.clone();
                context.push(value);
            }
            Instruction::LoadVar(_index) => {
                // This will be implemented when we have variable binding
                bail!("LoadVar not implemented yet");
            }
            Instruction::LoadLocal(index) => {
                let value = context.load_local(*index)?;
                context.push(value);
            }
            Instruction::StoreLocal(index) => {
                let value = context.pop()?;
                context.store_local(*index, value)?;
            }
            Instruction::AllocLocal => {
                let _index = context.alloc_local();
            }
            Instruction::BranchTrue(label) => {
                context.branch_true(label)?;
            }
            Instruction::BranchFalse(label) => {
                context.branch_false(label)?;
            }
            Instruction::BranchSuccess(_label) => {
                // This will be implemented when we have success/failure semantics
                bail!("BranchSuccess not implemented yet");
            }
            Instruction::Jump(label) => {
                context.jump(label)?;
            }
            Instruction::CmpEq => {
                let b = context.pop()?;
                let a = context.pop()?;
                context.push(Value::Bool(a == b));
            }
            Instruction::CmpNeq => {
                let b = context.pop()?;
                let a = context.pop()?;
                context.push(Value::Bool(a != b));
            }
            Instruction::CmpLt => {
                let b = context.pop()?;
                let a = context.pop()?;
                match (a, b) {
                    (Value::Int(a), Value::Int(b)) => {
                        context.push(Value::Bool(a < b));
                    }
                    _ => bail!("CmpLt requires integer operands"),
                }
            }
            Instruction::CmpLte => {
                let b = context.pop()?;
                let a = context.pop()?;
                match (a, b) {
                    (Value::Int(a), Value::Int(b)) => {
                        context.push(Value::Bool(a <= b));
                    }
                    _ => bail!("CmpLte requires integer operands"),
                }
            }
            Instruction::CmpGt => {
                let b = context.pop()?;
                let a = context.pop()?;
                match (a, b) {
                    (Value::Int(a), Value::Int(b)) => {
                        context.push(Value::Bool(a > b));
                    }
                    _ => bail!("CmpGt requires integer operands"),
                }
            }
            Instruction::CmpGte => {
                let b = context.pop()?;
                let a = context.pop()?;
                match (a, b) {
                    (Value::Int(a), Value::Int(b)) => {
                        context.push(Value::Bool(a >= b));
                    }
                    _ => bail!("CmpGte requires integer operands"),
                }
            }
            Instruction::Add => {
                let b = context.pop()?;
                let a = context.pop()?;
                match (a, b) {
                    (Value::Int(a), Value::Int(b)) => {
                        context.push(Value::Int(a + b));
                    }
                    (Value::String(a), Value::String(b)) => {
                        context.push(Value::String(a + &b));
                    }
                    _ => bail!("Add requires integer or string operands"),
                }
            }
            Instruction::Sub => {
                let b = context.pop()?;
                let a = context.pop()?;
                match (a, b) {
                    (Value::Int(a), Value::Int(b)) => {
                        context.push(Value::Int(a - b));
                    }
                    _ => bail!("Sub requires integer operands"),
                }
            }
            Instruction::Mul => {
                let b = context.pop()?;
                let a = context.pop()?;
                match (a, b) {
                    (Value::Int(a), Value::Int(b)) => {
                        context.push(Value::Int(a * b));
                    }
                    _ => bail!("Mul requires integer operands"),
                }
            }
            Instruction::Div => {
                let b = context.pop()?;
                let a = context.pop()?;
                match (a, b) {
                    (Value::Int(a), Value::Int(b)) => {
                        if b == 0 {
                            bail!("Division by zero");
                        }
                        context.push(Value::Int(a / b));
                    }
                    _ => bail!("Div requires integer operands"),
                }
            }
            Instruction::Mod => {
                let b = context.pop()?;
                let a = context.pop()?;
                match (a, b) {
                    (Value::Int(a), Value::Int(b)) => {
                        if b == 0 {
                            bail!("Modulo by zero");
                        }
                        context.push(Value::Int(a % b));
                    }
                    _ => bail!("Mod requires integer operands"),
                }
            }
            Instruction::Neg => {
                let a = context.pop()?;
                match a {
                    Value::Int(a) => {
                        context.push(Value::Int(-a));
                    }
                    _ => bail!("Neg requires integer operand"),
                }
            }
            Instruction::Not => {
                let a = context.pop()?;
                match a {
                    Value::Bool(a) => {
                        context.push(Value::Bool(!a));
                    }
                    _ => bail!("Not requires boolean operand"),
                }
            }
            Instruction::Concat => {
                let b = context.pop()?;
                let a = context.pop()?;
                match (a, b) {
                    (Value::String(a), Value::String(b)) => {
                        context.push(Value::String(a + &b));
                    }
                    (Value::List(a), Value::List(b)) => {
                        let mut result = a;
                        result.extend(b);
                        context.push(Value::List(result));
                    }
                    _ => bail!("Concat requires string or list operands"),
                }
            }
            Instruction::Diff => {
                // This will be implemented when we have collection difference semantics
                bail!("Diff not implemented yet");
            }
            Instruction::Interpolate => {
                // This will be implemented when we have string interpolation semantics
                bail!("Interpolate not implemented yet");
            }
            Instruction::CreateList(n) => {
                let mut list = Vec::with_capacity(*n);
                for _ in 0..*n {
                    list.push(context.pop()?);
                }
                list.reverse(); // Reverse to maintain original order
                context.push(Value::List(list));
            }
            Instruction::CreateTuple(n) => {
                let mut tuple = Vec::with_capacity(*n);
                for _ in 0..*n {
                    tuple.push(context.pop()?);
                }
                tuple.reverse(); // Reverse to maintain original order
                context.push(Value::Tuple(tuple));
            }
            Instruction::InvokeMethod => {
                // This will be implemented when we have method invocation semantics
                bail!("InvokeMethod not implemented yet");
            }

            // Evaluation Instructions
            Instruction::Eval => {
                // This will be implemented when we have process evaluation semantics
                bail!("Eval not implemented yet");
            }
            Instruction::EvalBool => {
                // This will be implemented when we have process evaluation semantics
                bail!("EvalBool not implemented yet");
            }
            Instruction::EvalToRSpace => {
                // This will be implemented when we have RSpace integration
                bail!("EvalToRSpace not implemented yet");
            }
            Instruction::EvalWithLocals => {
                // This will be implemented when we have process evaluation semantics
                bail!("EvalWithLocals not implemented yet");
            }
            Instruction::EvalInBundle => {
                // This will be implemented when we have bundle semantics
                bail!("EvalInBundle not implemented yet");
            }
            Instruction::Exec => {
                // This will be implemented when we have process execution semantics
                bail!("Exec not implemented yet");
            }

            // Pattern Matching Instructions
            Instruction::Pattern(_pattern) => {
                // This will be implemented when we have pattern matching semantics
                bail!("Pattern not implemented yet");
            }
            Instruction::MatchTest => {
                // This will be implemented when we have pattern matching semantics
                bail!("MatchTest not implemented yet");
            }
            Instruction::ExtractBindings => {
                // This will be implemented when we have pattern matching semantics
                bail!("ExtractBindings not implemented yet");
            }

            // Data Structure Instructions
            Instruction::MapBegin => {
                // This will be implemented when we have map construction semantics
                bail!("MapBegin not implemented yet");
            }
            Instruction::MapPut => {
                // This will be implemented when we have map construction semantics
                bail!("MapPut not implemented yet");
            }
            Instruction::MapEnd => {
                // This will be implemented when we have map construction semantics
                bail!("MapEnd not implemented yet");
            }

            // Process Control Instructions
            Instruction::SpawnAsync => {
                // This will be implemented when we have process spawning semantics
                bail!("SpawnAsync not implemented yet");
            }
            Instruction::ProcNeg => {
                // This will be implemented when we have process negation semantics
                bail!("ProcNeg not implemented yet");
            }

            // Reference Instructions
            Instruction::Copy => {
                // This will be implemented when we have reference semantics
                bail!("Copy not implemented yet");
            }
            Instruction::Move => {
                // This will be implemented when we have reference semantics
                bail!("Move not implemented yet");
            }
            Instruction::Ref => {
                // This will be implemented when we have reference semantics
                bail!("Ref not implemented yet");
            }
            Instruction::LoadMethod(_name) => {
                // This will be implemented when we have method invocation semantics
                bail!("LoadMethod not implemented yet");
            }

            // RSpace Instructions
            Instruction::RSpacePut(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpacePut not implemented yet");
            }
            Instruction::RSpaceGet(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceGet not implemented yet");
            }
            Instruction::RSpaceGetNonblock(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceGetNonblock not implemented yet");
            }
            Instruction::RSpaceConsume(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceConsume not implemented yet");
            }
            Instruction::RSpaceProduce(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceProduce not implemented yet");
            }
            Instruction::RSpacePeek(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpacePeek not implemented yet");
            }
            Instruction::RSpaceMatch(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceMatch not implemented yet");
            }
            Instruction::RSpaceSelect(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceSelect not implemented yet");
            }
            Instruction::NameCreate(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("NameCreate not implemented yet");
            }
            Instruction::NameQuote(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("NameQuote not implemented yet");
            }
            Instruction::NameUnquote(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("NameUnquote not implemented yet");
            }
            Instruction::PatternCompile(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("PatternCompile not implemented yet");
            }
            Instruction::PatternBind(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("PatternBind not implemented yet");
            }
            Instruction::ContinuationStore(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("ContinuationStore not implemented yet");
            }
            Instruction::ContinuationResume(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("ContinuationResume not implemented yet");
            }
            Instruction::RSpaceBundleBegin(_rspace_type, _bundle_op) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceBundleBegin not implemented yet");
            }
            Instruction::RSpaceBundleEnd(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceBundleEnd not implemented yet");
            }

            // Label definition (for jumps)
            Instruction::Label(_label) => {
                // Labels are processed in the first pass, so we can skip them here
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_arithmetic() -> Result<()> {
        let vm = VM::new()?;
        let program = vec![
            Instruction::PushInt(2),
            Instruction::PushInt(3),
            Instruction::Add,
        ];
        let result = vm.execute(&program).await?;
        assert_eq!(result, "Int(5)");
        Ok(())
    }

    #[tokio::test]
    async fn test_comparison() -> Result<()> {
        let vm = VM::new()?;
        let program = vec![
            Instruction::PushInt(2),
            Instruction::PushInt(3),
            Instruction::CmpLt,
        ];
        let result = vm.execute(&program).await?;
        assert_eq!(result, "Bool(true)");
        Ok(())
    }

    #[tokio::test]
    async fn test_local_variables() -> Result<()> {
        let vm = VM::new()?;
        let program = vec![
            Instruction::AllocLocal,    // Allocate local 0
            Instruction::PushInt(42),   // Push 42
            Instruction::StoreLocal(0), // Store 42 in local 0
            Instruction::AllocLocal,    // Allocate local 1
            Instruction::PushInt(7),    // Push 7
            Instruction::StoreLocal(1), // Store 7 in local 1
            Instruction::LoadLocal(0),  // Load local 0 (42)
            Instruction::LoadLocal(1),  // Load local 1 (7)
            Instruction::Add,           // Add them
        ];
        let result = vm.execute(&program).await?;
        assert_eq!(result, "Int(49)");
        Ok(())
    }

    #[tokio::test]
    async fn test_jumps() -> Result<()> {
        let vm = VM::new()?;
        let program = vec![
            Instruction::PushInt(1),
            Instruction::PushInt(2),
            Instruction::Jump(Label("skip".to_string())),
            Instruction::Add, // This should be skipped
            Instruction::Label(Label("skip".to_string())),
            Instruction::Mul,
        ];
        let result = vm.execute(&program).await?;
        assert_eq!(result, "Int(2)");
        Ok(())
    }

    #[tokio::test]
    async fn test_conditional_jumps() -> Result<()> {
        let vm = VM::new()?;
        let program = vec![
            Instruction::PushInt(1),
            Instruction::PushInt(2),
            Instruction::PushBool(true),
            Instruction::BranchTrue(Label("true_branch".to_string())),
            // False branch (should be skipped)
            Instruction::Add,
            Instruction::Jump(Label("end".to_string())),
            // True branch
            Instruction::Label(Label("true_branch".to_string())),
            Instruction::Mul,
            Instruction::Label(Label("end".to_string())),
        ];
        let result = vm.execute(&program).await?;
        assert_eq!(result, "Int(2)");
        Ok(())
    }
}
