//! Opcode definitions for Rholang bytecode

use crate::error::{BytecodeError, Result};
use bitflags::bitflags;

/// Primary opcode enumeration (8 bits)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(non_camel_case_types)] // Opcodes follow assembly/bytecode naming convention
pub enum Opcode {
    // Control flow (0x00 - 0x0F)
    NOP = 0x00,
    JUMP = 0x01,
    BRANCH_TRUE = 0x02,
    BRANCH_FALSE = 0x03,
    BRANCH_SUCCESS = 0x04,
    RETURN = 0x05,
    HALT = 0x06,

    // Stack operations (0x10 - 0x1F)
    PUSH_INT = 0x10,
    PUSH_STR = 0x11,
    PUSH_BOOL = 0x12,
    PUSH_PROC = 0x13,
    PUSH_NAME = 0x14,
    PUSH_NIL = 0x15,
    POP = 0x16,
    DUP = 0x17,
    SWAP = 0x18,

    // Variable operations (0x20 - 0x2F)
    LOAD_VAR = 0x20,
    LOAD_LOCAL = 0x21,
    STORE_LOCAL = 0x22,
    ALLOC_LOCAL = 0x23,
    LOAD_ENV = 0x24,
    STORE_ENV = 0x25,

    // Arithmetic operations (0x30 - 0x3F)
    ADD = 0x30,
    SUB = 0x31,
    MUL = 0x32,
    DIV = 0x33,
    MOD = 0x34,
    NEG = 0x35,

    // Comparison operations (0x40 - 0x4F)
    CMP_EQ = 0x40,
    CMP_NEQ = 0x41,
    CMP_LT = 0x42,
    CMP_LTE = 0x43,
    CMP_GT = 0x44,
    CMP_GTE = 0x45,

    // Logical operations (0x50 - 0x5F)
    NOT = 0x50,
    AND = 0x51,
    OR = 0x52,

    // Collection operations (0x60 - 0x6F)
    CREATE_LIST = 0x60,
    CREATE_TUPLE = 0x61,
    CREATE_MAP = 0x62,
    CONCAT = 0x63,
    DIFF = 0x64,
    INTERPOLATE = 0x65,

    // Process operations (0x70 - 0x7F)
    SPAWN_ASYNC = 0x70,
    EVAL = 0x71,
    EVAL_BOOL = 0x72,
    EVAL_STAR = 0x73,
    EXEC = 0x74,
    PROC_NEG = 0x75,

    // RSpace operations (0x80 - 0x8F)
    TELL = 0x80,
    ASK = 0x81,
    ASK_NB = 0x82,
    PEEK = 0x83,
    NAME_CREATE = 0x84,
    NAME_QUOTE = 0x85,
    NAME_UNQUOTE = 0x86,
    CONT_STORE = 0x87,
    CONT_RESUME = 0x88,
    BUNDLE_BEGIN = 0x89,
    BUNDLE_END = 0x8A,

    // Pattern matching operations (0x90 - 0x9F)
    PATTERN = 0x90,
    MATCH_TEST = 0x91,
    EXTRACT_BINDINGS = 0x92,

    // Reference operations (0xA0 - 0xAF)
    COPY = 0xA0,
    MOVE = 0xA1,
    REF = 0xA2,

    // Method operations (0xB0 - 0xBF)
    LOAD_METHOD = 0xB0,
    INVOKE_METHOD = 0xB1,
}

