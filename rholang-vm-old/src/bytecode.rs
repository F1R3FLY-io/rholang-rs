// rholang-vm bytecode facade: re-export rholang-bytecode core types
// Full immediate replacement: remove legacy VM enums and use core definitions

pub use rholang_bytecode::core::constants::*;
pub use rholang_bytecode::core::instructions::{ExtendedInstruction, InstructionData};
use rholang_bytecode::core::instructions as core_instrs;
pub use rholang_bytecode::core::module::{BytecodeModule, OptimizationLevel, PatternPool, ProcessHeap, ReferenceTable, ReferenceType};
pub use rholang_bytecode::core::opcodes::Opcode;
pub use rholang_bytecode::core::types::{Key, NameRef, ProcessRef, RSpaceType, TypeRef, Value};

use serde::{Deserialize, Serialize};

/// Label for jump instructions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Label(pub String);

/// Bundle operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BundleOp {
    Read,
    Write,
    ReadWrite,
    Equiv,
}

/// High-level VM Instruction set (not provided by rholang-bytecode core)
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    // Computational Instructions
    Nop,
    PushInt(i64),
    PushStr(String),
    PushBool(bool),
    PushProc(String),
    Pop,
    Dup,
    LoadVar(usize),
    LoadLocal(usize),
    StoreLocal(usize),
    AllocLocal,
    BranchTrue(Label),
    BranchFalse(Label),
    BranchSuccess(Label),
    Jump(Label),
    CmpEq,
    CmpNeq,
    CmpLt,
    CmpLte,
    CmpGt,
    CmpGte,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Not,
    Concat,
    Diff,
    Interpolate,
    CreateList(usize),
    CreateTuple(usize),
    CreateMap(usize),
    InvokeMethod,
    // Evaluation
    Eval,
    EvalBool,
    EvalStar,
    EvalToRSpace,
    EvalWithLocals,
    EvalInBundle,
    Exec,
    // Pattern matching
    Pattern(String),
    MatchTest,
    ExtractBindings,
    // Process control
    SpawnAsync(RSpaceType),
    ProcNeg,
    Conj,
    Disj,
    // Reference ops
    Copy,
    Move,
    Ref,
    LoadMethod(String),
    // RSpace
    RSpaceProduce(RSpaceType),
    RSpaceConsume(RSpaceType),
    RSpaceConsumeNonblock(RSpaceType),
    RSpaceConsumePersistent(RSpaceType),
    RSpacePeek(RSpaceType),
    RSpaceMatch(RSpaceType),
    RSpaceSelectBegin(RSpaceType),
    RSpaceSelectAdd(RSpaceType),
    RSpaceSelectWait(RSpaceType),
    NameCreate(RSpaceType),
    NameQuote(RSpaceType),
    NameUnquote(RSpaceType),
    PatternCompile(RSpaceType),
    PatternBind(RSpaceType),
    ContinuationStore(RSpaceType),
    ContinuationResume(RSpaceType),
    RSpaceBundleBegin(RSpaceType, BundleOp),
    RSpaceBundleEnd(RSpaceType),
    // Label
    Label(Label),
}

// Provide a minimal helper API for building common ExtendedInstruction forms used by the VM/tests.
// This keeps rholang-vm imports stable while moving entirely to core types.

pub fn ext_simple(op: Opcode) -> ExtendedInstruction {
    ExtendedInstruction::simple(core_instrs::Instruction::nullary(op))
}

pub fn ext_with_int(op: Opcode, n: i64) -> ExtendedInstruction {
    ExtendedInstruction::with_data(core_instrs::Instruction::nullary(op), InstructionData::Integer(n))
}

pub fn ext_with_u16(op: Opcode, n: u16) -> ExtendedInstruction {
    // pack small immediates into operand, no extra data
    ExtendedInstruction::simple(core_instrs::Instruction::unary(op, n))
}

pub fn ext_with_u32_as_label(op: Opcode, pc: u32) -> ExtendedInstruction {
    ExtendedInstruction::with_data(core_instrs::Instruction::nullary(op), InstructionData::Label(pc as i32))
}
