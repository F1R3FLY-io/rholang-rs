//! BytecodeModule implementation

use crate::core::constants::{ConstantPool, StringInterner};
use crate::core::instructions::{ExtendedInstruction, Instruction};
use crate::core::types::{CompiledPattern, RSpaceType};
use crate::error::{BytecodeError, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Memory-mapped vector for zero-copy instruction access
/// Uses Arc<RwLock<Vec<T>>> for Phase 1 - provides Vec-like API with zero-copy sharing
#[derive(Clone, Debug)]
pub struct MmapVec<T> {
    /// Instructions stored in contiguous memory for cache efficiency
    data: Arc<RwLock<Vec<T>>>,

    /// Capacity for pre-allocation optimization
    capacity: usize,
}

impl<T> MmapVec<T> {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(Vec::new())),
            capacity: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Arc::new(RwLock::new(Vec::with_capacity(capacity))),
            capacity,
        }
    }

    /// Push an item to the vector
    pub fn push(&self, item: T) {
        self.data.write().push(item);
    }

    /// Get item by index (zero-copy access)
    pub fn get(&self, index: usize) -> Option<T>
    where
        T: Clone,
    {
        self.data.read().get(index).cloned()
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.data.read().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.read().is_empty()
    }

    /// Get slice for batch operations (requires read lock)
    pub fn with_slice<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[T]) -> R,
    {
        let data = self.data.read();
        f(&data)
    }

    /// Reserve capacity
    pub fn reserve(&self, additional: usize) {
        self.data.write().reserve(additional);
    }

    /// Get initial capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl<T> Default for MmapVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Reference table for zero-copy operation metadata
#[derive(Debug)]
pub struct ReferenceTable {
    /// Reference metadata indexed by reference ID
    references: RwLock<HashMap<u64, ReferenceMetadata>>,

    /// Next available reference ID
    next_ref_id: parking_lot::Mutex<u64>,

    /// Type-specific reference pools for optimization
    type_pools: RwLock<HashMap<ReferenceType, Vec<u64>>>,
}

