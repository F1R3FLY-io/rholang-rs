use serde::{Deserialize, Serialize};
use crate::errors::{BytecodeError, BytecodeResult};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Value {
    /// Unit/Nil value
    Nil,
    /// Boolean values
    Bool(bool),
    /// Integer values
    Int(i64),
    /// String values (literals in grammar)
    String(String),
    /// URI values (literals in grammar)
    Uri(String),
    /// ByteArray values (simple_type in grammar)
    ByteArray(Vec<u8>),
    /// Wildcard value ('_' in grammar)
    Wildcard,
    /// Variable reference (var_ref in grammar)
    VarRef { kind: VarRefKind, name: String },
    /// Unforgeable names (from New instruction)
    Name(UnforgeableName),
    /// Quoted processes
    Quote(QuotedValue),
    /// List values
    List(Vec<Value>),
    /// Tuple values
    Tuple(Vec<Value>),
    /// Set values
    Set(Vec<Value>),
    /// Map values
    Map(Vec<(String, Value)>),
    /// Bundle values
    Bundle(BundleValue),
    /// Contract values
    Contract(ContractValue),
}

/// Unforgeable names in Rholang (from New instruction)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnforgeableName {
    pub id: Vec<u8>,
}

/// Quoted values for Rholang quoting
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QuotedValue {
    Process(String),
    Name(UnforgeableName),
}

/// Bundle types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BundleValue {
    Read(Box<Value>),
    Write(Box<Value>),
    Equiv(Box<Value>),
    ReadWrite(Box<Value>),
}

/// Contract definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContractValue {
    pub name: String,
    pub formals: Vec<String>,
    pub body: String,
}

/// Variable reference kinds
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VarRefKind {
    Standard,    // '='
    Wildcard,    // '=*'
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Nil | Value::Wildcard => false,
            Value::List(l) | Value::Tuple(l) | Value::Set(l) => !l.is_empty(),
            Value::Map(m) => !m.is_empty(),
            Value::String(s) | Value::Uri(s) => !s.is_empty(),
            Value::ByteArray(b) => !b.is_empty(),
            _ => true,
        }
    }
    
    pub fn as_int(&self) -> BytecodeResult<i64> {
        match self {
            Value::Int(i) => Ok(*i),
            _ => Err(BytecodeError::TypeError(
                format!("Cannot convert {} to integer", self.type_name())
            )),
        }
    }
    
    pub fn as_bool(&self) -> BytecodeResult<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(BytecodeError::TypeError(
                format!("Cannot convert {} to boolean", self.type_name())
            )),
        }
    }
    
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "Nil",
            Value::Bool(_) => "Bool",
            Value::Int(_) => "Int",
            Value::String(_) => "String",
            Value::Uri(_) => "Uri",
            Value::ByteArray(_) => "ByteArray",
            Value::Wildcard => "Wildcard",
            Value::VarRef { .. } => "VarRef",
            Value::Name(_) => "Name",
            Value::Quote(_) => "Quote",
            Value::List(_) => "List",
            Value::Tuple(_) => "Tuple",
            Value::Set(_) => "Set",
            Value::Map(_) => "Map",
            Value::Bundle(_) => "Bundle",
            Value::Contract(_) => "Contract",
        }
    }
}

impl UnforgeableName {
    pub fn new(id: Vec<u8>) -> Self {
        Self { id }
    }

    pub fn generate() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        Self::new(timestamp.to_be_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_creation_and_types() {
        assert_eq!(Value::Int(42).type_name(), "Int");
        assert_eq!(Value::Bool(true).type_name(), "Bool");
        assert_eq!(Value::String("test".to_string()).type_name(), "String");
        assert_eq!(Value::Nil.type_name(), "Nil");
    }

    #[test]
    fn test_truthiness() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Int(1).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(!Value::Nil.is_truthy());
    }

    #[test]
    fn test_unforgeable_name_generation() {
        let name1 = UnforgeableName::generate();
        let name2 = UnforgeableName::generate();
        assert_ne!(name1.id, name2.id);
    }
}
