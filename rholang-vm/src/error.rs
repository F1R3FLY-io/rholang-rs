use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ExecError {
    OpcodeParamError {
        opcode: &'static str,
        message: String,
    },
}

impl fmt::Display for ExecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecError::OpcodeParamError { opcode, message } => {
                write!(f, "{} parameter error: {}", opcode, message)
            }
        }
    }
}

impl Error for ExecError {}
