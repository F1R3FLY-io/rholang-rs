use crate::parameter::Parameter;
use rholang_bytecode::core::instructions::Instruction as CoreInst;
use rholang_rspace::{ExecError, ProcessHolder, ProcessState, Value};
use rholang_vm::{StepResult, VM};
use std::any::Any;
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct Process {
    pub code: Vec<CoreInst>,
    pub source_ref: String,
    pub locals: Vec<Value>,
    pub names: Vec<Value>,
    pub vm: VM,
    pub state: ProcessState,
    /// Named parameter bindings that must be solved before execution
    pub parameters: Vec<Parameter>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessEvent {
    Value(String),
    Error(String),
}

pub type ProcessEventHandler = Arc<dyn Fn(ProcessEvent) + Send + Sync>;

impl Process {
    pub fn new<S: Into<String>>(code: Vec<CoreInst>, source_ref: S) -> Self {
        Self {
            code,
            source_ref: source_ref.into(),
            locals: Vec::new(),
            names: Vec::new(),
            vm: VM::new(),
            state: ProcessState::Ready,
            parameters: Vec::new(),
        }
    }

    /// Create a new Process with a custom VM (e.g., with shared RSpace)
    pub fn with_vm<S: Into<String>>(code: Vec<CoreInst>, source_ref: S, vm: VM) -> Self {
        Self {
            code,
            source_ref: source_ref.into(),
            locals: Vec::new(),
            names: Vec::new(),
            vm,
            state: ProcessState::Ready,
            parameters: Vec::new(),
        }
    }

    pub fn with_state(mut self, state: ProcessState) -> Self {
        self.state = state;
        self
    }

    /// Set the parameters for this process.
    ///
    /// Parameters are named bindings that must be solved (have resolved values in RSpace)
    /// before the process can execute.
    pub fn with_parameters(mut self, parameters: Vec<Parameter>) -> Self {
        self.parameters = parameters;
        self
    }

    /// Get the parameters for this process.
    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    /// Check if all parameters are solved.
    ///
    /// A process with no parameters always returns true.
    /// A process with parameters returns true only if all parameters are solved.
    fn all_parameters_solved(&self) -> bool {
        if self.parameters.is_empty() {
            return true;
        }
        // Lock the RSpace and check all parameters
        let rspace = self.vm.rspace.lock().unwrap();
        self.parameters.iter().all(|p| p.is_solved(rspace.as_ref()))
    }

    /// Box this process into a ProcessHolder trait object
    pub fn boxed(self) -> Box<dyn ProcessHolder> {
        Box::new(self)
    }

    /// Evaluate a value from EVAL opcode.
    /// For Par values: execute ready processes and return list of results.
    /// For other values: return them as-is (already evaluated).
    fn evaluate_value(target: Value) -> Result<Value, ExecError> {
        match target {
            Value::Par(mut procs) => {
                let mut results = Vec::new();
                for proc in procs.iter_mut() {
                    if proc.is_ready() {
                        let result = proc.execute()?;
                        results.push(result);
                    }
                }
                // If only one result, return it directly; otherwise return list
                if results.len() == 1 {
                    Ok(results.pop().unwrap())
                } else {
                    Ok(Value::List(results))
                }
            }
            // Non-Par values are already evaluated, just pass through
            other => Ok(other),
        }
    }

    pub fn execute(&mut self) -> Result<Value, ExecError> {
        self.execute_with_event(None)
    }

    pub fn execute_with_event(
        &mut self,
        handler: Option<&ProcessEventHandler>,
    ) -> Result<Value, ExecError> {
        // Terminal states (value, error) must not be re-executed (rspace.md rule)
        match &self.state {
            ProcessState::Value(val) => return Ok(val.clone()),
            ProcessState::Error(msg) => {
                return Err(ExecError::OpcodeParamError {
                    opcode: "EXECUTE",
                    message: format!("cannot re-execute process in error state: {}", msg),
                })
            }
            ProcessState::Wait => {
                return Err(ExecError::OpcodeParamError {
                    opcode: "EXECUTE",
                    message: "cannot execute process in wait state".to_string(),
                })
            }
            ProcessState::Ready => {} // OK to execute
        }

        // Check that all parameters are solved before executing
        if !self.all_parameters_solved() {
            return Err(ExecError::OpcodeParamError {
                opcode: "EXECUTE",
                message: "cannot execute process with unsolved parameters".to_string(),
            });
        }

        self.vm.reset_stack();

        let mut pc = 0usize;
        let code = self.code.clone();
        let result = loop {
            if pc >= code.len() {
                break Ok(self.vm.stack.last().cloned().unwrap_or(Value::Nil));
            }

            let inst = code[pc];
            match self.vm.execute(&mut self.locals, &self.names, inst) {
                Ok(StepResult::Next) => {
                    pc += 1;
                }
                Ok(StepResult::Stop) => {
                    break Ok(self.vm.stack.last().cloned().unwrap_or(Value::Nil));
                }
                Ok(StepResult::Jump(target)) => {
                    pc = target;
                }
                Ok(StepResult::Eval(target)) => {
                    // Handle EVAL: execute Par values or pass through others
                    let eval_result = Self::evaluate_value(target)?;
                    self.vm.stack.push(eval_result);
                    pc += 1;
                }
                Err(err) => {
                    break Err(err);
                }
            }
        };

        match result {
            Ok(val) => {
                self.state = ProcessState::Value(val.clone());
                if let Some(handler) = handler {
                    handler(ProcessEvent::Value(self.source_ref.clone()));
                }
                Ok(val)
            }
            Err(err) => {
                self.state = ProcessState::Error(err.to_string());
                if let Some(handler) = handler {
                    handler(ProcessEvent::Error(self.source_ref.clone()));
                }
                Err(err)
            }
        }
    }
}

// Implement ProcessHolder trait for Process
impl ProcessHolder for Process {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn ProcessHolder> {
        Box::new(self.clone())
    }

    fn eq_box(&self, other: &dyn ProcessHolder) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<Process>() {
            self == other
        } else {
            false
        }
    }

    fn is_ready(&self) -> bool {
        // Process must be in Ready state AND all parameters must be solved
        matches!(self.state, ProcessState::Ready) && self.all_parameters_solved()
    }

    fn execute(&mut self) -> Result<Value, ExecError> {
        Process::execute(self)
    }

    fn source_ref(&self) -> &str {
        &self.source_ref
    }

    fn state(&self) -> &ProcessState {
        &self.state
    }
}

impl fmt::Display for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Header
        writeln!(f, "=== Process: {} ===", self.source_ref)?;

        // Parameters section
        if !self.parameters.is_empty() {
            writeln!(f, "\n[Parameters] ({} entries)", self.parameters.len())?;
            for (idx, param) in self.parameters.iter().enumerate() {
                writeln!(f, "  [{}]: {}", idx, param.name())?;
            }
        }

        // String pool section
        if !self.names.is_empty() {
            writeln!(f, "\n[String Pool] ({} entries)", self.names.len())?;
            for (idx, name) in self.names.iter().enumerate() {
                writeln!(f, "  [{}]: {:?}", idx, name)?;
            }
        }

        // Bytecode section
        writeln!(f, "\n[Bytecode] ({} instructions)", self.code.len())?;
        for (idx, inst) in self.code.iter().enumerate() {
            writeln!(f, "  {:04}: {:?}", idx, inst)?;
        }

        Ok(())
    }
}
