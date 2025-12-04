use crate::value::Value;
use rholang_bytecode::core::instructions::Instruction as CoreInst;
use std::fmt;

#[derive(Clone, Debug)]
pub struct Process {
    pub code: Vec<CoreInst>,
    pub source_ref: String,
    pub locals: Vec<Value>,
    pub names: Vec<Value>,
}

impl Process {
    pub fn new<S: Into<String>>(code: Vec<CoreInst>, source_ref: S) -> Self {
        Self {
            code,
            source_ref: source_ref.into(),
            locals: Vec::new(),
            names: Vec::new(),
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
