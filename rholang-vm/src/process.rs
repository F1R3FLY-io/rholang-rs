use crate::value::Value;
use rholang_bytecode::core::instructions::Instruction as CoreInst;

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
