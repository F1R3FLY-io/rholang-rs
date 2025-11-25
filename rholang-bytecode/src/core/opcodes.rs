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
    /// Lookup table for opcode validation and conversion
    /// Each entry corresponds to the byte value index, with Some(opcode) for valid bytes
    const OPCODE_TABLE: [Option<Opcode>; 256] = {
        let mut table = [None; 256];

        // Control flow (0x00 - 0x0F)
        table[0x00] = Some(Opcode::NOP);
        table[0x01] = Some(Opcode::JUMP);
        table[0x02] = Some(Opcode::BRANCH_TRUE);
        table[0x03] = Some(Opcode::BRANCH_FALSE);
        table[0x04] = Some(Opcode::BRANCH_SUCCESS);
        table[0x05] = Some(Opcode::RETURN);
        table[0x06] = Some(Opcode::HALT);

        // Stack operations (0x10 - 0x1F)
        table[0x10] = Some(Opcode::PUSH_INT);
        table[0x11] = Some(Opcode::PUSH_STR);
        table[0x12] = Some(Opcode::PUSH_BOOL);
        table[0x13] = Some(Opcode::PUSH_PROC);
        table[0x14] = Some(Opcode::PUSH_NAME);
        table[0x15] = Some(Opcode::PUSH_NIL);
        table[0x16] = Some(Opcode::POP);
        table[0x17] = Some(Opcode::DUP);
        table[0x18] = Some(Opcode::SWAP);

        // Variable operations (0x20 - 0x2F)
        table[0x20] = Some(Opcode::LOAD_VAR);
        table[0x21] = Some(Opcode::LOAD_LOCAL);
        table[0x22] = Some(Opcode::STORE_LOCAL);
        table[0x23] = Some(Opcode::ALLOC_LOCAL);
        table[0x24] = Some(Opcode::LOAD_ENV);
        table[0x25] = Some(Opcode::STORE_ENV);

        // Arithmetic operations (0x30 - 0x3F)
        table[0x30] = Some(Opcode::ADD);
        table[0x31] = Some(Opcode::SUB);
        table[0x32] = Some(Opcode::MUL);
        table[0x33] = Some(Opcode::DIV);
        table[0x34] = Some(Opcode::MOD);
        table[0x35] = Some(Opcode::NEG);

        // Comparison operations (0x40 - 0x4F)
        table[0x40] = Some(Opcode::CMP_EQ);
        table[0x41] = Some(Opcode::CMP_NEQ);
        table[0x42] = Some(Opcode::CMP_LT);
        table[0x43] = Some(Opcode::CMP_LTE);
        table[0x44] = Some(Opcode::CMP_GT);
        table[0x45] = Some(Opcode::CMP_GTE);

        // Logical operations (0x50 - 0x5F)
        table[0x50] = Some(Opcode::NOT);
        table[0x51] = Some(Opcode::AND);
        table[0x52] = Some(Opcode::OR);

        // Collection operations (0x60 - 0x6F)
        table[0x60] = Some(Opcode::CREATE_LIST);
        table[0x61] = Some(Opcode::CREATE_TUPLE);
        table[0x62] = Some(Opcode::CREATE_MAP);
        table[0x63] = Some(Opcode::CONCAT);
        table[0x64] = Some(Opcode::DIFF);
        table[0x65] = Some(Opcode::INTERPOLATE);

        // Process operations (0x70 - 0x7F)
        table[0x70] = Some(Opcode::SPAWN_ASYNC);
        table[0x71] = Some(Opcode::EVAL);
        table[0x72] = Some(Opcode::EVAL_BOOL);
        table[0x73] = Some(Opcode::EVAL_STAR);
        table[0x74] = Some(Opcode::EXEC);
        table[0x75] = Some(Opcode::PROC_NEG);

        // RSpace operations (0x80 - 0x8F)
        table[0x80] = Some(Opcode::TELL);
        table[0x81] = Some(Opcode::ASK);
        table[0x82] = Some(Opcode::ASK_NB);
        table[0x83] = Some(Opcode::PEEK);
        table[0x84] = Some(Opcode::NAME_CREATE);
        table[0x85] = Some(Opcode::NAME_QUOTE);
        table[0x86] = Some(Opcode::NAME_UNQUOTE);
        table[0x87] = Some(Opcode::CONT_STORE);
        table[0x88] = Some(Opcode::CONT_RESUME);
        table[0x89] = Some(Opcode::BUNDLE_BEGIN);
        table[0x8A] = Some(Opcode::BUNDLE_END);

        // Pattern matching operations (0x90 - 0x9F)
        table[0x90] = Some(Opcode::PATTERN);
        table[0x91] = Some(Opcode::MATCH_TEST);
        table[0x92] = Some(Opcode::EXTRACT_BINDINGS);

        // Reference operations (0xA0 - 0xAF)
        table[0xA0] = Some(Opcode::COPY);
        table[0xA1] = Some(Opcode::MOVE);
        table[0xA2] = Some(Opcode::REF);

        // Method operations (0xB0 - 0xBF)
        table[0xB0] = Some(Opcode::LOAD_METHOD);
        table[0xB1] = Some(Opcode::INVOKE_METHOD);

        table
    };

    /// Operand count lookup table for performance
    const OPERAND_COUNTS: [u8; 256] = {
        let mut counts = [0u8; 256];

        // Nullary operations (0 operands)
        counts[0x00] = 0; // NOP
        counts[0x16] = 0; // POP
        counts[0x17] = 0; // DUP
        counts[0x18] = 0; // SWAP
        counts[0x06] = 0; // HALT
        counts[0x05] = 0; // RETURN
        counts[0x35] = 0; // NEG
        counts[0x50] = 0; // NOT
        counts[0x30] = 0; // ADD
        counts[0x31] = 0; // SUB
        counts[0x32] = 0; // MUL
        counts[0x33] = 0; // DIV
        counts[0x34] = 0; // MOD
        counts[0x40] = 0; // CMP_EQ
        counts[0x41] = 0; // CMP_NEQ
        counts[0x42] = 0; // CMP_LT
        counts[0x43] = 0; // CMP_LTE
        counts[0x44] = 0; // CMP_GT
        counts[0x45] = 0; // CMP_GTE
        counts[0x51] = 0; // AND
        counts[0x52] = 0; // OR
        counts[0x71] = 0; // EVAL
        counts[0x72] = 0; // EVAL_BOOL
        counts[0x73] = 0; // EVAL_STAR
        counts[0x74] = 0; // EXEC
        counts[0x75] = 0; // PROC_NEG
        counts[0x91] = 0; // MATCH_TEST
        counts[0x92] = 0; // EXTRACT_BINDINGS
        counts[0xA0] = 0; // COPY
        counts[0xA1] = 0; // MOVE
        counts[0xA2] = 0; // REF
        counts[0x63] = 0; // CONCAT
        counts[0x64] = 0; // DIFF
        counts[0x65] = 0; // INTERPOLATE
        counts[0x89] = 0; // BUNDLE_BEGIN
        counts[0x8A] = 0; // BUNDLE_END
        counts[0x15] = 0; // PUSH_NIL

        // Unary operations (1 operand)
        counts[0x01] = 1; // JUMP
        counts[0x02] = 1; // BRANCH_TRUE
        counts[0x03] = 1; // BRANCH_FALSE
        counts[0x04] = 1; // BRANCH_SUCCESS
        counts[0x10] = 1; // PUSH_INT
        counts[0x11] = 1; // PUSH_STR
        counts[0x12] = 1; // PUSH_BOOL
        counts[0x13] = 1; // PUSH_PROC
        counts[0x14] = 1; // PUSH_NAME
        counts[0x20] = 1; // LOAD_VAR
        counts[0x21] = 1; // LOAD_LOCAL
        counts[0x22] = 1; // STORE_LOCAL
        counts[0x23] = 1; // ALLOC_LOCAL
        counts[0x24] = 1; // LOAD_ENV
        counts[0x25] = 1; // STORE_ENV
        counts[0x60] = 1; // CREATE_LIST
        counts[0x61] = 1; // CREATE_TUPLE
        counts[0x62] = 1; // CREATE_MAP
        counts[0x70] = 1; // SPAWN_ASYNC
        counts[0x84] = 1; // NAME_CREATE
        counts[0x85] = 1; // NAME_QUOTE
        counts[0x86] = 1; // NAME_UNQUOTE
        counts[0x87] = 1; // CONT_STORE
        counts[0x88] = 1; // CONT_RESUME
        counts[0x90] = 1; // PATTERN
        counts[0xB0] = 1; // LOAD_METHOD
        counts[0xB1] = 1; // INVOKE_METHOD

        // Binary operations (2 operands)
        counts[0x80] = 2; // TELL
        counts[0x81] = 2; // ASK
        counts[0x82] = 2; // ASK_NB
        counts[0x83] = 2; // PEEK

        counts
    };

    /// Parse opcode from byte using lookup table
    pub fn from_byte(byte: u8) -> Result<Self> {
        Self::OPCODE_TABLE[byte as usize].ok_or(BytecodeError::InvalidOpcode(byte))
    }

    /// Get the number of operands this opcode expects
    pub fn operand_count(&self) -> u8 {
        Self::OPERAND_COUNTS[*self as u8 as usize]
    }

    const CONTROL_FLOW_FLAGS: [bool; 256] = {
        let mut flags = [false; 256];
        flags[0x01] = true; // JUMP
        flags[0x02] = true; // BRANCH_TRUE
        flags[0x03] = true; // BRANCH_FALSE
        flags[0x04] = true; // BRANCH_SUCCESS
        flags[0x05] = true; // RETURN
        flags[0x06] = true; // HALT
        flags
    };

    const RSPACE_OP_FLAGS: [bool; 256] = {
        let mut flags = [false; 256];
        flags[0x80] = true; // TELL
        flags[0x81] = true; // ASK
        flags[0x82] = true; // ASK_NB
        flags[0x83] = true; // PEEK
        flags[0x84] = true; // NAME_CREATE
        flags[0x85] = true; // NAME_QUOTE
        flags[0x86] = true; // NAME_UNQUOTE
        flags[0x87] = true; // CONT_STORE
        flags[0x88] = true; // CONT_RESUME
        flags[0x89] = true; // BUNDLE_BEGIN
        flags[0x8A] = true; // BUNDLE_END
        flags
    };

    /// Check if this opcode modifies control flow
    pub fn is_control_flow(&self) -> bool {
        Self::CONTROL_FLOW_FLAGS[*self as u8 as usize]
    }

    /// Check if this opcode is an RSpace operation
    pub fn is_rspace_op(&self) -> bool {
        Self::RSPACE_OP_FLAGS[*self as u8 as usize]
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