impl Opcode {
    /// Parse opcode from byte
    pub fn from_byte(byte: u8) -> Result<Self> {
        match byte {
            0x00 => Ok(Opcode::NOP),
            0x01 => Ok(Opcode::JUMP),
            0x02 => Ok(Opcode::BRANCH_TRUE),
            0x03 => Ok(Opcode::BRANCH_FALSE),
            0x04 => Ok(Opcode::BRANCH_SUCCESS),
            0x05 => Ok(Opcode::RETURN),
            0x06 => Ok(Opcode::HALT),

            0x10 => Ok(Opcode::PUSH_INT),
            0x11 => Ok(Opcode::PUSH_STR),
            0x12 => Ok(Opcode::PUSH_BOOL),
            0x13 => Ok(Opcode::PUSH_PROC),
            0x14 => Ok(Opcode::PUSH_NAME),
            0x15 => Ok(Opcode::PUSH_NIL),
            0x16 => Ok(Opcode::POP),
            0x17 => Ok(Opcode::DUP),
            0x18 => Ok(Opcode::SWAP),

            0x20 => Ok(Opcode::LOAD_VAR),
            0x21 => Ok(Opcode::LOAD_LOCAL),
            0x22 => Ok(Opcode::STORE_LOCAL),
            0x23 => Ok(Opcode::ALLOC_LOCAL),
            0x24 => Ok(Opcode::LOAD_ENV),
            0x25 => Ok(Opcode::STORE_ENV),

            0x30 => Ok(Opcode::ADD),
            0x31 => Ok(Opcode::SUB),
            0x32 => Ok(Opcode::MUL),
            0x33 => Ok(Opcode::DIV),
            0x34 => Ok(Opcode::MOD),
            0x35 => Ok(Opcode::NEG),

            0x40 => Ok(Opcode::CMP_EQ),
            0x41 => Ok(Opcode::CMP_NEQ),
            0x42 => Ok(Opcode::CMP_LT),
            0x43 => Ok(Opcode::CMP_LTE),
            0x44 => Ok(Opcode::CMP_GT),
            0x45 => Ok(Opcode::CMP_GTE),

            0x50 => Ok(Opcode::NOT),
            0x51 => Ok(Opcode::AND),
            0x52 => Ok(Opcode::OR),

            0x60 => Ok(Opcode::CREATE_LIST),
            0x61 => Ok(Opcode::CREATE_TUPLE),
            0x62 => Ok(Opcode::CREATE_MAP),
            0x63 => Ok(Opcode::CONCAT),
            0x64 => Ok(Opcode::DIFF),
            0x65 => Ok(Opcode::INTERPOLATE),

            0x70 => Ok(Opcode::SPAWN_ASYNC),
            0x71 => Ok(Opcode::EVAL),
            0x72 => Ok(Opcode::EVAL_BOOL),
            0x73 => Ok(Opcode::EVAL_STAR),
            0x74 => Ok(Opcode::EXEC),
            0x75 => Ok(Opcode::PROC_NEG),

            0x80 => Ok(Opcode::TELL),
            0x81 => Ok(Opcode::ASK),
            0x82 => Ok(Opcode::ASK_NB),
            0x83 => Ok(Opcode::PEEK),
            0x84 => Ok(Opcode::NAME_CREATE),
            0x85 => Ok(Opcode::NAME_QUOTE),
            0x86 => Ok(Opcode::NAME_UNQUOTE),
            0x87 => Ok(Opcode::CONT_STORE),
            0x88 => Ok(Opcode::CONT_RESUME),
            0x89 => Ok(Opcode::BUNDLE_BEGIN),
            0x8A => Ok(Opcode::BUNDLE_END),

            0x90 => Ok(Opcode::PATTERN),
            0x91 => Ok(Opcode::MATCH_TEST),
            0x92 => Ok(Opcode::EXTRACT_BINDINGS),

            0xA0 => Ok(Opcode::COPY),
            0xA1 => Ok(Opcode::MOVE),
            0xA2 => Ok(Opcode::REF),

            0xB0 => Ok(Opcode::LOAD_METHOD),
            0xB1 => Ok(Opcode::INVOKE_METHOD),

            _ => Err(BytecodeError::InvalidOpcode(byte)),
        }
    }

