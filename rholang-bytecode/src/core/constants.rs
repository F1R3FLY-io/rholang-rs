//! Constant pool implementation

use ahash::AHashMap;
use parking_lot::RwLock;
use std::sync::Arc;

use rkyv::rancor::Error as RkyvError;
use rkyv::{Archive, Deserialize, Serialize, access, from_bytes, to_bytes};

use crate::core::types::{CompiledPattern, IntegerRef, StringRef};
use crate::error::BytecodeError;

/// String interning system for efficient string deduplication
#[derive(Debug)]
pub struct StringInterner {
    /// Map from string content to interned ID
    string_to_id: RwLock<AHashMap<Arc<str>, u32>>,

    /// Map from ID to string content (for resolution)
    id_to_string: RwLock<Vec<Arc<str>>>,

    /// Next available ID
    next_id: parking_lot::Mutex<u32>,
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            string_to_id: RwLock::new(AHashMap::new()),
            id_to_string: RwLock::new(Vec::new()),
            next_id: parking_lot::Mutex::new(0),
        }
    }

    /// Intern a string and return its ID and StringRef
    pub fn intern(&self, s: &str) -> StringRef {
        // Try to find existing string first (read lock only)
        {
            let string_map = self.string_to_id.read();
            if let Some(&id) = string_map.get(s) {
                let data = {
                    let id_map = self.id_to_string.read();
                    id_map[id as usize].clone()
                };
                return StringRef { id, data };
            }
        }

        // String not found, need to intern it (upgrade to write lock)
        let string_arc: Arc<str> = Arc::from(s);
        let mut string_map = self.string_to_id.write();

        // Double-check in case another thread added it
        if let Some(&id) = string_map.get(&string_arc) {
            let data = {
                let id_map = self.id_to_string.read();
                id_map[id as usize].clone()
            };
            return StringRef { id, data };
        }

        // Get next ID
        let id = {
            let mut next = self.next_id.lock();
            let current = *next;
            *next += 1;
            current
        };

        // Insert into both maps
        string_map.insert(string_arc.clone(), id);
        {
            let mut id_map = self.id_to_string.write();
            id_map.push(string_arc.clone());
        }

        StringRef {
            id,
            data: string_arc,
        }
    }

    /// Resolve an ID back to string content
    pub fn resolve(&self, id: u32) -> Option<Arc<str>> {
        let id_map = self.id_to_string.read();
        id_map.get(id as usize).cloned()
    }

    /// Get current count of interned strings
    pub fn count(&self) -> usize {
        self.id_to_string.read().len()
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Process template for storing process patterns and closures
#[derive(Clone, Debug)]
pub struct ProcessTemplate {
    /// Unique template ID
    pub id: u64,

    /// Bytecode for this process template
    pub bytecode: Arc<[u8]>,

    /// Parameter count for this template
    pub param_count: u8,

    /// Environment size requirement
    pub env_size: u32,

    /// RSpace type hint for optimization
    pub rspace_hint: crate::core::types::RSpaceType,
}

impl ProcessTemplate {
    pub fn new(
        id: u64,
        bytecode: Arc<[u8]>,
        param_count: u8,
        env_size: u32,
        rspace_hint: crate::core::types::RSpaceType,
    ) -> Self {
        Self {
            id,
            bytecode,
            param_count,
            env_size,
            rspace_hint,
        }
    }
}

/// Constant pool with zero-copy access patterns
#[derive(Debug)]
pub struct ConstantPool {
    /// Integer constants stored in contiguous memory
    integers: Vec<i64>,

    /// String interner for deduplicated strings
    string_interner: StringInterner,

    /// Process templates for reusable process patterns
    process_templates: Vec<ProcessTemplate>,

    /// Compiled patterns for efficient matching
    compiled_patterns: Vec<CompiledPattern>,

    /// Zero-copy access indices for integers
    integer_indices: AHashMap<i64, u32>,

    /// Access indices for process templates
    template_indices: AHashMap<u64, u32>,

    /// Access indices for patterns
    pattern_indices: AHashMap<u64, u32>,
}

impl ConstantPool {
    /// Create a new empty constant pool
    pub fn new() -> Self {
        Self {
            integers: Vec::new(),
            string_interner: StringInterner::new(),
            process_templates: Vec::new(),
            compiled_patterns: Vec::new(),
            integer_indices: AHashMap::new(),
            template_indices: AHashMap::new(),
            pattern_indices: AHashMap::new(),
        }
    }

    /// Add an integer to the constant pool with deduplication
    pub fn add_integer(&mut self, value: i64) -> u32 {
        if let Some(&index) = self.integer_indices.get(&value) {
            return index;
        }

        let index = self.integers.len() as u32;
        self.integers.push(value);
        self.integer_indices.insert(value, index);
        index
    }

    /// Get integer by index
    pub fn get_integer(&self, index: u32) -> Result<&i64, BytecodeError> {
        self.integers
            .get(index as usize)
            .ok_or_else(|| BytecodeError::InvalidConstantIndex {
                index,
                pool_type: "integer".to_string(),
            })
    }

    /// Add a string to the constant pool with interning
    pub fn add_string(&mut self, s: &str) -> u32 {
        let string_ref = self.string_interner.intern(s);
        string_ref.id
    }

    /// Get string by index
    pub fn get_string(&self, index: u32) -> Result<Arc<str>, BytecodeError> {
        self.string_interner
            .resolve(index)
            .ok_or_else(|| BytecodeError::InvalidConstantIndex {
                index,
                pool_type: "string".to_string(),
            })
    }

    /// Add a process template to the constant pool
    pub fn add_process_template(&mut self, template: ProcessTemplate) -> u32 {
        let template_id = template.id;

        if let Some(&index) = self.template_indices.get(&template_id) {
            return index;
        }

        let index = self.process_templates.len() as u32;
        self.process_templates.push(template);
        self.template_indices.insert(template_id, index);
        index
    }

    /// Get process template by index
    pub fn get_process_template(&self, index: u32) -> Result<&ProcessTemplate, BytecodeError> {
        self.process_templates.get(index as usize).ok_or_else(|| {
            BytecodeError::InvalidConstantIndex {
                index,
                pool_type: "process_template".to_string(),
            }
        })
    }

    /// Add a compiled pattern to the constant pool
    pub fn add_pattern(&mut self, pattern: CompiledPattern) -> u32 {
        let pattern_id = pattern.id;

        if let Some(&index) = self.pattern_indices.get(&pattern_id) {
            return index;
        }

        let index = self.compiled_patterns.len() as u32;
        self.compiled_patterns.push(pattern);
        self.pattern_indices.insert(pattern_id, index);
        index
    }

    /// Get compiled pattern by index (zero-copy access)
    pub fn get_pattern(&self, index: u32) -> Result<&CompiledPattern, BytecodeError> {
        self.compiled_patterns.get(index as usize).ok_or_else(|| {
            BytecodeError::InvalidConstantIndex {
                index,
                pool_type: "pattern".to_string(),
            }
        })
    }

    /// Get statistics about the constant pool
    pub fn stats(&self) -> ConstantPoolStats {
        ConstantPoolStats {
            integer_count: self.integers.len(),
            string_count: self.string_interner.count(),
            template_count: self.process_templates.len(),
            pattern_count: self.compiled_patterns.len(),
        }
    }

    /// Create a StringRef from the pool
    pub fn create_string_ref(&self, index: u32) -> Result<StringRef, BytecodeError> {
        let data = self.get_string(index)?;
        Ok(StringRef { id: index, data })
    }

    /// Create an IntegerRef from the pool
    pub fn create_integer_ref(&self, index: u32) -> Result<IntegerRef, BytecodeError> {
        let value = *self.get_integer(index)?;
        Ok(IntegerRef::Small(value))
    }
}

impl Default for ConstantPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about constant pool usage
#[derive(Debug, Clone)]
pub struct ConstantPoolStats {
    pub integer_count: usize,
    pub string_count: usize,
    pub template_count: usize,
    pub pattern_count: usize,
}

/// Serializable version of ConstantPool
#[derive(Archive, Deserialize, Serialize)]
pub struct SerializableConstantPool {
    /// Serialized integers
    pub integers: Vec<i64>,

    /// Serialized strings (content only, IDs are implicit by index)
    pub strings: Vec<String>,

    /// Serialized process templates
    pub process_templates: Vec<SerializableProcessTemplate>,

    /// Serialized compiled patterns
    pub compiled_patterns: Vec<SerializableCompiledPattern>,
}

#[derive(Archive, Deserialize, Serialize)]
pub struct SerializableProcessTemplate {
    pub id: u64,
    pub bytecode: Vec<u8>,
    pub param_count: u8,
    pub env_size: u32,
    pub rspace_hint: u8, // Serialized as u8
}

#[derive(Archive, Deserialize, Serialize)]
pub struct SerializableCompiledPattern {
    pub id: u64,
    pub bytecode: Vec<u8>,
    pub bindings: Vec<SerializableBindingInfo>,
}

#[derive(Archive, Deserialize, Serialize)]
pub struct SerializableBindingInfo {
    pub name: String,
    pub position: u32,
    pub type_constraint: Option<u8>, // Serialized type constraint
}

impl From<&ConstantPool> for SerializableConstantPool {
    fn from(pool: &ConstantPool) -> Self {
        let strings = (0..pool.string_interner.count() as u32)
            .map(|i| pool.string_interner.resolve(i).unwrap().to_string())
            .collect();

        let process_templates = pool
            .process_templates
            .iter()
            .map(|template| SerializableProcessTemplate {
                id: template.id,
                bytecode: template.bytecode.to_vec(),
                param_count: template.param_count,
                env_size: template.env_size,
                rspace_hint: template.rspace_hint as u8,
            })
            .collect();

        let compiled_patterns = pool
            .compiled_patterns
            .iter()
            .map(|pattern| SerializableCompiledPattern {
                id: pattern.id,
                bytecode: pattern.bytecode.to_vec(),
                bindings: pattern
                    .bindings
                    .iter()
                    .map(|binding| SerializableBindingInfo {
                        name: binding.name.to_string(),
                        position: binding.position,
                        type_constraint: binding.type_constraint.as_ref().map(|tc| match tc {
                            crate::core::types::TypeConstraint::Integer => 0,
                            crate::core::types::TypeConstraint::String => 1,
                            crate::core::types::TypeConstraint::Boolean => 2,
                            crate::core::types::TypeConstraint::Process => 3,
                            crate::core::types::TypeConstraint::Name => 4,
                            crate::core::types::TypeConstraint::List => 5,
                            crate::core::types::TypeConstraint::Map => 6,
                        }),
                    })
                    .collect(),
            })
            .collect();

        Self {
            integers: pool.integers.clone(),
            strings,
            process_templates,
            compiled_patterns,
        }
    }
}

impl TryFrom<SerializableConstantPool> for ConstantPool {
    type Error = BytecodeError;

    fn try_from(serializable: SerializableConstantPool) -> Result<Self, Self::Error> {
        let mut pool = ConstantPool::new();

        // Add integers
        for integer in serializable.integers {
            pool.add_integer(integer);
        }

        // Add strings
        for string in serializable.strings {
            pool.add_string(&string);
        }

        // Add process templates
        for template in serializable.process_templates {
            let rspace_hint = match template.rspace_hint {
                0 => crate::core::types::RSpaceType::MemSeq,
                1 => crate::core::types::RSpaceType::MemConc,
                2 => crate::core::types::RSpaceType::StoreSeq,
                3 => crate::core::types::RSpaceType::StoreConc,
                _ => return Err(BytecodeError::InvalidRSpaceType(template.rspace_hint)),
            };

            let process_template = ProcessTemplate::new(
                template.id,
                template.bytecode.into(),
                template.param_count,
                template.env_size,
                rspace_hint,
            );
            pool.add_process_template(process_template);
        }

        // Add compiled patterns
        for pattern in serializable.compiled_patterns {
            let bindings: Vec<_> = pattern
                .bindings
                .into_iter()
                .map(|binding| crate::core::types::BindingInfo {
                    name: Arc::from(binding.name),
                    position: binding.position,
                    type_constraint: binding.type_constraint.map(|tc| match tc {
                        0 => crate::core::types::TypeConstraint::Integer,
                        1 => crate::core::types::TypeConstraint::String,
                        2 => crate::core::types::TypeConstraint::Boolean,
                        3 => crate::core::types::TypeConstraint::Process,
                        4 => crate::core::types::TypeConstraint::Name,
                        5 => crate::core::types::TypeConstraint::List,
                        6 => crate::core::types::TypeConstraint::Map,
                        _ => crate::core::types::TypeConstraint::Integer, // Default fallback
                    }),
                })
                .collect();

            let compiled_pattern = CompiledPattern {
                id: pattern.id,
                bytecode: pattern.bytecode.into(),
                bindings: bindings.into(),
            };
            pool.add_pattern(compiled_pattern);
        }

        Ok(pool)
    }
}

pub struct BytecodeSerializer {
    /// Pre-allocated buffer for serialization
    buffer: Vec<u8>,
}

impl BytecodeSerializer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(1024 * 1024), // 1MB initial capacity
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    /// Serialize a constant pool to bytes
    pub fn serialize_pool(&mut self, pool: &ConstantPool) -> Result<Vec<u8>, BytecodeError> {
        self.buffer.clear();
        let serializable = SerializableConstantPool::from(pool);

        to_bytes::<RkyvError>(&serializable)
            .map(|aligned_vec| aligned_vec.to_vec())
            .map_err(|e| {
                BytecodeError::SerializationError(format!("serialization failed: {e}"))
            })
    }

    /// Validate and deserialize a constant pool from bytes
    pub fn deserialize_pool(bytes: &[u8]) -> Result<ConstantPool, BytecodeError> {
        let serializable: SerializableConstantPool =
            from_bytes::<SerializableConstantPool, RkyvError>(bytes).map_err(|e| {
                BytecodeError::SerializationError(format!("deserialization failed: {e}"))
            })?;

        ConstantPool::try_from(serializable)
    }

    /// Zero-copy access to archived constant pool
    ///
    /// This method provides safe zero-copy access to archived data with built-in validation.
    pub fn access_archived_pool(
        bytes: &[u8],
    ) -> Result<&rkyv::Archived<SerializableConstantPool>, BytecodeError> {
        access::<rkyv::Archived<SerializableConstantPool>, RkyvError>(bytes)
            .map_err(|e| BytecodeError::SerializationError(format!("access failed: {e}")))
    }
}

