// Rholang Bytecode Format and Instruction Set
// Based on the design in BYTECODE_DESIGN.md

use serde::{Deserialize, Serialize};
use std::fmt;

/// RSpace types as defined in the bytecode design
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RSpaceType {
    /// In-memory sequential (hashmap)
    MemorySequential,
    /// In-memory concurrent (concurrent hashmap)
    MemoryConcurrent,
    /// On-store sequential (LMDB wrapper)
    StoreSequential,
    /// On-store concurrent (LMDB wrapper)
    StoreConcurrent,
}

impl fmt::Display for RSpaceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RSpaceType::MemorySequential => write!(f, "RSPACE_MEM_SEQ"),
            RSpaceType::MemoryConcurrent => write!(f, "RSPACE_MEM_CONC"),
            RSpaceType::StoreSequential => write!(f, "RSPACE_STORE_SEQ"),
            RSpaceType::StoreConcurrent => write!(f, "RSPACE_STORE_CONC"),
        }
    }
}

/// Bundle operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BundleOp {
    /// Read-only bundle
    Read,
    /// Write-only bundle
    Write,
    /// Read-write bundle
    ReadWrite,
}

/// Value types that can be stored on the stack
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Integer value
    Int(i64),
    /// String value
    String(String),
    /// Boolean value
    Bool(bool),
    /// Process value (represented as a string for now)
    Process(String),
    /// Name value
    Name(String),
    /// List value
    List(Vec<Value>),
    /// Tuple value
    Tuple(Vec<Value>),
    /// Map value
    Map(Vec<(Value, Value)>),
    /// Nil value
    Nil,
}

/// Label for jump instructions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Label(pub String);

/// Bytecode instruction set
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    // Computational Instructions
    /// No operation
    Nop,
    /// Push integer literal
    PushInt(i64),
    /// Push string literal
    PushStr(String),
    /// Push boolean literal
    PushBool(bool),
    /// Push process to stack
    PushProc(String),
    /// Pop top of stack
    Pop,
    /// Duplicate top of stack
    Dup,
    /// Load variable by index
    LoadVar(usize),
    /// Load local variable by index
    LoadLocal(usize),
    /// Store to local variable
    StoreLocal(usize),
    /// Allocate new local slot
    AllocLocal,
    /// Conditional jump if true
    BranchTrue(Label),
    /// Conditional jump if false
    BranchFalse(Label),
    /// Branch if operation succeeded
    BranchSuccess(Label),
    /// Unconditional jump
    Jump(Label),
    /// Equality comparison
    CmpEq,
    /// Inequality comparison
    CmpNeq,
    /// Less than comparison
    CmpLt,
    /// Less than or equal
    CmpLte,
    /// Greater than comparison
    CmpGt,
    /// Greater than or equal
    CmpGte,
    /// Arithmetic addition
    Add,
    /// Arithmetic subtraction
    Sub,
    /// Arithmetic multiplication
    Mul,
    /// Arithmetic division
    Div,
    /// Arithmetic modulo
    Mod,
    /// Arithmetic negation
    Neg,
    /// Logical NOT
    Not,
    /// String/collection concatenation
    Concat,
    /// Collection difference
    Diff,
    /// String interpolation
    Interpolate,
    /// Create list from n stack elements
    CreateList(usize),
    /// Create tuple from n stack elements
    CreateTuple(usize),
    /// Method invocation
    InvokeMethod,

    // Evaluation Instructions
    /// Evaluate process on stack
    Eval,
    /// Evaluate to boolean
    EvalBool,
    /// Evaluate and prepare for RSpace
    EvalToRSpace,
    /// Evaluate with local bindings
    EvalWithLocals,
    /// Evaluate in bundle context
    EvalInBundle,
    /// Execute process on stack
    Exec,

    // Pattern Matching Instructions
    /// Load pattern
    Pattern(String),
    /// Test pattern match (leaves boolean on stack)
    MatchTest,
    /// Extract bound variables from pattern match
    ExtractBindings,

    // Data Structure Instructions
    /// Start map construction
    MapBegin,
    /// Add key-value pair to map
    MapPut,
    /// Finish map construction
    MapEnd,

    // Process Control Instructions
    /// Spawn process asynchronously
    SpawnAsync,
    /// Process negation
    ProcNeg,

    // Reference Instructions
    /// Copy value
    Copy,
    /// Move value
    Move,
    /// Create reference
    Ref,
    /// Load method name for invocation
    LoadMethod(String),

    // RSpace Instructions
    /// Put data into specified RSpace
    RSpacePut(RSpaceType),
    /// Get data from specified RSpace (blocking)
    RSpaceGet(RSpaceType),
    /// Get data from specified RSpace (non-blocking)
    RSpaceGetNonblock(RSpaceType),
    /// Consume data from specified RSpace
    RSpaceConsume(RSpaceType),
    /// Produce data to specified RSpace
    RSpaceProduce(RSpaceType),
    /// Peek at data without consuming
    RSpacePeek(RSpaceType),
    /// Pattern match against specified RSpace data
    RSpaceMatch(RSpaceType),
    /// Atomic select operation across channels
    RSpaceSelect(RSpaceType),
    /// Create fresh name in specified RSpace
    NameCreate(RSpaceType),
    /// Quote process to name in specified RSpace
    NameQuote(RSpaceType),
    /// Unquote name to process in specified RSpace
    NameUnquote(RSpaceType),
    /// Compile pattern for specified RSpace matching
    PatternCompile(RSpaceType),
    /// Bind pattern variables from specified RSpace match
    PatternBind(RSpaceType),
    /// Store continuation in specified RSpace
    ContinuationStore(RSpaceType),
    /// Resume stored continuation from specified RSpace
    ContinuationResume(RSpaceType),
    /// Start bundle in specified RSpace
    RSpaceBundleBegin(RSpaceType, BundleOp),
    /// End bundle in specified RSpace
    RSpaceBundleEnd(RSpaceType),

    // Label definition (for jumps)
    Label(Label),
}

/// A sequence of bytecode instructions
pub type BytecodeProgram = Vec<Instruction>;

/// Serialize bytecode to a string
pub fn serialize_bytecode(program: &BytecodeProgram) -> Result<String, serde_json::Error> {
    serde_json::to_string(program)
}

/// Deserialize bytecode from a string
pub fn deserialize_bytecode(s: &str) -> Result<BytecodeProgram, serde_json::Error> {
    serde_json::from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_deserialize() {
        let program = vec![
            Instruction::PushInt(42),
            Instruction::PushStr("hello".to_string()),
            Instruction::Add,
        ];

        let serialized = serialize_bytecode(&program).unwrap();
        let deserialized = deserialize_bytecode(&serialized).unwrap();

        assert_eq!(program, deserialized);
    }
}