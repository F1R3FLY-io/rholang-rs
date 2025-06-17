//! Common types and data structures for Rholang bytecode.
//!
//! This module defines the core data types used throughout the bytecode representation.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A literal value in Rholang.
///
/// Literals are constant values that can be used directly in Rholang code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Literal {
    /// Integer literal
    Int(i64),
    /// Boolean literal
    Bool(bool),
    /// String literal
    String(String),
    /// URI literal
    Uri(String),
    /// Byte array literal
    ByteArray(Vec<u8>),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Int(i) => write!(f, "{}", i),
            Literal::Bool(b) => write!(f, "{}", b),
            Literal::String(s) => write!(f, "\"{}\"", s),
            Literal::Uri(u) => write!(f, "`{}`", u),
            Literal::ByteArray(bytes) => {
                write!(f, "0x")?;
                for byte in bytes {
                    write!(f, "{:02x}", byte)?;
                }
                Ok(())
            }
        }
    }
}

/// A constant in Rholang bytecode.
///
/// Constants are values that are known at compile time and can be embedded directly
/// in the bytecode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Constant {
    /// Literal constant
    Literal(Literal),
    /// Tuple constant
    Tuple(Vec<Constant>),
    /// List constant
    List(Vec<Constant>),
    /// Map constant (key-value pairs)
    Map(Vec<(Constant, Constant)>),
}

impl fmt::Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Literal(lit) => write!(f, "{}", lit),
            Constant::Tuple(elements) => {
                write!(f, "(")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
            Constant::List(elements) => {
                write!(f, "[")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Constant::Map(entries) => {
                write!(f, "{{")?;
                for (i, (key, value)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
        }
    }
}
