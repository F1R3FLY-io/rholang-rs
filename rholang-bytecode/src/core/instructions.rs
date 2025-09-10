//! Instruction definitions and encoding for Rholang bytecode
//!
//! Implements 32-bit fixed-width instructions with zero-copy operands

use crate::core::opcodes::{InstructionFlags, Opcode};
use crate::core::types::{NameRef, ProcessRef, RSpaceType};
use crate::error::{BytecodeError, Result};
use byteorder::{ByteOrder, LittleEndian};
use std::fmt;

/// 32-bit fixed-width instruction
/// Layout: [opcode:8][flags:8][operand1:8][operand2:8]
#[derive(Clone, Copy)]
#[repr(C, align(4))]
pub struct Instruction {
    pub opcode: u8,
    pub flags: u8,
    pub operands: [u8; 2],
}

impl Instruction {
    pub const fn new(opcode: Opcode, flags: InstructionFlags, op1: u8, op2: u8) -> Self {
        Self {
            opcode: opcode as u8,
            flags: flags.bits(),
            operands: [op1, op2],
        }
    }

    pub const fn nullary(opcode: Opcode) -> Self {
        Self::new(opcode, InstructionFlags::empty(), 0, 0)
    }

    pub const fn unary(opcode: Opcode, operand: u16) -> Self {
        let bytes = operand.to_le_bytes();
        Self::new(opcode, InstructionFlags::empty(), bytes[0], bytes[1])
    }

    pub const fn binary(opcode: Opcode, op1: u8, op2: u8) -> Self {
        Self::new(opcode, InstructionFlags::empty(), op1, op2)
    }

    pub fn opcode(&self) -> Result<Opcode> {
        Opcode::from_byte(self.opcode)
    }

    pub fn flags(&self) -> InstructionFlags {
        InstructionFlags::from_bits_truncate(self.flags)
    }

    pub fn op1(&self) -> u8 {
        self.operands[0]
    }

    pub fn op2(&self) -> u8 {
        self.operands[1]
    }

    pub fn op16(&self) -> u16 {
        LittleEndian::read_u16(&self.operands)
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        [self.opcode, self.flags, self.operands[0], self.operands[1]]
    }

    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            opcode: bytes[0],
            flags: bytes[1],
            operands: [bytes[2], bytes[3]],
        }
    }

    pub fn validate(&self) -> Result<()> {
        let opcode = self.opcode()?;
        let expected_operands = opcode.operand_count();

        // Check that unused operand bytes are zero
        match expected_operands {
            0 => {
                if self.operands[0] != 0 || self.operands[1] != 0 {
                    return Err(BytecodeError::InvalidInstruction {
                        offset: 0, // Will be filled by caller
                    });
                }
            }
            1 => {
                // 16-bit operand uses both bytes, so no check needed
            }
            2 => {
                // Both bytes used for two 8-bit operands
            }
            _ => unreachable!("Invalid operand count"),
        }

        Ok(())
    }
}

impl fmt::Debug for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(opcode) = self.opcode() {
            write!(f, "{opcode:?}")?;
            match opcode.operand_count() {
                0 => Ok(()),
                1 => write!(f, " {:#04x}", self.op16()),
                2 => write!(f, " {:#02x}, {:#02x}", self.op1(), self.op2()),
                _ => unreachable!(),
            }
        } else {
            write!(f, "INVALID({:#02x})", self.opcode)
        }
    }
}

/// Extended instruction data for complex operands
/// Stored separately to maintain 32-bit instruction alignment
#[derive(Clone, Debug)]
pub enum InstructionData {
    /// Integer literal
    Integer(i64),

    /// String reference (index into string pool)
    String(u32),

    /// Process reference
    Process(ProcessRef),

    /// Name reference
    Name(NameRef),

    /// Pattern ID
    Pattern(u32),

    /// Label offset for jumps
    Label(i32),

    /// Method name ID
    Method(u32),

    /// RSpace type hint
    RSpace(RSpaceType),
}

/// Instruction with associated data
#[derive(Clone, Debug)]
pub struct ExtendedInstruction {
    pub instruction: Instruction,
    pub data: Option<InstructionData>,
}

impl ExtendedInstruction {
    pub fn simple(instruction: Instruction) -> Self {
        Self {
            instruction,
            data: None,
        }
    }

    pub fn with_data(instruction: Instruction, data: InstructionData) -> Self {
        Self {
            instruction,
            data: Some(data),
        }
    }
}

/// Instruction builder for convenient construction
pub struct InstructionBuilder {
    instructions: Vec<ExtendedInstruction>,
    labels: Vec<Option<usize>>,
    current_offset: usize,
}

impl Default for InstructionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl InstructionBuilder {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            labels: Vec::new(),
            current_offset: 0,
        }
    }

    /// Emit a simple instruction
    pub fn emit(&mut self, instruction: Instruction) -> &mut Self {
        self.instructions
            .push(ExtendedInstruction::simple(instruction));
        self.current_offset += 4;
        self
    }

    /// Emit an instruction with data
    pub fn emit_with_data(&mut self, instruction: Instruction, data: InstructionData) -> &mut Self {
        self.instructions
            .push(ExtendedInstruction::with_data(instruction, data));
        self.current_offset += 4;
        self
    }

    /// Create a new label
    pub fn create_label(&mut self) -> usize {
        let label_id = self.labels.len();
        self.labels.push(None);
        label_id
    }

    /// Place a label at the current position
    pub fn place_label(&mut self, label_id: usize) -> &mut Self {
        if label_id < self.labels.len() {
            self.labels[label_id] = Some(self.current_offset);
        }
        self
    }

    /// Emit a jump to a label
    pub fn emit_jump(&mut self, label_id: usize) -> &mut Self {
        let instruction = Instruction::unary(Opcode::JUMP, label_id as u16);
        self.emit(instruction)
    }

    /// Build the final instruction sequence
    pub fn build(self) -> Vec<ExtendedInstruction> {
        // TODO: Resolve label references
        self.instructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_encoding() {
        let inst = Instruction::nullary(Opcode::NOP);
        assert_eq!(inst.opcode, 0x00);
        assert_eq!(inst.flags, 0x00);
        assert_eq!(inst.operands, [0x00, 0x00]);

        let bytes = inst.to_bytes();
        assert_eq!(bytes, [0x00, 0x00, 0x00, 0x00]);

        let decoded = Instruction::from_bytes(bytes);
        assert_eq!(decoded.opcode, inst.opcode);
    }

    #[test]
    fn test_instruction_operands() {
        let inst = Instruction::unary(Opcode::JUMP, 0x1234);
        assert_eq!(inst.op16(), 0x1234);

        let inst = Instruction::binary(Opcode::TELL, 0x12, 0x34);
        assert_eq!(inst.op1(), 0x12);
        assert_eq!(inst.op2(), 0x34);
    }

    #[test]
    fn test_instruction_validation() {
        let valid = Instruction::nullary(Opcode::NOP);
        assert!(valid.validate().is_ok());

        let invalid = Instruction::new(
            Opcode::NOP,
            InstructionFlags::empty(),
            0x01, // Should be 0 for NOP
            0x00,
        );
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_instruction_builder() {
        let mut builder = InstructionBuilder::new();
        let label = builder.create_label();

        builder
            .emit(Instruction::nullary(Opcode::NOP))
            .emit_jump(label)
            .place_label(label)
            .emit(Instruction::nullary(Opcode::HALT));

        let instructions = builder.build();
        assert_eq!(instructions.len(), 3);
    }
}