#[derive(Debug, Clone)]
pub struct ReferenceMetadata {
    pub ref_type: ReferenceType,
    pub size_hint: usize,
    pub access_count: usize,
    pub last_accessed: std::time::Instant,
    pub is_shared: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceType {
    Process,
    Name,
    Pattern,
    String,
    Environment,
}

impl ReferenceTable {
    pub fn new() -> Self {
        Self {
            references: RwLock::new(HashMap::new()),
            next_ref_id: parking_lot::Mutex::new(1),
            type_pools: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_reference(
        &self,
        ref_type: ReferenceType,
        size_hint: usize,
        is_shared: bool,
    ) -> u64 {
        let ref_id = {
            let mut next_id = self.next_ref_id.lock();
            let current = *next_id;
            *next_id += 1;
            current
        };

        let metadata = ReferenceMetadata {
            ref_type,
            size_hint,
            access_count: 0,
            last_accessed: std::time::Instant::now(),
            is_shared,
        };

        {
            let mut references = self.references.write();
            references.insert(ref_id, metadata);
        }

        {
            let mut pools = self.type_pools.write();
            pools.entry(ref_type).or_default().push(ref_id);
        }

        ref_id
    }

    pub fn access_reference(&self, ref_id: u64) -> Option<ReferenceMetadata> {
        let mut references = self.references.write();
        if let Some(metadata) = references.get_mut(&ref_id) {
            metadata.access_count += 1;
            metadata.last_accessed = std::time::Instant::now();
            Some(metadata.clone())
        } else {
            None
        }
    }

    pub fn remove_reference(&self, ref_id: u64) -> bool {
        let metadata = {
            let mut references = self.references.write();
            references.remove(&ref_id)
        };

        if let Some(metadata) = metadata {
            let mut pools = self.type_pools.write();
            if let Some(pool) = pools.get_mut(&metadata.ref_type) {
                pool.retain(|&id| id != ref_id);
            }
            true
        } else {
            false
        }
    }

    pub fn get_references_by_type(&self, ref_type: ReferenceType) -> Vec<u64> {
        self.type_pools
            .read()
            .get(&ref_type)
            .cloned()
            .unwrap_or_default()
    }

    pub fn stats(&self) -> ReferenceTableStats {
        let references = self.references.read();
        let pools = self.type_pools.read();

        let mut type_counts = HashMap::new();
        for metadata in references.values() {
            *type_counts.entry(metadata.ref_type).or_insert(0) += 1;
        }

        ReferenceTableStats {
            total_references: references.len(),
            type_counts,
            pool_sizes: pools.iter().map(|(k, v)| (*k, v.len())).collect(),
        }
    }
}

impl Default for ReferenceTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about reference table usage
#[derive(Debug, Clone)]
pub struct ReferenceTableStats {
    pub total_references: usize,
    pub type_counts: HashMap<ReferenceType, usize>,
    pub pool_sizes: HashMap<ReferenceType, usize>,
}

/// Pattern pool for efficient pattern storage and reuse
#[derive(Debug)]
pub struct PatternPool {
    /// Compiled patterns indexed by ID
    patterns: RwLock<HashMap<u64, Arc<CompiledPattern>>>,

    /// Pattern hash to ID mapping for deduplication
    pattern_hashes: RwLock<HashMap<u64, u64>>,

    /// Next available pattern ID
    next_pattern_id: parking_lot::Mutex<u64>,

    /// Pattern access frequency for optimization
    access_counts: RwLock<HashMap<u64, usize>>,
}

impl PatternPool {
    pub fn new() -> Self {
        Self {
            patterns: RwLock::new(HashMap::new()),
            pattern_hashes: RwLock::new(HashMap::new()),
            next_pattern_id: parking_lot::Mutex::new(1),
            access_counts: RwLock::new(HashMap::new()),
        }
    }

    /// Add a pattern to the pool with deduplication
    pub fn add_pattern(&self, pattern: CompiledPattern) -> u64 {
        let hash = self.calculate_pattern_hash(&pattern);

        // Check if pattern already exists
        {
            let hashes = self.pattern_hashes.read();
            if let Some(&existing_id) = hashes.get(&hash) {
                return existing_id;
            }
        }

        // Create new pattern entry
        let pattern_id = {
            let mut next_id = self.next_pattern_id.lock();
            let current = *next_id;
            *next_id += 1;
            current
        };

        let pattern_arc = Arc::new(pattern);

        // Store pattern
        {
            let mut patterns = self.patterns.write();
            patterns.insert(pattern_id, pattern_arc);
        }

        // Store hash mapping
        {
            let mut hashes = self.pattern_hashes.write();
            hashes.insert(hash, pattern_id);
        }

        // Initialize access count
        {
            let mut access_counts = self.access_counts.write();
            access_counts.insert(pattern_id, 0);
        }

        pattern_id
    }

    /// Get pattern by ID
    pub fn get_pattern(&self, pattern_id: u64) -> Option<Arc<CompiledPattern>> {
        let patterns = self.patterns.read();
        let pattern = patterns.get(&pattern_id).cloned();

        if pattern.is_some() {
            // Update access count
            let mut access_counts = self.access_counts.write();
            if let Some(count) = access_counts.get_mut(&pattern_id) {
                *count += 1;
            }
        }

        pattern
    }

    /// Remove pattern from pool
    pub fn remove_pattern(&self, pattern_id: u64) -> bool {
        let pattern = {
            let mut patterns = self.patterns.write();
            patterns.remove(&pattern_id)
        };

        if pattern.is_some() {
            // Remove from access counts
            let mut access_counts = self.access_counts.write();
            access_counts.remove(&pattern_id);

            // Note: We don't remove from pattern_hashes as the hash calculation
            // would require the pattern data. In a future implementation, this
            // could be optimized with reverse mapping.

            true
        } else {
            false
        }
    }

    /// Get pattern pool statistics
    pub fn stats(&self) -> PatternPoolStats {
        let patterns = self.patterns.read();
        let access_counts = self.access_counts.read();

        let total_access_count: usize = access_counts.values().sum();
        let avg_access_count = if !patterns.is_empty() {
            total_access_count as f64 / patterns.len() as f64
        } else {
            0.0
        };

        PatternPoolStats {
            pattern_count: patterns.len(),
            total_access_count,
            avg_access_count,
            next_pattern_id: *self.next_pattern_id.lock(),
        }
    }

    /// Calculate hash for pattern deduplication
    /// This is a simplified hash - future implementation would use a proper hash
    fn calculate_pattern_hash(&self, pattern: &CompiledPattern) -> u64 {
        // TODO!
        // Simple hash based on pattern ID and bytecode length
        // In a future implementation, this would hash the actual bytecode content
        pattern
            .id
            .wrapping_mul(31)
            .wrapping_add(pattern.bytecode.len() as u64)
    }
}

impl Default for PatternPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about pattern pool usage
#[derive(Debug, Clone)]
pub struct PatternPoolStats {
    pub pattern_count: usize,
    pub total_access_count: usize,
    pub avg_access_count: f64,
    pub next_pattern_id: u64,
}

/// Main bytecode module structure
#[derive(Debug)]
pub struct BytecodeModule {
    /// Memory-mapped instructions
    pub instructions: MmapVec<Instruction>,

    /// Constant pool for literals and templates
    pub constant_pool: ConstantPool,

    /// Pattern pool for efficient pattern storage
    pub pattern_pool: PatternPool,

    /// Reference table for zero-copy metadata
    pub reference_table: ReferenceTable,

    /// String interning for deduplication
    pub string_interning: StringInterner,

    /// Extended instructions with associated data
    extended_instructions: RwLock<Vec<ExtendedInstruction>>,

    /// Module metadata
    metadata: BytecodeModuleMetadata,
}

#[derive(Debug, Clone)]
pub struct BytecodeModuleMetadata {
    pub version: u32,
    pub created: std::time::SystemTime,
    pub rspace_hint: RSpaceType,
    pub optimization_level: OptimizationLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    None,
    Basic,
    Aggressive,
}

impl BytecodeModule {
    /// Create a new bytecode module
    pub fn new() -> Self {
        Self {
            instructions: MmapVec::new(),
            constant_pool: ConstantPool::new(),
            pattern_pool: PatternPool::new(),
            reference_table: ReferenceTable::new(),
            string_interning: StringInterner::new(),
            extended_instructions: RwLock::new(Vec::new()),
            metadata: BytecodeModuleMetadata {
                version: 1,
                created: std::time::SystemTime::now(),
                rspace_hint: RSpaceType::StoreConc, // Default to persistent concurrent
                optimization_level: OptimizationLevel::Basic,
            },
        }
    }

    /// Create a module with specified capacity for optimization
    pub fn with_capacity(instruction_capacity: usize) -> Self {
        Self {
            instructions: MmapVec::with_capacity(instruction_capacity),
            constant_pool: ConstantPool::new(),
            pattern_pool: PatternPool::new(),
            reference_table: ReferenceTable::new(),
            string_interning: StringInterner::new(),
            extended_instructions: RwLock::new(Vec::with_capacity(instruction_capacity)),
            metadata: BytecodeModuleMetadata {
                version: 1,
                created: std::time::SystemTime::now(),
                rspace_hint: RSpaceType::StoreConc,
                optimization_level: OptimizationLevel::Basic,
            },
        }
    }

    pub fn add_instruction(&self, instruction: Instruction) -> usize {
        self.instructions.push(instruction);
        self.instructions.len() - 1
    }

    pub fn add_extended_instruction(&self, extended: ExtendedInstruction) -> usize {
        let index = {
            let mut extended_instructions = self.extended_instructions.write();
            extended_instructions.push(extended.clone());
            extended_instructions.len() - 1
        };

        // Also add the base instruction
        self.instructions.push(extended.instruction);

        index
    }

    pub fn get_instruction(&self, index: usize) -> Option<Instruction> {
        self.instructions.get(index)
    }

    pub fn get_extended_instruction(&self, index: usize) -> Option<ExtendedInstruction> {
        self.extended_instructions.read().get(index).cloned()
    }

    pub fn instruction_count(&self) -> usize {
        self.instructions.len()
    }

    /// TODO!
    /// Execute optimization passes on the module
    pub fn optimize(&mut self, level: OptimizationLevel) -> Result<()> {
        self.metadata.optimization_level = level;

        match level {
            OptimizationLevel::None => {
                // No optimization
            }
            OptimizationLevel::Basic => {
                // Basic optimization: constant pool optimization
                // self.constant_pool.optimize();
                unimplemented!()
            }
            OptimizationLevel::Aggressive => {
                // Aggressive optimization: everything + instruction reordering
                // self.constant_pool.optimize();
                self.optimize_instruction_layout()?;
            }
        }

        Ok(())
    }

    /// Optimize instruction layout for better cache performance
    fn optimize_instruction_layout(&self) -> Result<()> {
        // This would implement instruction reordering, dead code elimination, etc.
        // For now, it's a placeholder
        Ok(())
    }

    /// Get comprehensive module statistics
    pub fn stats(&self) -> BytecodeModuleStats {
        BytecodeModuleStats {
            instruction_count: self.instructions.len(),
            extended_instruction_count: self.extended_instructions.read().len(),
            constant_pool_stats: self.constant_pool.stats(),
            pattern_pool_stats: self.pattern_pool.stats(),
            reference_table_stats: self.reference_table.stats(),
            string_count: self.string_interning.count(),
            metadata: self.metadata.clone(),
        }
    }

    /// Validate module integrity
    pub fn validate(&self) -> Result<()> {
        // Validate instructions
        self.instructions.with_slice(|instructions| {
            for (i, instruction) in instructions.iter().enumerate() {
                if let Err(_e) = instruction.validate() {
                    return Err(BytecodeError::InvalidInstruction {
                        offset: i * 4, // 4 bytes per instruction
                    });
                }
            }
            Ok(())
        })?;

        // Additional validation could be added here in the future
        Ok(())
    }
}

impl Default for BytecodeModule {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive statistics about the bytecode module
#[derive(Debug, Clone)]
pub struct BytecodeModuleStats {
    pub instruction_count: usize,
    pub extended_instruction_count: usize,
    pub constant_pool_stats: crate::core::constants::ConstantPoolStats,
    pub pattern_pool_stats: PatternPoolStats,
    pub reference_table_stats: ReferenceTableStats,
    pub string_count: usize,
    pub metadata: BytecodeModuleMetadata,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::opcodes::Opcode;

    #[test]
    fn test_mmap_vec_operations() {
        let vec = MmapVec::new();
        assert!(vec.is_empty());

        vec.push(42i32);
        vec.push(100i32);

        assert_eq!(vec.len(), 2);
        assert_eq!(vec.get(0), Some(42));
        assert_eq!(vec.get(1), Some(100));
        assert_eq!(vec.get(2), None);
    }

    #[test]
    fn test_reference_table() {
        let table = ReferenceTable::new();

        let ref1 = table.create_reference(ReferenceType::Process, 64, false);
        let ref2 = table.create_reference(ReferenceType::String, 32, true);

        // Test access
        let metadata = table.access_reference(ref1);
        assert!(metadata.is_some());
        assert_eq!(metadata.unwrap().access_count, 1);

        // Test type-based queries
        let process_refs = table.get_references_by_type(ReferenceType::Process);
        assert_eq!(process_refs.len(), 1);
        assert_eq!(process_refs[0], ref1);

        // Test removal
        assert!(table.remove_reference(ref2));
        assert!(!table.remove_reference(ref2)); // Should fail second time
    }

    #[test]
    fn test_pattern_pool_deduplication() {
        use crate::core::types::{BindingInfo, TypeConstraint};

        let pool = PatternPool::new();

        // Create identical patterns
        let bindings = vec![BindingInfo {
            name: Arc::from("x"),
            position: 0,
            type_constraint: Some(TypeConstraint::Integer),
        }];

        let pattern1 = CompiledPattern {
            id: 1,
            bytecode: vec![0x01, 0x02].into(),
            bindings: bindings.clone().into(),
        };

        let pattern2 = CompiledPattern {
            id: 1, // Same ID - should deduplicate
            bytecode: vec![0x01, 0x02].into(),
            bindings: bindings.into(),
        };

        let id1 = pool.add_pattern(pattern1);
        let id2 = pool.add_pattern(pattern2);

        // Should be deduplicated (same ID returned)
        assert_eq!(id1, id2);
        assert_eq!(pool.stats().pattern_count, 1);

        // Test access
        let retrieved = pool.get_pattern(id1);
        assert!(retrieved.is_some());
        assert_eq!(pool.stats().total_access_count, 1);
    }

    #[test]
    fn test_bytecode_module_integration() {
        let module = BytecodeModule::new();

        // Add some instructions
        let inst1 = Instruction::nullary(Opcode::NOP);
        let inst2 = Instruction::unary(Opcode::PUSH_INT, 42);

        let idx1 = module.add_instruction(inst1);
        let idx2 = module.add_instruction(inst2);

        assert_eq!(idx1, 0);
        assert_eq!(idx2, 1);
        assert_eq!(module.instruction_count(), 2);

        // Verify retrieval
        let retrieved1 = module.get_instruction(0);
        assert!(retrieved1.is_some());
        assert_eq!(retrieved1.unwrap().opcode().unwrap(), Opcode::NOP);

        // Test validation
        assert!(module.validate().is_ok());

        // Test statistics
        let stats = module.stats();
        assert_eq!(stats.instruction_count, 2);
        assert_eq!(stats.extended_instruction_count, 0);
    }

    #[test]
    fn test_module_with_capacity() {
        let module = BytecodeModule::with_capacity(1000);

        // Should start empty but with reserved capacity
        assert_eq!(module.instruction_count(), 0);

        // Add instructions up to capacity
        for i in 0..100 {
            let inst = Instruction::unary(Opcode::PUSH_INT, i as u16);
            module.add_instruction(inst);
        }

        assert_eq!(module.instruction_count(), 100);

        // Test optimization (only None level works for now)
        let mut mutable_module = module; // Move to make it mutable
        assert!(mutable_module.optimize(OptimizationLevel::None).is_ok());
        assert_eq!(
            mutable_module.metadata.optimization_level,
            OptimizationLevel::None
        );
    }

    #[test]
    fn test_comprehensive_stats() {
        let mut module = BytecodeModule::new();

        // Add some content to all components
        module.add_instruction(Instruction::nullary(Opcode::NOP));
        module.constant_pool.add_integer(42);
        module.string_interning.intern("test").unwrap();

        let stats = module.stats();
        assert_eq!(stats.instruction_count, 1);
        assert_eq!(stats.constant_pool_stats.integer_count, 1);
        assert_eq!(stats.string_count, 1);
        assert_eq!(stats.reference_table_stats.total_references, 0);
        assert_eq!(stats.pattern_pool_stats.pattern_count, 0);
    }
}
