use crate::value::Value;
use rholang_bytecode::core::instructions::Instruction as CoreInst;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Process {
    pub code: Vec<CoreInst>,
    pub source_ref: String,
    pub locals: Vec<Value>,
    pub names: Vec<Value>,
    // Map of label name -> program counter (index in code)
    pub labels: HashMap<String, usize>,
}

impl Process {
    pub fn new<S: Into<String>>(code: Vec<CoreInst>, source_ref: S) -> Self {
        Self {
            code,
            source_ref: source_ref.into(),
            locals: Vec::new(),
            names: Vec::new(),
            labels: HashMap::new(),
        }
    }

    /// Replace labels map with provided entries
    pub fn set_labels<I, K>(&mut self, entries: I)
    where
        I: IntoIterator<Item = (K, usize)>,
        K: Into<String>,
    {
        self.labels.clear();
        for (k, v) in entries {
            self.labels.insert(k.into(), v);
        }
    }
}
