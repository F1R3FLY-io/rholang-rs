use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::{Value, ConstantIndex};
use crate::errors::{BytecodeError, BytecodeResult};

/// Stores frequently used constants and provides index-based access
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstantPool {
    /// Vector of constants (indexed access)
    constants: Vec<ConstantEntry>,
    /// Map for fast lookup of existing constants (deduplication)
    lookup: HashMap<ConstantEntry, ConstantIndex>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstantEntry {
    /// String literal constants
    String(String),
    /// Integer literal constants  
    Integer(i64),
    /// Boolean literal constants
    Boolean(bool),
    /// URI literal constants
    Uri(String),
    /// ByteArray literal constants
    ByteArray(Vec<u8>),
    /// Variable/identifier names
    Identifier(String),
    /// Contract names
    ContractName(String),
    /// Method names
    MethodName(String),
    /// Type names (Bool, Int, String, etc.)
    TypeName(String),
    /// Full Value constants (for complex literals)
    Value(Value),
}

impl ConstantPool {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            lookup: HashMap::new(),
        }
    }
    
    pub fn add_constant(&mut self, entry: ConstantEntry) -> ConstantIndex {
        if let Some(&index) = self.lookup.get(&entry) {
            return index;
        }
        
        let index = ConstantIndex::new(self.constants.len());
        self.constants.push(entry.clone());
        self.lookup.insert(entry, index);
        index
    }
    
    pub fn get_constant(&self, index: ConstantIndex) -> BytecodeResult<&ConstantEntry> {
        self.constants.get(index.get()).ok_or_else(|| {
            BytecodeError::IndexOutOfBounds {
                index: index.get(),
                max: self.constants.len(),
            }
        })
    }
    
    pub fn len(&self) -> usize {
        self.constants.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.constants.is_empty()
    }

    /// Helper methods for adding specific constant types
    pub fn add_string(&mut self, s: impl Into<String>) -> ConstantIndex {
        self.add_constant(ConstantEntry::String(s.into()))
    }
    
    pub fn add_integer(&mut self, i: i64) -> ConstantIndex {
        self.add_constant(ConstantEntry::Integer(i))
    }
    
    pub fn add_boolean(&mut self, b: bool) -> ConstantIndex {
        self.add_constant(ConstantEntry::Boolean(b))
    }
    
    pub fn add_uri(&mut self, uri: impl Into<String>) -> ConstantIndex {
        self.add_constant(ConstantEntry::Uri(uri.into()))
    }
    
    pub fn add_byte_array(&mut self, bytes: Vec<u8>) -> ConstantIndex {
        self.add_constant(ConstantEntry::ByteArray(bytes))
    }
    
    pub fn add_identifier(&mut self, name: impl Into<String>) -> ConstantIndex {
        self.add_constant(ConstantEntry::Identifier(name.into()))
    }
    
    pub fn add_contract_name(&mut self, name: impl Into<String>) -> ConstantIndex {
        self.add_constant(ConstantEntry::ContractName(name.into()))
    }
    
    pub fn add_method_name(&mut self, name: impl Into<String>) -> ConstantIndex {
        self.add_constant(ConstantEntry::MethodName(name.into()))
    }
    
    pub fn add_type_name(&mut self, name: impl Into<String>) -> ConstantIndex {
        self.add_constant(ConstantEntry::TypeName(name.into()))
    }
    
    pub fn add_value(&mut self, value: Value) -> ConstantIndex {
        self.add_constant(ConstantEntry::Value(value))
    }

    /// Helper methods for retrieving specific constant types
    pub fn get_string(&self, index: ConstantIndex) -> BytecodeResult<&str> {
        match self.get_constant(index)? {
            ConstantEntry::String(s) => Ok(s),
            entry => Err(BytecodeError::TypeError(
                format!("Expected string constant, got {:?}", entry)
            )),
        }
    }
    
    pub fn get_integer(&self, index: ConstantIndex) -> BytecodeResult<i64> {
        match self.get_constant(index)? {
            ConstantEntry::Integer(i) => Ok(*i),
            entry => Err(BytecodeError::TypeError(
                format!("Expected integer constant, got {:?}", entry)
            )),
        }
    }
    
    pub fn get_boolean(&self, index: ConstantIndex) -> BytecodeResult<bool> {
        match self.get_constant(index)? {
            ConstantEntry::Boolean(b) => Ok(*b),
            entry => Err(BytecodeError::TypeError(
                format!("Expected boolean constant, got {:?}", entry)
            )),
        }
    }
    
    pub fn get_uri(&self, index: ConstantIndex) -> BytecodeResult<&str> {
        match self.get_constant(index)? {
            ConstantEntry::Uri(uri) => Ok(uri),
            entry => Err(BytecodeError::TypeError(
                format!("Expected URI constant, got {:?}", entry)
            )),
        }
    }
    
    pub fn get_byte_array(&self, index: ConstantIndex) -> BytecodeResult<&[u8]> {
        match self.get_constant(index)? {
            ConstantEntry::ByteArray(bytes) => Ok(bytes),
            entry => Err(BytecodeError::TypeError(
                format!("Expected byte array constant, got {:?}", entry)
            )),
        }
    }
    
    pub fn get_identifier(&self, index: ConstantIndex) -> BytecodeResult<&str> {
        match self.get_constant(index)? {
            ConstantEntry::Identifier(name) => Ok(name),
            entry => Err(BytecodeError::TypeError(
                format!("Expected identifier constant, got {:?}", entry)
            )),
        }
    }
    
    pub fn get_value(&self, index: ConstantIndex) -> BytecodeResult<&Value> {
        match self.get_constant(index)? {
            ConstantEntry::Value(value) => Ok(value),
            entry => Err(BytecodeError::TypeError(
                format!("Expected value constant, got {:?}", entry)
            )),
        }
    }
    
    pub fn clear(&mut self) {
        self.constants.clear();
        self.lookup.clear();
    }
    
    pub fn constants(&self) -> &[ConstantEntry] {
        &self.constants
    }
}

