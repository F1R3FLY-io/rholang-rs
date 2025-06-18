use serde::{Deserialize, Serialize};

/// Index into the constant pool
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConstantIndex(pub usize);

/// Index for local variables (used in Match/MatchCase patterns)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocalIndex(pub usize);

/// Offset for bytecode jumps (used internally by Match/MatchCase)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BytecodeOffset(pub isize);

impl ConstantIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }
    
    pub fn get(&self) -> usize {
        self.0
    }
}

impl LocalIndex {
    pub fn new(index: usize) -> Self {
        Self(index)
    }
    
    pub fn get(&self) -> usize {
        self.0
    }
}

impl BytecodeOffset {
    pub fn new(offset: isize) -> Self {
        Self(offset)
    }
    
    pub fn get(&self) -> isize {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_index() {
        let index = ConstantIndex::new(42);
        assert_eq!(index.get(), 42);
    }

    #[test]
    fn test_local_index() {
        let index = LocalIndex::new(10);
        assert_eq!(index.get(), 10);
    }

    #[test]
    fn test_bytecode_offset() {
        let offset = BytecodeOffset::new(-5);
        assert_eq!(offset.get(), -5);
    }
}
