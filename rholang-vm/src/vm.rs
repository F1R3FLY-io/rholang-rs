// Rholang Virtual Machine Implementation
// Based on the design in BYTECODE_DESIGN.md

use crate::bytecode::{Instruction, Label, Value};
use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;

/// VM Memory Layout segments as per BYTECODE_DESIGN.md
#[derive(Default, serde::Serialize, serde::Deserialize, Debug)]
pub struct VmMemory {
    /// Bytecode Segment: instructions and inline data
    pub bytecode_segment: Vec<Instruction>,
    /// Constant Pool: literals, patterns, predefined processes
    pub constant_pool: Vec<Value>,
    /// Process Heap: dynamic processes and closures (addressed by ID)
    pub process_heap: HashMap<u32, Value>,
    /// Continuation Table: suspended computations keyed by ID
    pub continuation_table: HashMap<u32, ContinuationRecord>,
    /// Pattern Cache: compiled pattern matchers
    pub pattern_cache: HashMap<String, PatternCompiled>,
    /// Name Registry: unforgeable names and quotes
    pub name_registry: HashMap<String, Value>,
}

/// Minimal placeholder for a compiled pattern representation
#[derive(Clone, Default, serde::Serialize, serde::Deserialize, Debug)]
pub struct PatternCompiled {
    /// textual key or descriptor; real impl would be bytecode or DFA
    pub key: String,
}