    /// Get the number of operands this opcode expects
    pub fn operand_count(&self) -> u8 {
        match self {
            Opcode::NOP
            | Opcode::POP
            | Opcode::DUP
            | Opcode::SWAP
            | Opcode::HALT
            | Opcode::RETURN
            | Opcode::NEG
            | Opcode::NOT
            | Opcode::ADD
            | Opcode::SUB
            | Opcode::MUL
            | Opcode::DIV
            | Opcode::MOD
            | Opcode::CMP_EQ
            | Opcode::CMP_NEQ
            | Opcode::CMP_LT
            | Opcode::CMP_LTE
            | Opcode::CMP_GT
            | Opcode::CMP_GTE
            | Opcode::AND
            | Opcode::OR
            | Opcode::EVAL
            | Opcode::EVAL_BOOL
            | Opcode::EVAL_STAR
            | Opcode::EXEC
            | Opcode::PROC_NEG
            | Opcode::MATCH_TEST
            | Opcode::EXTRACT_BINDINGS
            | Opcode::COPY
            | Opcode::MOVE
            | Opcode::REF
            | Opcode::CONCAT
            | Opcode::DIFF
            | Opcode::INTERPOLATE
            | Opcode::BUNDLE_BEGIN
            | Opcode::BUNDLE_END
            | Opcode::PUSH_NIL => 0,

            Opcode::JUMP
            | Opcode::BRANCH_TRUE
            | Opcode::BRANCH_FALSE
            | Opcode::BRANCH_SUCCESS
            | Opcode::PUSH_INT
            | Opcode::PUSH_STR
            | Opcode::PUSH_BOOL
            | Opcode::PUSH_PROC
            | Opcode::PUSH_NAME
            | Opcode::LOAD_VAR
            | Opcode::LOAD_LOCAL
            | Opcode::STORE_LOCAL
            | Opcode::ALLOC_LOCAL
            | Opcode::LOAD_ENV
            | Opcode::STORE_ENV
            | Opcode::CREATE_LIST
            | Opcode::CREATE_TUPLE
            | Opcode::CREATE_MAP
            | Opcode::SPAWN_ASYNC
            | Opcode::NAME_CREATE
            | Opcode::NAME_QUOTE
            | Opcode::NAME_UNQUOTE
            | Opcode::CONT_STORE
            | Opcode::CONT_RESUME
            | Opcode::PATTERN
            | Opcode::LOAD_METHOD
            | Opcode::INVOKE_METHOD => 1,

            Opcode::TELL | Opcode::ASK | Opcode::ASK_NB | Opcode::PEEK => 2,
        }
    }

    /// Check if this opcode modifies control flow
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            Opcode::JUMP
                | Opcode::BRANCH_TRUE
                | Opcode::BRANCH_FALSE
                | Opcode::BRANCH_SUCCESS
                | Opcode::RETURN
                | Opcode::HALT
        )
    }

    /// Check if this opcode is an RSpace operation
    pub fn is_rspace_op(&self) -> bool {
        matches!(
            self,
            Opcode::TELL
                | Opcode::ASK
                | Opcode::ASK_NB
                | Opcode::PEEK
                | Opcode::NAME_CREATE
                | Opcode::NAME_QUOTE
                | Opcode::NAME_UNQUOTE
                | Opcode::CONT_STORE
                | Opcode::CONT_RESUME
                | Opcode::BUNDLE_BEGIN
                | Opcode::BUNDLE_END
        )
    }
}

bitflags! {
    /// Instruction flags for additional metadata (8 bits)
    #[derive(Debug, Clone, Copy)]
    pub struct InstructionFlags: u8 {
        /// Instruction has been optimized
        const OPTIMIZED = 0b00000001;

        /// Instruction is a jump target
        const JUMP_TARGET = 0b00000010;

        /// Instruction begins a basic block
        const BLOCK_START = 0b00000100;

        /// Instruction ends a basic block
        const BLOCK_END = 0b00001000;

        /// Instruction has cost accounting
        const HAS_COST = 0b00010000;

        /// Instruction can throw an error
        const CAN_FAIL = 0b00100000;

        /// Instruction is in a hot path
        const HOT_PATH = 0b01000000;

        /// Instruction has debug info
        const DEBUG_INFO = 0b10000000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_from_byte() {
        assert_eq!(Opcode::from_byte(0x00).unwrap(), Opcode::NOP);
        assert_eq!(Opcode::from_byte(0x80).unwrap(), Opcode::TELL);
        assert!(Opcode::from_byte(0xFF).is_err());
    }

    #[test]
    fn test_opcode_properties() {
        assert_eq!(Opcode::NOP.operand_count(), 0);
        assert_eq!(Opcode::JUMP.operand_count(), 1);
        assert_eq!(Opcode::TELL.operand_count(), 2);

        assert!(Opcode::JUMP.is_control_flow());
        assert!(!Opcode::ADD.is_control_flow());

        assert!(Opcode::TELL.is_rspace_op());
        assert!(!Opcode::ADD.is_rspace_op());
    }

    #[test]
    fn test_instruction_flags() {
        let mut flags = InstructionFlags::empty();
        assert!(!flags.contains(InstructionFlags::OPTIMIZED));

        flags |= InstructionFlags::OPTIMIZED | InstructionFlags::HOT_PATH;
        assert!(flags.contains(InstructionFlags::OPTIMIZED));
        assert!(flags.contains(InstructionFlags::HOT_PATH));
    }
}
