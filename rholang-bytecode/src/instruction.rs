//! Instruction set for Rholang bytecode.
//!
//! This module defines the instruction set for the Rholang virtual machine.
//! Instructions are organized into categories based on their functionality.

use crate::types::{Constant, Literal};
use serde::{Deserialize, Serialize};
use std::fmt;

/// An instruction in the Rholang bytecode.
///
/// Instructions are the basic building blocks of Rholang bytecode programs.
/// They manipulate the stack, perform operations, and control program flow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    // Stack Operations
    /// Push a constant value onto the stack
    Push(Constant),
    /// Remove the top value from the stack
    Pop,
    /// Duplicate the top value on the stack
    Dup,
    /// Swap the top two values on the stack
    Swap,
    /// Rotate the top three values on the stack
    Rot,

    // Arithmetic Instructions
    /// Add the top two values on the stack
    Add,
    /// Subtract the top value from the second value on the stack
    Sub,
    /// Multiply the top two values on the stack
    Mul,
    /// Divide the second value by the top value on the stack
    Div,
    /// Compute the remainder of dividing the second value by the top value
    Mod,

    // Logical Instructions
    /// Logical AND of the top two values on the stack
    And,
    /// Logical OR of the top two values on the stack
    Or,
    /// Logical NOT of the top value on the stack
    Not,
    /// Check if the top two values on the stack are equal
    Eq,
    /// Check if the top two values on the stack are not equal
    Ne,
    /// Check if the second value is less than the top value
    Lt,
    /// Check if the second value is less than or equal to the top value
    Le,
    /// Check if the second value is greater than the top value
    Gt,
    /// Check if the second value is greater than or equal to the top value
    Ge,

    // Process Instructions
    /// Parallel composition of processes
    Par,
    /// Send a message on a channel
    Send {
        /// Number of arguments to send
        arity: usize,
    },
    /// Receive a message from a channel
    Receive {
        /// Number of patterns to match
        arity: usize,
        /// Whether this is a persistent receive (replication)
        persistent: bool,
    },
    /// Create a new name
    New {
        /// Number of new names to create
        count: usize,
    },

    // Control Flow Instructions
    /// Unconditional jump to a target address
    Jump {
        /// Target address to jump to
        target: usize,
    },
    /// Conditional jump if the top of the stack is true
    JumpIf {
        /// Target address to jump to if condition is true
        target: usize,
    },
    /// Conditional jump if the top of the stack is false
    JumpIfNot {
        /// Target address to jump to if condition is false
        target: usize,
    },
    /// Call a function at the target address
    Call {
        /// Target address of the function to call
        target: usize,
    },
    /// Return from a function call
    Return,
    /// Call a built-in function
    CallBuiltin {
        /// Name of the built-in function to call
        name: String,
        /// Number of arguments for the built-in function
        arity: usize,
    },
    /// Start a pattern match operation
    Match,
    /// Define a case for a pattern match
    MatchCase {
        /// Target address to jump to if the pattern matches
        target: usize,
    },

    // Memory Instructions
    /// Load a value from global storage
    Load {
        /// Index of the value to load
        index: usize,
    },
    /// Store a value to global storage
    Store {
        /// Index where the value should be stored
        index: usize,
    },
    /// Load a value from the local environment
    LoadLocal {
        /// Index of the value to load
        index: usize,
    },
    /// Store a value to the local environment
    StoreLocal {
        /// Index where the value should be stored
        index: usize,
    },
    /// Push a new environment frame
    PushEnv,
    /// Pop the current environment frame
    PopEnv,

    // Data Structure Instructions
    /// Create a new empty list
    ListNew,
    /// Push a value onto a list
    ListPush,
    /// Pop a value from a list
    ListPop,
    /// Get a value from a list at a specific index
    ListGet,
    /// Create a new empty map
    MapNew,
    /// Insert a key-value pair into a map
    MapInsert,
    /// Get a value from a map by key
    MapGet,
    /// Remove a key-value pair from a map
    MapRemove,
    /// Create a new tuple with a specific size
    TupleNew {
        /// Size of the tuple to create
        size: usize,
    },
    /// Get a value from a tuple at a specific index
    TupleGet {
        /// Index of the value to get
        index: usize,
    },

    // Built-in Instructions
    /// Concatenate two strings
    StringConcat,
    /// Get the length of a string
    StringLength,
    /// Get a slice of a string
    StringSlice,

    // Quoting Instructions
    /// Quote a process to create a name
    Quote,
    /// Unquote a name to get the original process
    Unquote,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Stack Operations
            Instruction::Push(constant) => write!(f, "PUSH {}", constant),
            Instruction::Pop => write!(f, "POP"),
            Instruction::Dup => write!(f, "DUP"),
            Instruction::Swap => write!(f, "SWAP"),
            Instruction::Rot => write!(f, "ROT"),

            // Arithmetic Instructions
            Instruction::Add => write!(f, "ADD"),
            Instruction::Sub => write!(f, "SUB"),
            Instruction::Mul => write!(f, "MUL"),
            Instruction::Div => write!(f, "DIV"),
            Instruction::Mod => write!(f, "MOD"),

            // Logical Instructions
            Instruction::And => write!(f, "AND"),
            Instruction::Or => write!(f, "OR"),
            Instruction::Not => write!(f, "NOT"),
            Instruction::Eq => write!(f, "EQ"),
            Instruction::Ne => write!(f, "NE"),
            Instruction::Lt => write!(f, "LT"),
            Instruction::Le => write!(f, "LE"),
            Instruction::Gt => write!(f, "GT"),
            Instruction::Ge => write!(f, "GE"),

            // Process Instructions
            Instruction::Par => write!(f, "PAR"),
            Instruction::Send { arity } => write!(f, "SEND {}", arity),
            Instruction::Receive { arity, persistent } => {
                write!(
                    f,
                    "RECEIVE {} {}",
                    arity,
                    if *persistent { "PERSISTENT" } else { "ONCE" }
                )
            }
            Instruction::New { count } => write!(f, "NEW {}", count),

            // Control Flow Instructions
            Instruction::Jump { target } => write!(f, "JUMP {}", target),
            Instruction::JumpIf { target } => write!(f, "JUMPIF {}", target),
            Instruction::JumpIfNot { target } => write!(f, "JUMPIFNOT {}", target),
            Instruction::Call { target } => write!(f, "CALL {}", target),
            Instruction::Return => write!(f, "RETURN"),
            Instruction::CallBuiltin { name, arity } => write!(f, "CALLBUILTIN {} {}", name, arity),
            Instruction::Match => write!(f, "MATCH"),
            Instruction::MatchCase { target } => write!(f, "MATCHCASE {}", target),

            // Memory Instructions
            Instruction::Load { index } => write!(f, "LOAD {}", index),
            Instruction::Store { index } => write!(f, "STORE {}", index),
            Instruction::LoadLocal { index } => write!(f, "LOADLOCAL {}", index),
            Instruction::StoreLocal { index } => write!(f, "STORELOCAL {}", index),
            Instruction::PushEnv => write!(f, "PUSHENV"),
            Instruction::PopEnv => write!(f, "POPENV"),

            // Data Structure Instructions
            Instruction::ListNew => write!(f, "LISTNEW"),
            Instruction::ListPush => write!(f, "LISTPUSH"),
            Instruction::ListPop => write!(f, "LISTPOP"),
            Instruction::ListGet => write!(f, "LISTGET"),
            Instruction::MapNew => write!(f, "MAPNEW"),
            Instruction::MapInsert => write!(f, "MAPINSERT"),
            Instruction::MapGet => write!(f, "MAPGET"),
            Instruction::MapRemove => write!(f, "MAPREMOVE"),
            Instruction::TupleNew { size } => write!(f, "TUPLENEW {}", size),
            Instruction::TupleGet { index } => write!(f, "TUPLEGET {}", index),

            // Built-in Instructions
            Instruction::StringConcat => write!(f, "STRINGCONCAT"),
            Instruction::StringLength => write!(f, "STRINGLENGTH"),
            Instruction::StringSlice => write!(f, "STRINGSLICE"),

            // Quoting Instructions
            Instruction::Quote => write!(f, "QUOTE"),
            Instruction::Unquote => write!(f, "UNQUOTE"),
        }
    }
}

/// Helper functions for creating instructions
impl Instruction {
    /// Creates a Push instruction with an integer constant
    pub fn push_int(value: i64) -> Self {
        Instruction::Push(Constant::Literal(Literal::Int(value)))
    }

    /// Creates a Push instruction with a boolean constant
    pub fn push_bool(value: bool) -> Self {
        Instruction::Push(Constant::Literal(Literal::Bool(value)))
    }

    /// Creates a Push instruction with a string constant
    pub fn push_string<S: Into<String>>(value: S) -> Self {
        Instruction::Push(Constant::Literal(Literal::String(value.into())))
    }

    /// Creates a Push instruction with a URI constant
    pub fn push_uri<S: Into<String>>(value: S) -> Self {
        Instruction::Push(Constant::Literal(Literal::Uri(value.into())))
    }

    /// Creates a Push instruction with a byte array constant
    pub fn push_bytes<B: Into<Vec<u8>>>(value: B) -> Self {
        Instruction::Push(Constant::Literal(Literal::ByteArray(value.into())))
    }
}
