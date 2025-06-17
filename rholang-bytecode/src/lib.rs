//! Rholang Bytecode Library
//!
//! This crate provides functionality for representing and manipulating Rholang bytecode.
//! It implements the core data types and structures needed for bytecode representation.

pub mod name;
pub mod types;
pub mod value;

// Re-export commonly used types
pub use name::*;
pub use types::*;
pub use value::*;
