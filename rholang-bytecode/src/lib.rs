//! Rholang Bytecode Implementation
//!
//! High-performance bytecode layer for Rholang with zero-copy operations
//! and Graph-Structured Lambda Theory (GSLT) compliance.

#![warn(rust_2018_idioms)]
#![deny(unsafe_code)] // We'll use #[allow(unsafe_code)] only where necessary with safety proofs

pub mod core;
pub mod error;
// pub mod memory;

// Re-export commonly used types
pub use crate::core::{
    instructions::Instruction,
    // module::BytecodeModule,
    types::{Key, NameRef, ProcessRef, TypeRef, Value},
};

pub use crate::error::{BytecodeError, Result};

/// Version information for bytecode format
pub const BYTECODE_VERSION_MAJOR: u16 = 1;
pub const BYTECODE_VERSION_MINOR: u16 = 0;
pub const BYTECODE_VERSION_PATCH: u16 = 0;

/// Magic number for bytecode files (ASCII "RHBC")
pub const BYTECODE_MAGIC: [u8; 4] = [0x52, 0x48, 0x42, 0x43];
