use crate::{ExecError, Value, VM};
use rholang_bytecode::core::instructions::Instruction as CoreInst;
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct Process {
    pub code: Vec<CoreInst>,
    pub source_ref: String,
    pub locals: Vec<Value>,
    pub names: Vec<Value>,
    pub vm: Option<VM>,
    pub state: ProcessState,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProcessState {
    Wait,
    Ready,
    Value(Value),
    Error(String),
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
            vm: None,
            state: ProcessState::Ready,
        }
    }

    pub fn with_state(mut self, state: ProcessState) -> Self {
        self.state = state;
        self
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.state, ProcessState::Ready)
    }

    pub fn execute(&mut self) -> Result<Value, ExecError> {
        self.execute_with_event(None)
    }

    pub fn execute_with_event(
        &mut self,
        handler: Option<&ProcessEventHandler>,
    ) -> Result<Value, ExecError> {
        let mut vm = self.vm.take().unwrap_or_default();
        let result = vm.execute(self);
        self.vm = Some(vm);

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

impl fmt::Display for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Header
        writeln!(f, "=== Process: {} ===", self.source_ref)?;

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
