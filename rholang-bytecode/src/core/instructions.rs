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
                // 16-bit operand uses both bytes, validate range for specific opcodes
                let operand_value = self.op16();
                self.validate_operand_range(opcode, operand_value)?;
            }
            2 => {
                // Both bytes used for two 8-bit operands, validate individually
                self.validate_operand_range(opcode, self.op1() as u16)?;
                self.validate_operand_range(opcode, self.op2() as u16)?;
            }
            _ => unreachable!("Invalid operand count"),
        }

        Ok(())
    }

    /// Validate that operand values are within reasonable bounds for the opcode
    fn validate_operand_range(&self, opcode: Opcode, operand: u16) -> Result<()> {
        match opcode {
            Opcode::PUSH_INT | Opcode::PUSH_STR | Opcode::PUSH_PROC | Opcode::PUSH_NAME => {
                // Constant pool indices should be reasonable (max 65536 constants)
                // No additional validation needed as u16 already limits this
            }

            // Variable operations - validate local variable indices
            Opcode::LOAD_LOCAL | Opcode::STORE_LOCAL | Opcode::ALLOC_LOCAL => {
                // Local variable indices should be reasonable (max 1024 locals)
                const MAX_LOCAL_VARS: u16 = 1024;
                if operand >= MAX_LOCAL_VARS {
                    return Err(BytecodeError::ValidationError(format!(
                        "Local variable index {operand} exceeds maximum {MAX_LOCAL_VARS}"
                    )));
                }
            }

            // Environment operations - validate environment indices
            Opcode::LOAD_ENV | Opcode::STORE_ENV => {
                // Environment indices should be reasonable (max 256 environment slots)
                const MAX_ENV_SLOTS: u16 = 256;
                if operand >= MAX_ENV_SLOTS {
                    return Err(BytecodeError::ValidationError(format!(
                        "Environment index {operand} exceeds maximum {MAX_ENV_SLOTS}"
                    )));
                }
            }

            // Jump operations - operands are offsets, validated elsewhere
            Opcode::JUMP | Opcode::BRANCH_TRUE | Opcode::BRANCH_FALSE | Opcode::BRANCH_SUCCESS => {
                // Jump offsets are validated during label resolution
            }

            // Collection operations - validate size limits
            Opcode::CREATE_LIST | Opcode::CREATE_TUPLE | Opcode::CREATE_MAP => {
                // Collection sizes should be reasonable (max 65536 elements)
                // No additional validation needed as u16 already limits this
            }

            // Process operations - validate reasonable limits
            Opcode::SPAWN_ASYNC => {
                // Process spawn count should be reasonable
                const MAX_SPAWN_COUNT: u16 = 10000;
                if operand >= MAX_SPAWN_COUNT {
                    return Err(BytecodeError::ValidationError(format!(
                        "Spawn count {operand} exceeds maximum {MAX_SPAWN_COUNT}"
                    )));
                }
            }

            // RSpace operations with 2 operands need both validated
            Opcode::TELL | Opcode::ASK | Opcode::ASK_NB | Opcode::PEEK => {
                // These are validated as pairs in the 2-operand case above
            }

            // Method operations
            Opcode::LOAD_METHOD | Opcode::INVOKE_METHOD => {
                // Method indices should be reasonable (max 65536 methods)
                // No additional validation needed as u16 already limits this
            }

            // Pattern operations
            Opcode::PATTERN => {
                // Pattern indices should be reasonable (max 65536 patterns)
                // No additional validation needed as u16 already limits this
            }

            // Other operations don't need operand validation
            _ => {}
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

#[derive(Debug, Clone)]
struct UnresolvedJump {
    instruction_index: usize,
    label_id: usize,
    /// Type of jump instruction
    #[allow(dead_code)] // Reserved for future optimizations
    jump_type: JumpType,
}

#[derive(Debug, Clone, Copy)]
enum JumpType {
    Absolute,           // JUMP
    ConditionalTrue,    // BRANCH_TRUE
    ConditionalFalse,   // BRANCH_FALSE
    ConditionalSuccess, // BRANCH_SUCCESS
}

/// Instruction builder for convenient construction
pub struct InstructionBuilder {
    instructions: Vec<ExtendedInstruction>,
    labels: Vec<Option<usize>>,
    unresolved_jumps: Vec<UnresolvedJump>,
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
            unresolved_jumps: Vec::new(),
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
        let instruction = Instruction::unary(Opcode::JUMP, 0); // Placeholder operand
        let instruction_index = self.instructions.len();
        self.emit(instruction);

        self.unresolved_jumps.push(UnresolvedJump {
            instruction_index,
            label_id,
            jump_type: JumpType::Absolute,
        });

        self
    }

    pub fn emit_branch_true(&mut self, label_id: usize) -> &mut Self {
        let instruction = Instruction::unary(Opcode::BRANCH_TRUE, 0);
        let instruction_index = self.instructions.len();
        self.emit(instruction);

        self.unresolved_jumps.push(UnresolvedJump {
            instruction_index,
            label_id,
            jump_type: JumpType::ConditionalTrue,
        });

        self
    }

    pub fn emit_branch_false(&mut self, label_id: usize) -> &mut Self {
        let instruction = Instruction::unary(Opcode::BRANCH_FALSE, 0);
        let instruction_index = self.instructions.len();
        self.emit(instruction);

        self.unresolved_jumps.push(UnresolvedJump {
            instruction_index,
            label_id,
            jump_type: JumpType::ConditionalFalse,
        });

        self
    }

    pub fn emit_branch_success(&mut self, label_id: usize) -> &mut Self {
        let instruction = Instruction::unary(Opcode::BRANCH_SUCCESS, 0);
        let instruction_index = self.instructions.len();
        self.emit(instruction);

        self.unresolved_jumps.push(UnresolvedJump {
            instruction_index,
            label_id,
            jump_type: JumpType::ConditionalSuccess,
        });

        self
    }

    /// Build the final instruction sequence
    pub fn build(mut self) -> Result<Vec<ExtendedInstruction>> {
        // Resolve all label references
        self.resolve_labels()?;

        // Apply instruction compression optimizations
        self.compress_patterns();

        Ok(self.instructions)
    }

    /// Build without optimizations (for debugging or when optimization is disabled)
    pub fn build_unoptimized(mut self) -> Result<Vec<ExtendedInstruction>> {
        // Resolve all label references
        self.resolve_labels()?;
        Ok(self.instructions)
    }

    /// Resolve all label references in jump instructions
    fn resolve_labels(&mut self) -> Result<()> {
        for unresolved in &self.unresolved_jumps {
            // Get the target address for this label
            let label_position = self
                .labels
                .get(unresolved.label_id)
                .ok_or(BytecodeError::InvalidLabel {
                    label_id: unresolved.label_id,
                })?
                .ok_or_else(|| BytecodeError::UnresolvedLabel {
                    label_id: unresolved.label_id,
                })?;

            // Calculate the jump offset
            let jump_instruction_position = unresolved
                .instruction_index
                .checked_mul(4)
                .ok_or(BytecodeError::JumpOutOfRange {
                    offset: 0,
                    max_range: i16::MAX as i32,
                })?;

            let offset = if label_position >= jump_instruction_position {
                let diff = label_position - jump_instruction_position;
                i32::try_from(diff).map_err(|_| BytecodeError::JumpOutOfRange {
                    offset: diff as i32, // This cast is for error reporting only
                    max_range: i16::MAX as i32,
                })?
            } else {
                let diff = jump_instruction_position - label_position;
                let signed_diff =
                    i32::try_from(diff).map_err(|_| BytecodeError::JumpOutOfRange {
                        offset: -(diff as i32), // This cast is for error reporting only
                        max_range: i16::MAX as i32,
                    })?;
                -signed_diff
            };

            // Check if offset fits in 16-bit signed range
            if offset < i16::MIN as i32 || offset > i16::MAX as i32 {
                return Err(BytecodeError::JumpOutOfRange {
                    offset,
                    max_range: i16::MAX as i32,
                });
            }

            // Update the instruction with the resolved offset
            if let Some(extended_instruction) =
                self.instructions.get_mut(unresolved.instruction_index)
            {
                // Update the operand with the calculated offset
                let offset_u16 = offset as u16;
                let bytes = offset_u16.to_le_bytes();
                extended_instruction.instruction.operands[0] = bytes[0];
                extended_instruction.instruction.operands[1] = bytes[1];

                // Add label data for extended instruction
                extended_instruction.data = Some(InstructionData::Label(offset));
            } else {
                return Err(BytecodeError::InvalidInstruction {
                    offset: jump_instruction_position,
                });
            }
        }

        Ok(())
    }

    /// Compress common instruction patterns for better performance
    fn compress_patterns(&mut self) {
        let mut i = 0;
        let mut compressed_instructions = Vec::new();

        while i < self.instructions.len() {
            // Look for common patterns and compress them
            if let Some(compressed_count) = self.try_compress_push_pop_sequence(i) {
                // Skip the original instructions as they were compressed
                i += compressed_count;
            } else if let Some(compressed_count) = self.try_compress_variable_sequence(i) {
                i += compressed_count;
            } else {
                // No compression possible
                compressed_instructions.push(self.instructions[i].clone());
                i += 1;
            }
        }

        // Replace with compressed instructions if we achieved any compression
        if compressed_instructions.len() < self.instructions.len() {
            self.instructions = compressed_instructions;
        }
    }

    /// Try to compress push-pop sequences (returns number of instructions consumed)
    fn try_compress_push_pop_sequence(&mut self, start: usize) -> Option<usize> {
        if start + 1 >= self.instructions.len() {
            return None;
        }

        let first = &self.instructions[start];
        let second = &self.instructions[start + 1];

        // Pattern: PUSH_X followed by POP -> NOP (can be eliminated entirely)
        if let (Ok(first_op), Ok(second_op)) =
            (first.instruction.opcode(), second.instruction.opcode())
        {
            if matches!(
                first_op,
                Opcode::PUSH_INT
                    | Opcode::PUSH_STR
                    | Opcode::PUSH_BOOL
                    | Opcode::PUSH_PROC
                    | Opcode::PUSH_NAME
                    | Opcode::PUSH_NIL
            ) && matches!(second_op, Opcode::POP)
            {
                // This pair can be eliminated (push followed immediately by pop)
                return Some(2);
            }
        }

        None
    }

    /// Try to compress variable access sequences
    fn try_compress_variable_sequence(&mut self, start: usize) -> Option<usize> {
        if start + 1 >= self.instructions.len() {
            return None;
        }

        let first = &self.instructions[start];
        let second = &self.instructions[start + 1];

        if let (Ok(first_op), Ok(second_op)) =
            (first.instruction.opcode(), second.instruction.opcode())
        {
            // Pattern: LOAD_LOCAL followed by STORE_LOCAL to same location -> NOP
            if matches!(first_op, Opcode::LOAD_LOCAL) && matches!(second_op, Opcode::STORE_LOCAL) {
                let first_operand = first.instruction.op16();
                let second_operand = second.instruction.op16();

                if first_operand == second_operand {
                    // Load and store to same location - this is a no-op
                    return Some(2);
                }
            }
        }

        None
    }

    /// Emit optimized sequence for pushing multiple constants
    pub fn emit_push_sequence(&mut self, values: &[u16]) -> &mut Self {
        // For now, emit individually (future optimization could batch these)
        for &value in values {
            self.emit(Instruction::unary(Opcode::PUSH_INT, value));
        }
        self
    }

    /// Get compression statistics
    pub fn compression_stats(&self) -> CompressionStats {
        let mut push_pop_pairs = 0;
        let mut redundant_loads = 0;

        for i in 0..self.instructions.len().saturating_sub(1) {
            if let (Ok(first_op), Ok(second_op)) = (
                self.instructions[i].instruction.opcode(),
                self.instructions[i + 1].instruction.opcode(),
            ) {
                if matches!(
                    first_op,
                    Opcode::PUSH_INT
                        | Opcode::PUSH_STR
                        | Opcode::PUSH_BOOL
                        | Opcode::PUSH_PROC
                        | Opcode::PUSH_NAME
                        | Opcode::PUSH_NIL
                ) && matches!(second_op, Opcode::POP)
                {
                    push_pop_pairs += 1;
                }

                if matches!(first_op, Opcode::LOAD_LOCAL)
                    && matches!(second_op, Opcode::STORE_LOCAL)
                {
                    let first_operand = self.instructions[i].instruction.op16();
                    let second_operand = self.instructions[i + 1].instruction.op16();
                    if first_operand == second_operand {
                        redundant_loads += 1;
                    }
                }
            }
        }

        CompressionStats {
            total_instructions: self.instructions.len(),
            push_pop_pairs,
            redundant_loads,
            potential_savings: push_pop_pairs * 2 + redundant_loads * 2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompressionStats {
    pub total_instructions: usize,
    pub push_pop_pairs: usize,
    pub redundant_loads: usize,
    pub potential_savings: usize,
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

        let instructions = builder.build().unwrap();
        assert_eq!(instructions.len(), 3);
    }

    #[test]
    fn test_label_resolution_forward_jump() {
        let mut builder = InstructionBuilder::new();
        let forward_label = builder.create_label();

        // Jump forward to a label that hasn't been placed yet
        builder
            .emit(Instruction::nullary(Opcode::NOP)) // Position 0
            .emit_jump(forward_label) // Position 4
            .emit(Instruction::nullary(Opcode::NOP)) // Position 8
            .place_label(forward_label) // Position 12
            .emit(Instruction::nullary(Opcode::HALT)); // Position 12

        let instructions = builder.build().unwrap();
        assert_eq!(instructions.len(), 4);

        // Check that jump instruction has correct offset
        let jump_instruction = &instructions[1];
        assert_eq!(jump_instruction.instruction.opcode().unwrap(), Opcode::JUMP);

        // Offset should be 12 - 4 = 8 bytes (forward jump)
        assert_eq!(jump_instruction.instruction.op16() as i16, 8);

        // Check that extended data contains the label offset
        if let Some(InstructionData::Label(offset)) = &jump_instruction.data {
            assert_eq!(*offset, 8);
        } else {
            panic!("Expected label data");
        }
    }

    #[test]
    fn test_label_resolution_backward_jump() {
        let mut builder = InstructionBuilder::new();
        let backward_label = builder.create_label();

        // Place label first, then jump back to it
        builder
            .place_label(backward_label) // Position 0
            .emit(Instruction::nullary(Opcode::NOP)) // Position 0
            .emit(Instruction::nullary(Opcode::NOP)) // Position 4
            .emit_jump(backward_label) // Position 8
            .emit(Instruction::nullary(Opcode::HALT)); // Position 12

        let instructions = builder.build().unwrap();
        assert_eq!(instructions.len(), 4);

        // Check that jump instruction has correct offset
        let jump_instruction = &instructions[2];
        assert_eq!(jump_instruction.instruction.opcode().unwrap(), Opcode::JUMP);

        // Offset should be 0 - 8 = -8 bytes (backward jump)
        assert_eq!(jump_instruction.instruction.op16() as i16, -8);

        // Check that extended data contains the label offset
        if let Some(InstructionData::Label(offset)) = &jump_instruction.data {
            assert_eq!(*offset, -8);
        } else {
            panic!("Expected label data");
        }
    }

    #[test]
    fn test_all_branch_types() {
        let mut builder = InstructionBuilder::new();
        let label1 = builder.create_label();
        let label2 = builder.create_label();
        let label3 = builder.create_label();
        let label4 = builder.create_label();

        builder
            .emit_jump(label1) // Position 0
            .emit_branch_true(label2) // Position 4
            .emit_branch_false(label3) // Position 8
            .emit_branch_success(label4) // Position 12
            .place_label(label1) // Position 16
            .place_label(label2) // Position 16
            .place_label(label3) // Position 16
            .place_label(label4) // Position 16
            .emit(Instruction::nullary(Opcode::HALT)); // Position 16

        let instructions = builder.build().unwrap();
        assert_eq!(instructions.len(), 5);

        assert_eq!(instructions[0].instruction.opcode().unwrap(), Opcode::JUMP);
        assert_eq!(
            instructions[1].instruction.opcode().unwrap(),
            Opcode::BRANCH_TRUE
        );
        assert_eq!(
            instructions[2].instruction.opcode().unwrap(),
            Opcode::BRANCH_FALSE
        );
        assert_eq!(
            instructions[3].instruction.opcode().unwrap(),
            Opcode::BRANCH_SUCCESS
        );
        assert_eq!(instructions[4].instruction.opcode().unwrap(), Opcode::HALT);

        // All jumps should have offset 16 (from their respective positions)
        assert_eq!(instructions[0].instruction.op16() as i16, 16);
        assert_eq!(instructions[1].instruction.op16() as i16, 12);
        assert_eq!(instructions[2].instruction.op16() as i16, 8);
        assert_eq!(instructions[3].instruction.op16() as i16, 4);
    }

    #[test]
    fn test_unresolved_label_error() {
        let mut builder = InstructionBuilder::new();

        let unplaced_label = builder.create_label();
        builder.emit_jump(unplaced_label);

        let result = builder.build();
        assert!(result.is_err());

        match result.unwrap_err() {
            BytecodeError::UnresolvedLabel { label_id } => {
                assert_eq!(label_id, unplaced_label);
            }
            _ => panic!("Expected UnresolvedLabel error"),
        }
    }

    #[test]
    fn test_invalid_label_error() {
        let mut builder = InstructionBuilder::new();

        // Reference a label ID that was never created
        let invalid_label = 999;
        builder.emit_jump(invalid_label);

        let result = builder.build();
        assert!(result.is_err());

        match result.unwrap_err() {
            BytecodeError::InvalidLabel { label_id } => {
                assert_eq!(label_id, invalid_label);
            }
            _ => panic!("Expected InvalidLabel error"),
        }
    }

    #[test]
    fn test_complex_control_flow() {
        let mut builder = InstructionBuilder::new();
        let loop_start = builder.create_label();
        let loop_end = builder.create_label();
        let condition_true = builder.create_label();

        // Simulate: while (condition) { body }
        builder
            .place_label(loop_start) // Position 0
            .emit(Instruction::nullary(Opcode::PUSH_BOOL)) // Position 0 - push condition
            .emit_branch_false(loop_end) // Position 4 - exit if false
            .emit_branch_true(condition_true) // Position 8 - body if true
            .place_label(condition_true) // Position 12
            .emit(Instruction::nullary(Opcode::NOP)) // Position 12 - loop body
            .emit_jump(loop_start) // Position 16 - back to start
            .place_label(loop_end) // Position 20
            .emit(Instruction::nullary(Opcode::HALT)); // Position 20

        let instructions = builder.build().unwrap();
        assert_eq!(instructions.len(), 6);

        // branch_false at position 4 jumps to position 20: offset = 16
        assert_eq!(instructions[1].instruction.op16() as i16, 16);

        // branch_true at position 8 jumps to position 12: offset = 4
        assert_eq!(instructions[2].instruction.op16() as i16, 4);

        // jump at position 16 jumps to position 0: offset = -16
        assert_eq!(instructions[4].instruction.op16() as i16, -16);
    }

    #[test]
    fn test_instruction_compression() {
        let mut builder = InstructionBuilder::new();

        // Create a sequence with push-pop pairs that should be optimized
        builder
            .emit(Instruction::unary(Opcode::PUSH_INT, 42))
            .emit(Instruction::nullary(Opcode::POP))
            .emit(Instruction::unary(Opcode::LOAD_LOCAL, 5))
            .emit(Instruction::unary(Opcode::STORE_LOCAL, 5))
            .emit(Instruction::nullary(Opcode::HALT));

        // Get compression stats before optimization
        let stats_before = builder.compression_stats();
        assert_eq!(stats_before.push_pop_pairs, 1);
        assert_eq!(stats_before.redundant_loads, 1);
        assert_eq!(stats_before.potential_savings, 4); // 2 pairs * 2 instructions each

        let instructions = builder.build().unwrap();

        // After compression, we should have fewer instructions
        // Original: PUSH_INT, POP, LOAD_LOCAL, STORE_LOCAL, HALT (5 instructions)
        // Optimized: HALT (1 instruction) - both pairs eliminated
        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0].instruction.opcode().unwrap(), Opcode::HALT);
    }

    #[test]
    fn test_compression_stats() {
        let mut builder = InstructionBuilder::new();

        // Add various patterns
        builder
            .emit(Instruction::unary(Opcode::PUSH_STR, 1))
            .emit(Instruction::nullary(Opcode::POP))
            .emit(Instruction::unary(Opcode::PUSH_BOOL, 1))
            .emit(Instruction::nullary(Opcode::NOP))
            .emit(Instruction::unary(Opcode::LOAD_LOCAL, 10))
            .emit(Instruction::unary(Opcode::STORE_LOCAL, 10));

        let stats = builder.compression_stats();
        assert_eq!(stats.total_instructions, 6);
        assert_eq!(stats.push_pop_pairs, 1); // PUSH_STR + POP
        assert_eq!(stats.redundant_loads, 1); // LOAD_LOCAL + STORE_LOCAL same location
        assert_eq!(stats.potential_savings, 4);
    }

    #[test]
    fn test_emit_push_sequence() {
        let mut builder = InstructionBuilder::new();
        let values = [10, 20, 30];

        builder.emit_push_sequence(&values);

        let instructions = builder.instructions;
        assert_eq!(instructions.len(), 3);

        for (i, &expected_value) in values.iter().enumerate() {
            assert_eq!(
                instructions[i].instruction.opcode().unwrap(),
                Opcode::PUSH_INT
            );
            assert_eq!(instructions[i].instruction.op16(), expected_value);
        }
    }
}
