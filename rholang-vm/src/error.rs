use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ExecError {
    LabelNotFound { label: String, source: String },
    OpcodeParamError { opcode: &'static str, message: String },
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecError::LabelNotFound { label, source } => {
                write!(f, "label not found: '{}' in {}", label, source)
            }
            ExecError::OpcodeParamError { opcode, message } => {
                write!(f, "{} parameter error: {}", opcode, message)
            }
        }
    }
}

impl Error for ExecError {}