impl Default for ConstantPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstantEntry {
    pub fn type_name(&self) -> &'static str {
        match self {
            ConstantEntry::String(_) => "String",
            ConstantEntry::Integer(_) => "Integer",
            ConstantEntry::Boolean(_) => "Boolean",
            ConstantEntry::Uri(_) => "Uri",
            ConstantEntry::ByteArray(_) => "ByteArray",
            ConstantEntry::Identifier(_) => "Identifier",
            ConstantEntry::ContractName(_) => "ContractName",
            ConstantEntry::MethodName(_) => "MethodName",
            ConstantEntry::TypeName(_) => "TypeName",
            ConstantEntry::Value(_) => "Value",
        }
    }
    
    pub fn to_value(&self) -> Value {
        match self {
            ConstantEntry::String(s) => Value::String(s.clone()),
            ConstantEntry::Integer(i) => Value::Int(*i),
            ConstantEntry::Boolean(b) => Value::Bool(*b),
            ConstantEntry::Uri(u) => Value::Uri(u.clone()),
            ConstantEntry::ByteArray(b) => Value::ByteArray(b.clone()),
            ConstantEntry::Identifier(name) => Value::VarRef {
                kind: crate::types::VarRefKind::Standard,
                name: name.clone(),
            },
            ConstantEntry::Value(v) => v.clone(),
            ConstantEntry::ContractName(name) => Value::String(name.clone()),
            ConstantEntry::MethodName(name) => Value::String(name.clone()),
            ConstantEntry::TypeName(name) => Value::String(name.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_pool_basic_operations() {
        let mut pool = ConstantPool::new();
        
        let str_idx = pool.add_string("hello");
        let int_idx = pool.add_integer(42);
        let bool_idx = pool.add_boolean(true);

        assert_eq!(pool.len(), 3);
        
        assert_eq!(pool.get_string(str_idx).unwrap(), "hello");
        assert_eq!(pool.get_integer(int_idx).unwrap(), 42);
        assert_eq!(pool.get_boolean(bool_idx).unwrap(), true);
    }

    #[test]
    fn test_constant_deduplication() {
        let mut pool = ConstantPool::new();

        // Add the same string twice
        let idx1 = pool.add_string("hello");
        let idx2 = pool.add_string("hello");

        // Should get the same index
        assert_eq!(idx1, idx2);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_constant_type_errors() {
        let mut pool = ConstantPool::new();
        let str_idx = pool.add_string("hello");
        
        assert!(pool.get_integer(str_idx).is_err());
    }

    #[test]
    fn test_index_out_of_bounds() {
        let pool = ConstantPool::new();
        let invalid_idx = ConstantIndex::new(999);

        assert!(pool.get_constant(invalid_idx).is_err());
    }

    #[test]
    fn test_constant_entry_to_value() {
        let entry = ConstantEntry::String("test".to_string());
        let value = entry.to_value();

        assert_eq!(value, Value::String("test".to_string()));
        assert_eq!(entry.type_name(), "String");
    }
}