/// Minimal placeholder for a continuation record
#[derive(Clone, Default, serde::Serialize, serde::Deserialize, Debug)]
pub struct ContinuationRecord {
    /// textual reference to process/closure; real impl would be ProcessRef
    pub proc_ref: String,
    /// optional environment snapshot id (not implemented)
    pub env_id: Option<u32>,
}

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
    /// Cached RSpace instances by type for the duration of program execution
    pub rspaces: HashMap<crate::bytecode::RSpaceType, Box<dyn crate::rspace::RSpace>>,
    /// VM Memory Layout segments
    pub memory: VmMemory,
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
            rspaces: HashMap::new(),
            memory: VmMemory::default(),
        }
    }

    /// Pause execution and produce a canonical snapshot of the current state
    pub fn pause_and_snapshot(&self) -> crate::state::VmStateSnapshot {
        crate::state::snapshot_from_context(self)
    }

    /// Resume this execution context from a given snapshot
    pub fn resume_from_snapshot(&mut self, snapshot: &crate::state::VmStateSnapshot) -> anyhow::Result<()> {
        use std::collections::HashMap;
        use crate::bytecode::Label;
        use crate::rspace::RSpaceFactory;

        // Restore simple fields
        self.stack = snapshot.stack.clone();
        self.locals = snapshot.locals.clone();
        self.ip = snapshot.ip;

        // Restore labels map
        let mut labels: HashMap<Label, usize> = HashMap::new();
        for (name, idx) in &snapshot.labels {
            labels.insert(Label(name.clone()), *idx);
        }
        self.labels = labels;

        // Restore memory segments
        self.memory.constant_pool = snapshot.memory.constant_pool.clone();
        self.memory.process_heap = snapshot.memory.process_heap.iter().map(|(k, v)| (*k, v.clone())).collect();
        self.memory.continuation_table = snapshot.memory.continuation_table.iter().map(|(k, v)| (*k, v.clone())).collect();
        self.memory.pattern_cache = snapshot.memory.pattern_cache.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        self.memory.name_registry = snapshot.memory.name_registry.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // Recreate RSpaces listed in snapshot (empty instances; guest-visible state restore TBD)
        self.rspaces.clear();
        for rs_snap in &snapshot.rspaces {
            let rs = RSpaceFactory::create(rs_snap.rspace_type)?;
            self.rspaces.insert(rs_snap.rspace_type, rs);
        }

        Ok(())
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

    /// Get or create an RSpace instance for the given type
    pub fn get_or_create_rspace(&mut self, rspace_type: crate::bytecode::RSpaceType) -> Result<&mut Box<dyn crate::rspace::RSpace>> {
        if !self.rspaces.contains_key(&rspace_type) {
            let r = crate::rspace::RSpaceFactory::create(rspace_type)?;
            self.rspaces.insert(rspace_type, r);
        }
        Ok(self.rspaces.get_mut(&rspace_type).expect("rspace must exist"))
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

    /// Accessors for memory segments (minimal stubs for future use)
    pub fn constant_pool(&self) -> &Vec<Value> { &self.memory.constant_pool }
    pub fn constant_pool_mut(&mut self) -> &mut Vec<Value> { &mut self.memory.constant_pool }
    pub fn process_heap(&self) -> &HashMap<u32, Value> { &self.memory.process_heap }
    pub fn process_heap_mut(&mut self) -> &mut HashMap<u32, Value> { &mut self.memory.process_heap }
    pub fn continuation_table(&self) -> &HashMap<u32, ContinuationRecord> { &self.memory.continuation_table }
    pub fn continuation_table_mut(&mut self) -> &mut HashMap<u32, ContinuationRecord> { &mut self.memory.continuation_table }
    pub fn pattern_cache(&self) -> &HashMap<String, PatternCompiled> { &self.memory.pattern_cache }
    pub fn pattern_cache_mut(&mut self) -> &mut HashMap<String, PatternCompiled> { &mut self.memory.pattern_cache }
    pub fn name_registry(&self) -> &HashMap<String, Value> { &self.memory.name_registry }
    pub fn name_registry_mut(&mut self) -> &mut HashMap<String, Value> { &mut self.memory.name_registry }

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

    /// Pause given context and return snapshot (convenience wrapper)
    pub fn pause_and_snapshot(&self, context: &ExecutionContext) -> crate::state::VmStateSnapshot {
        context.pause_and_snapshot()
    }

    /// Create a new execution context from a snapshot (convenience wrapper)
    pub fn resume_from_snapshot(&self, snapshot: &crate::state::VmStateSnapshot) -> Result<ExecutionContext> {
        let mut ctx = ExecutionContext::new();
        ctx.resume_from_snapshot(snapshot)?;
        Ok(ctx)
    }

    /// Execute a bytecode program
    pub async fn execute(&self, program: &[Instruction]) -> Result<String> {
        let mut context = ExecutionContext::new();

        // Initialize memory layout: load bytecode segment
        context.memory.bytecode_segment = program.to_vec();

        // First pass: collect all labels from bytecode segment
        for (i, instruction) in context.memory.bytecode_segment.iter().enumerate() {
            if let Instruction::Label(label) = instruction {
                context.labels.insert(label.clone(), i);
            }
        }

        // Second pass: execute instructions from bytecode segment
        while context.ip < context.memory.bytecode_segment.len() {
            let instruction = context.memory.bytecode_segment[context.ip].clone();
            self.execute_instruction(&mut context, &instruction).await?;
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
                // BranchTrue: Pop Bool; if true, jump to label; else fall through
                context.branch_true(label)?;
            }
            Instruction::BranchFalse(label) => {
                // BranchFalse: Pop Bool; if false, jump to label; else fall through
                context.branch_false(label)?;
            }
            Instruction::BranchSuccess(_label) => {
                // BranchSuccess: Jump on previous operation success (semantics TBD)
                bail!("BranchSuccess not implemented yet");
            }
            Instruction::Jump(label) => {
                // Jump: Unconditional jump to label
                context.jump(label)?;
            }
            Instruction::CmpEq => {
                // CmpEq: Pop b, then a; push Bool(a == b)
                let b = context.pop()?;
                let a = context.pop()?;
                context.push(Value::Bool(a == b));
            }
            Instruction::CmpNeq => {
                // CmpNeq: Pop b, then a; push Bool(a != b)
                let b = context.pop()?;
                let a = context.pop()?;
                context.push(Value::Bool(a != b));
            }
            Instruction::CmpLt => {
                // CmpLt: Pop b, then a; require Ints; push Bool(a < b)
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
                // CmpLte: Pop b, then a; require Ints; push Bool(a <= b)
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
                // CmpGt: Pop b, then a; require Ints; push Bool(a > b)
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
                // CmpGte: Pop b, then a; require Ints; push Bool(a >= b)
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
                // Add: Pop b, then a; if Ints => a+b; if Strings => concat; push result
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
                // Sub: Pop b, then a; require Ints; push Int(a - b)
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
                // Mul: Pop b, then a; require Ints; push Int(a * b)
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
                // Div: Pop b, then a; require Ints; error on divide-by-zero; push Int(a / b)
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
                // Mod: Pop b, then a; require Ints; error on modulo-by-zero; push Int(a % b)
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
                // Neg: Pop Int a; push Int(-a)
                let a = context.pop()?;
                match a {
                    Value::Int(a) => {
                        context.push(Value::Int(-a));
                    }
                    _ => bail!("Neg requires integer operand"),
                }
            }
            Instruction::Not => {
                // Not: Pop Bool a; push Bool(!a)
                let a = context.pop()?;
                match a {
                    Value::Bool(a) => {
                        context.push(Value::Bool(!a));
                    }
                    _ => bail!("Not requires boolean operand"),
                }
            }
            Instruction::Concat => {
                // Concat: Pop b, then a; concat Strings or Lists; push result
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
                // Diff: Collection difference (semantics TBD)
                bail!("Diff not implemented yet");
            }
            Instruction::Interpolate => {
                // Interpolate: String interpolation (semantics TBD)
                bail!("Interpolate not implemented yet");
            }
            Instruction::CreateList(n) => {
                // CreateList(n): Pop n values (rightmost first), reverse to original order; push List
                let mut list = Vec::with_capacity(*n);
                for _ in 0..*n {
                    list.push(context.pop()?);
                }
                list.reverse(); // Reverse to maintain original order
                context.push(Value::List(list));
            }
            Instruction::CreateTuple(n) => {
                // CreateTuple(n): Pop n values, reverse to original order; push Tuple
                let mut tuple = Vec::with_capacity(*n);
                for _ in 0..*n {
                    tuple.push(context.pop()?);
                }
                tuple.reverse(); // Reverse to maintain original order
                context.push(Value::Tuple(tuple));
            }
            Instruction::CreateMap(n) => {
                // CreateMap(n): Pop n value/key pairs (value then key), reverse to original order; push Map
                let mut map = Vec::with_capacity(*n);
                for _ in 0..*n {
                    let value = context.pop()?;
                    let key = context.pop()?;
                    map.push((key, value));
                }
                map.reverse(); // Reverse to maintain original order
                context.push(Value::Map(map));
            }
            Instruction::InvokeMethod => {
                // InvokeMethod: Invoke a previously loaded method (object model TBD)
                bail!("InvokeMethod not implemented yet");
            }

            // Evaluation Instructions
            Instruction::Eval => {
                // Eval: Evaluate a process on stack in current env
                bail!("Eval not implemented yet");
            }
            Instruction::EvalBool => {
                // EvalBool: Evaluate and coerce to Bool
                bail!("EvalBool not implemented yet");
            }
            Instruction::EvalStar => {
                // EvalStar: Explicit evaluation (Rholang * semantics)
                bail!("EvalStar not implemented yet");
            }
            Instruction::EvalToRSpace => {
                // EvalToRSpace: Evaluate to a Value suitable for RSpace
                bail!("EvalToRSpace not implemented yet");
            }
            Instruction::EvalWithLocals => {
                // EvalWithLocals: Evaluate with provided local bindings
                bail!("EvalWithLocals not implemented yet");
            }
            Instruction::EvalInBundle => {
                // EvalInBundle: Evaluate within a bundle capability context
                bail!("EvalInBundle not implemented yet");
            }
            Instruction::Exec => {
                // Exec: Execute a process (fire-and-forget)
                bail!("Exec not implemented yet");
            }

            // Pattern Matching Instructions
            Instruction::Pattern(_pattern) => {
                // Pattern: Load/compile a pattern representation
                bail!("Pattern not implemented yet");
            }
            Instruction::MatchTest => {
                // MatchTest: Test pattern match, push Bool
                bail!("MatchTest not implemented yet");
            }
            Instruction::ExtractBindings => {
                // ExtractBindings: Extract bound variables from last match
                bail!("ExtractBindings not implemented yet");
            }

            // Process Control Instructions
            Instruction::SpawnAsync(_rspace_type) => {
                // SpawnAsync: Spawn a process asynchronously
                bail!("SpawnAsync not implemented yet");
            }
            Instruction::ProcNeg => {
                // ProcNeg: Process negation
                bail!("ProcNeg not implemented yet");
            }
            Instruction::Conj => {
                // Conj: Process conjunction (both must succeed)
                bail!("Conj not implemented yet");
            }
            Instruction::Disj => {
                // Disj: Process disjunction (either can succeed)
                bail!("Disj not implemented yet");
            }

            // Reference Instructions
            Instruction::Copy => {
                // Copy: Copy value (reference semantics TBD)
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
            Instruction::RSpaceProduce(rspace_type) => {
                // Stack: ... , channel(Name), data(Value)
                let data = context.pop()?;
                let channel_name = match context.pop()? {
                    Value::Name(s) => s,
                    other => bail!("RSpaceProduce expects a Name channel on stack, got {:?}", other),
                };
                let rspace = context.get_or_create_rspace(*rspace_type)?;
                let channel = crate::rspace::ChannelName { name: channel_name, rspace_type: *rspace_type };
                rspace.produce(channel, data).await?;
                // Indicate success
                context.push(Value::Bool(true));
            }
            Instruction::RSpaceConsume(rspace_type) => {
                // Blocking consume mapped to RSpace::get
                let channel_name = match context.pop()? {
                    Value::Name(s) => s,
                    other => bail!("RSpaceConsume expects a Name channel on stack, got {:?}", other),
                };
                let rspace = context.get_or_create_rspace(*rspace_type)?;
                let channel = crate::rspace::ChannelName { name: channel_name, rspace_type: *rspace_type };
                let value = rspace.get(channel).await?;
                context.push(value);
            }
            Instruction::RSpaceConsumeNonblock(rspace_type) => {
                // Non-blocking consume mapped to RSpace::get_nonblock
                let channel_name = match context.pop()? {
                    Value::Name(s) => s,
                    other => bail!("RSpaceConsumeNonblock expects a Name channel on stack, got {:?}", other),
                };
                let rspace = context.get_or_create_rspace(*rspace_type)?;
                let channel = crate::rspace::ChannelName { name: channel_name, rspace_type: *rspace_type };
                let value_opt = rspace.get_nonblock(channel).await?;
                match value_opt {
                    Some(v) => context.push(v),
                    None => context.push(Value::Nil),
                }
            }
            Instruction::RSpaceConsumePersistent(rspace_type) => {
                // Persistent consume behaves like peek (non-consuming)
                let channel_name = match context.pop()? {
                    Value::Name(s) => s,
                    other => bail!("RSpaceConsumePersistent expects a Name channel on stack, got {:?}", other),
                };
                let rspace = context.get_or_create_rspace(*rspace_type)?;
                let channel = crate::rspace::ChannelName { name: channel_name, rspace_type: *rspace_type };
                let value_opt = rspace.peek(channel).await?;
                match value_opt {
                    Some(v) => context.push(v),
                    None => context.push(Value::Nil),
                }
            }
            Instruction::RSpacePeek(rspace_type) => {
                // Peek at data without consuming
                let channel_name = match context.pop()? {
                    Value::Name(s) => s,
                    other => bail!("RSpacePeek expects a Name channel on stack, got {:?}", other),
                };
                let rspace = context.get_or_create_rspace(*rspace_type)?;
                let channel = crate::rspace::ChannelName { name: channel_name, rspace_type: *rspace_type };
                let value_opt = rspace.peek(channel).await?;
                match value_opt {
                    Some(v) => context.push(v),
                    None => context.push(Value::Nil),
                }
            }
            Instruction::RSpaceMatch(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceMatch not implemented yet");
            }
            Instruction::RSpaceSelectBegin(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceSelectBegin not implemented yet");
            }
            Instruction::RSpaceSelectAdd(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceSelectAdd not implemented yet");
            }
            Instruction::RSpaceSelectWait(_rspace_type) => {
                // This will be implemented when we have RSpace integration
                bail!("RSpaceSelectWait not implemented yet");
            }
            Instruction::NameCreate(rspace_type) => {
                // Create a corresponding RSpace and generate a fresh name
                // Use cached RSpace of the given type to create a fresh name
                let rspace = context.get_or_create_rspace(*rspace_type)?;
                let channel = rspace.name_create().await?;
                // Push the created name onto the stack as a Value::Name
                context.push(Value::Name(channel.name));
            }
            Instruction::NameQuote(rspace_type) => {
                // Pop a process and quote it to a name in the given RSpace
                let proc_str = match context.pop()? {
                    Value::Process(s) => s,
                    Value::String(s) => s,
                    other => bail!("NameQuote expects a Process or String on stack, got {:?}", other),
                };
                let rspace = context.get_or_create_rspace(*rspace_type)?;
                let name = rspace.name_quote(proc_str).await?;
                context.push(Value::Name(name.name));
            }
            Instruction::NameUnquote(rspace_type) => {
                // Pop a name and unquote it to a process string
                let channel_name = match context.pop()? {
                    Value::Name(s) => s,
                    other => bail!("NameUnquote expects a Name on stack, got {:?}", other),
                };
                let rspace = context.get_or_create_rspace(*rspace_type)?;
                let process = rspace
                    .name_unquote(crate::rspace::ChannelName { name: channel_name, rspace_type: *rspace_type })
                    .await?;
                context.push(Value::Process(process));
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
