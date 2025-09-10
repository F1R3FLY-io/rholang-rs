pub mod instructions;
pub mod module;
pub mod constants;
pub mod opcodes;
pub mod types;
// pub mod patterns;
// pub mod continuations;
// pub mod metadata;

// Re-export core types
pub use self::instructions::Instruction;
pub use self::module::{
    BytecodeModule, BytecodeModuleStats, MmapVec, OptimizationLevel, PatternPool, PatternPoolStats,
    ReferenceTable, ReferenceTableStats, ReferenceType,
};
pub use self::constants::{
    BytecodeSerializer, ConstantPool, ProcessTemplate, SerializableConstantPool, StringInterner,
};
pub use self::opcodes::Opcode;
pub use self::types::{Key, NameRef, ProcessRef, TypeRef, Value};
