//! Value types for Rholang bytecode.
//!
//! This module defines the Value type, which represents any Rholang value that can
//! be manipulated at runtime.

use crate::name::Name;
use crate::types::{Constant, Literal};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A Rholang value.
///
/// Values are the runtime representation of data in Rholang. They can be
/// literals, data structures, or more complex entities like processes or names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// A literal value (integer, boolean, string, etc.)
    Literal(Literal),
    /// A tuple of values
    Tuple(Vec<Value>),
    /// A list of values
    List(Vec<Value>),
    /// A map of key-value pairs
    Map(Vec<(Value, Value)>),
    /// A Rholang name
    Name(Name),
    /// A quoted process (not yet implemented)
    QuotedProcess(Box<QuotedProcess>),
}

impl Value {
    /// Creates a new integer value
    pub fn int(value: i64) -> Self {
        Value::Literal(Literal::Int(value))
    }

    /// Creates a new boolean value
    pub fn bool(value: bool) -> Self {
        Value::Literal(Literal::Bool(value))
    }

    /// Creates a new string value
    pub fn string<S: Into<String>>(value: S) -> Self {
        Value::Literal(Literal::String(value.into()))
    }

    /// Creates a new URI value
    pub fn uri<S: Into<String>>(value: S) -> Self {
        Value::Literal(Literal::Uri(value.into()))
    }

    /// Creates a new byte array value
    pub fn bytes<B: Into<Vec<u8>>>(value: B) -> Self {
        Value::Literal(Literal::ByteArray(value.into()))
    }

    /// Creates a new tuple value
    pub fn tuple(values: Vec<Value>) -> Self {
        Value::Tuple(values)
    }

    /// Creates a new list value
    pub fn list(values: Vec<Value>) -> Self {
        Value::List(values)
    }

    /// Creates a new map value
    pub fn map(entries: Vec<(Value, Value)>) -> Self {
        Value::Map(entries)
    }

    /// Converts a constant to a value
    pub fn from_constant(constant: Constant) -> Self {
        match constant {
            Constant::Literal(lit) => Value::Literal(lit),
            Constant::Tuple(elements) => {
                Value::Tuple(elements.into_iter().map(Value::from_constant).collect())
            }
            Constant::List(elements) => {
                Value::List(elements.into_iter().map(Value::from_constant).collect())
            }
            Constant::Map(entries) => Value::Map(
                entries
                    .into_iter()
                    .map(|(k, v)| (Value::from_constant(k), Value::from_constant(v)))
                    .collect(),
            ),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Literal(lit) => write!(f, "{}", lit),
            Value::Tuple(elements) => {
                write!(f, "(")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
            Value::List(elements) => {
                write!(f, "[")?;
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", elem)?;
                }
                write!(f, "]")
            }
            Value::Map(entries) => {
                write!(f, "{{")?;
                for (i, (key, value)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
            Value::Name(name) => write!(f, "{}", name),
            Value::QuotedProcess(proc) => write!(f, "@{{{}}}", proc),
        }
    }
}

/// A quoted process in Rholang.
///
/// Quoted processes are processes that have been reified as data.
/// This is a placeholder implementation that will be expanded in future phases.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotedProcess {
    // This is a placeholder for now
    // Will be expanded in future phases
    placeholder: String,
}

impl fmt::Display for QuotedProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<quoted process: {}>", self.placeholder)
    }
}