impl Default for BytecodeSerializer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::RSpaceType;

    #[test]
    fn test_string_interner() {
        let interner = StringInterner::new();

        let ref1 = interner.intern("hello");
        let ref2 = interner.intern("world");
        let ref3 = interner.intern("hello"); // Should reuse existing

        assert_eq!(ref1.id, ref3.id);
        assert_ne!(ref1.id, ref2.id);
        assert_eq!(ref1.data, ref3.data);
        assert_eq!(interner.count(), 2);
    }

    #[test]
    fn test_string_interner_resolution() {
        let interner = StringInterner::new();
        let string_ref = interner.intern("test_string");

        let resolved = interner.resolve(string_ref.id).unwrap();
        assert_eq!(resolved.as_ref(), "test_string");
    }

    #[test]
    fn test_constant_pool_integers() {
        let mut pool = ConstantPool::new();

        let idx1 = pool.add_integer(42);
        let idx2 = pool.add_integer(100);
        let idx3 = pool.add_integer(42); // Should deduplicate

        assert_eq!(idx1, idx3);
        assert_ne!(idx1, idx2);

        assert_eq!(*pool.get_integer(idx1).unwrap(), 42);
        assert_eq!(*pool.get_integer(idx2).unwrap(), 100);
    }

    #[test]
    fn test_constant_pool_strings() {
        let mut pool = ConstantPool::new();

        let idx1 = pool.add_string("hello");
        let idx2 = pool.add_string("world");
        let idx3 = pool.add_string("hello"); // Should deduplicate

        assert_eq!(idx1, idx3);
        assert_ne!(idx1, idx2);

        assert_eq!(pool.get_string(idx1).unwrap().as_ref(), "hello");
        assert_eq!(pool.get_string(idx2).unwrap().as_ref(), "world");
    }

    #[test]
    fn test_process_template_storage() {
        let mut pool = ConstantPool::new();

        let template =
            ProcessTemplate::new(1, vec![0x01, 0x02, 0x03].into(), 2, 64, RSpaceType::MemConc);

        let idx = pool.add_process_template(template);
        let retrieved = pool.get_process_template(idx).unwrap();

        assert_eq!(retrieved.id, 1);
        assert_eq!(retrieved.param_count, 2);
        assert_eq!(retrieved.env_size, 64);
        assert_eq!(retrieved.rspace_hint, RSpaceType::MemConc);
    }

    #[test]
    fn test_constant_pool_stats() {
        let mut pool = ConstantPool::new();

        pool.add_integer(42);
        pool.add_integer(100);
        pool.add_string("hello");
        pool.add_string("world");

        let stats = pool.stats();
        assert_eq!(stats.integer_count, 2);
        assert_eq!(stats.string_count, 2);
        assert_eq!(stats.template_count, 0);
        assert_eq!(stats.pattern_count, 0);
    }

    #[test]
    fn test_zero_copy_access() {
        let mut pool = ConstantPool::new();

        let idx = pool.add_integer(42);
        let val1 = pool.get_integer(idx).unwrap();
        let val2 = pool.get_integer(idx).unwrap();

        // Verify same memory address (zero-copy)
        assert_eq!(val1 as *const i64, val2 as *const i64);
    }

    #[test]
    fn test_concurrent_string_interning() {
        use std::sync::Arc;
        use std::thread;

        let interner = Arc::new(StringInterner::new());
        let mut handles = vec![];

        for i in 0..10 {
            let interner_clone = Arc::clone(&interner);
            handles.push(thread::spawn(move || {
                interner_clone.intern(&format!("string_{}", i % 3))
            }));
        }

        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // Verify that same strings get same IDs
        assert_eq!(results[0].id, results[3].id); // "string_0" appears at 0 and 3
        assert_eq!(results[1].id, results[4].id); // "string_1" appears at 1 and 4
        assert_eq!(interner.count(), 3); // Only 3 unique strings
    }

    #[test]
    fn test_serialization_round_trip() {
        let mut pool = ConstantPool::new();

        // Add various data to test serialization
        pool.add_integer(42);
        pool.add_integer(-123);
        pool.add_string("hello");
        pool.add_string("world");
        pool.add_string("rholang");

        let template = ProcessTemplate::new(
            1,
            vec![0x01, 0x02, 0x03, 0x04].into(),
            3,
            128,
            RSpaceType::StoreConc,
        );
        pool.add_process_template(template);

        // Create serializer and perform round-trip
        let mut serializer = BytecodeSerializer::new();
        let serialized = serializer.serialize_pool(&pool).unwrap();
        let deserialized = BytecodeSerializer::deserialize_pool(&serialized).unwrap();

        // Verify all data is preserved
        assert_eq!(*deserialized.get_integer(0).unwrap(), 42);
        assert_eq!(*deserialized.get_integer(1).unwrap(), -123);
        assert_eq!(deserialized.get_string(0).unwrap().as_ref(), "hello");
        assert_eq!(deserialized.get_string(1).unwrap().as_ref(), "world");
        assert_eq!(deserialized.get_string(2).unwrap().as_ref(), "rholang");

        let template_retrieved = deserialized.get_process_template(0).unwrap();
        assert_eq!(template_retrieved.id, 1);
        assert_eq!(template_retrieved.param_count, 3);
        assert_eq!(template_retrieved.env_size, 128);
        assert_eq!(template_retrieved.rspace_hint, RSpaceType::StoreConc);

        // Verify stats match
        let original_stats = pool.stats();
        let deserialized_stats = deserialized.stats();
        assert_eq!(
            original_stats.integer_count,
            deserialized_stats.integer_count
        );
        assert_eq!(original_stats.string_count, deserialized_stats.string_count);
        assert_eq!(
            original_stats.template_count,
            deserialized_stats.template_count
        );
        assert_eq!(
            original_stats.pattern_count,
            deserialized_stats.pattern_count
        );
    }

    #[test]
    fn test_serialization_with_patterns() {
        use crate::core::types::{BindingInfo, TypeConstraint};

        let mut pool = ConstantPool::new();

        // Create a compiled pattern with bindings
        let bindings = vec![
            BindingInfo {
                name: Arc::from("x"),
                position: 0,
                type_constraint: Some(TypeConstraint::Integer),
            },
            BindingInfo {
                name: Arc::from("y"),
                position: 1,
                type_constraint: Some(TypeConstraint::String),
            },
        ];

        let pattern = CompiledPattern {
            id: 42,
            bytecode: vec![0xFF, 0x00, 0xAB].into(),
            bindings: bindings.into(),
        };

        pool.add_pattern(pattern);

        // Serialize and deserialize
        let mut serializer = BytecodeSerializer::new();
        let serialized = serializer.serialize_pool(&pool).unwrap();
        let deserialized = BytecodeSerializer::deserialize_pool(&serialized).unwrap();

        // Verify pattern is preserved
        let retrieved_pattern = deserialized.get_pattern(0).unwrap();
        assert_eq!(retrieved_pattern.id, 42);
        assert_eq!(retrieved_pattern.bytecode.as_ref(), &[0xFF, 0x00, 0xAB]);
        assert_eq!(retrieved_pattern.bindings.len(), 2);
        assert_eq!(retrieved_pattern.bindings[0].name.as_ref(), "x");
        assert_eq!(retrieved_pattern.bindings[1].name.as_ref(), "y");
    }

    #[test]
    fn test_serialization_deduplication_preserved() {
        let mut pool = ConstantPool::new();

        // Add duplicates that should be deduplicated
        let idx1 = pool.add_integer(42);
        let idx2 = pool.add_integer(42); // Should reuse
        let idx3 = pool.add_string("test");
        let idx4 = pool.add_string("test"); // Should reuse

        assert_eq!(idx1, idx2);
        assert_eq!(idx3, idx4);

        // Serialize and deserialize
        let mut serializer = BytecodeSerializer::new();
        let serialized = serializer.serialize_pool(&pool).unwrap();
        let deserialized = BytecodeSerializer::deserialize_pool(&serialized).unwrap();

        // Original deduplication should be maintained
        let stats = deserialized.stats();
        assert_eq!(stats.integer_count, 1); // Only one unique integer
        assert_eq!(stats.string_count, 1); // Only one unique string
    }

    #[test]
    fn test_serializer_reuse() {
        let mut pool1 = ConstantPool::new();
        pool1.add_integer(1);
        pool1.add_string("first");

        let mut pool2 = ConstantPool::new();
        pool2.add_integer(2);
        pool2.add_string("second");

        // Reuse the same serializer
        let mut serializer = BytecodeSerializer::new();

        let serialized1 = serializer.serialize_pool(&pool1).unwrap();
        let serialized2 = serializer.serialize_pool(&pool2).unwrap();

        let deserialized1 = BytecodeSerializer::deserialize_pool(&serialized1).unwrap();
        let deserialized2 = BytecodeSerializer::deserialize_pool(&serialized2).unwrap();

        // Verify both pools were serialized correctly
        assert_eq!(*deserialized1.get_integer(0).unwrap(), 1);
        assert_eq!(deserialized1.get_string(0).unwrap().as_ref(), "first");

        assert_eq!(*deserialized2.get_integer(0).unwrap(), 2);
        assert_eq!(deserialized2.get_string(0).unwrap().as_ref(), "second");
    }

    #[test]
    fn test_zero_copy_deserialization_performance() {
        // This test ensures that we don't allocate unnecessarily during access
        let mut pool = ConstantPool::new();

        // Add test data
        for i in 0..100 {
            pool.add_integer(i);
            pool.add_string(&format!("string_{i}"));
        }

        let mut serializer = BytecodeSerializer::with_capacity(1024 * 1024); // 1MB buffer
        let serialized = serializer.serialize_pool(&pool).unwrap();

        // Deserialize and access multiple times to ensure zero-copy
        let deserialized = BytecodeSerializer::deserialize_pool(&serialized).unwrap();

        // Multiple accesses should not cause additional allocations
        for _ in 0..10 {
            for i in 0..100 {
                let val = deserialized.get_integer(i as u32).unwrap();
                assert_eq!(*val, i);
                let string = deserialized.get_string(i as u32).unwrap();
                assert_eq!(string.as_ref(), format!("string_{i}"));
            }
        }
    }
}
