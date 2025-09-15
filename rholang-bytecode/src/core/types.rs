//! Core type system

use parking_lot::RwLock;
use std::sync::Arc;

/// Tagged pointer for efficient value representation
/// Uses the lower 3 bits for type tagging (8-byte aligned pointers)
#[derive(Clone)]
pub struct TaggedPtr<T> {
    ptr: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TaggedPtr<T> {
    const TAG_MASK: usize = 0b111;
    const PTR_MASK: usize = !0b111;

    pub fn new(ptr: *const T, tag: u8) -> Self {
        debug_assert!(tag <= 7, "Tag must fit in 3 bits");
        debug_assert_eq!(
            ptr as usize & Self::TAG_MASK,
            0,
            "Pointer must be 8-byte aligned"
        );

        Self {
            ptr: (ptr as usize) | (tag as usize),
            _phantom: std::marker::PhantomData,
        }
    }

    #[inline]
    pub fn tag(&self) -> u8 {
        (self.ptr & Self::TAG_MASK) as u8
    }

    #[inline]
    pub fn as_ptr(&self) -> *const T {
        (self.ptr & Self::PTR_MASK) as *const T
    }

    /// Safe dereference with tag check
    #[inline]
    #[allow(unsafe_code)] // Necessary for tagged pointer implementation with safety proof
    pub fn get(&self, expected_tag: u8) -> Option<&T> {
        if self.tag() == expected_tag {
            // SAFETY: We ensure the pointer is valid when creating TaggedPtr
            // and the pointer is 8-byte aligned as asserted in `new`
            unsafe { self.as_ptr().as_ref() }
        } else {
            None
        }
    }
}

// Ensure TaggedPtr is Send + Sync when T is
#[allow(unsafe_code)] // Necessary for Send/Sync implementation with safety proof
unsafe impl<T: Send + Sync> Send for TaggedPtr<T> {}
#[allow(unsafe_code)] // Necessary for Send/Sync implementation with safety proof  
unsafe impl<T: Send + Sync> Sync for TaggedPtr<T> {}

/// Process reference
#[derive(Clone, Debug)]
pub struct ProcessRef {
    /// Shared reference to process bytecode
    inner: Arc<ProcessData>,
}

#[derive(Clone, Debug)]
pub struct ProcessData {
    /// Process ID for debugging and tracing
    pub id: u64,

    pub bytecode_offset: u32,

    pub bytecode_length: u32,

    /// Captured environment (if closure)
    pub environment: Option<Arc<Environment>>,

    /// RSpace type hint for optimization
    pub rspace_hint: RSpaceType,
}

impl ProcessRef {
    pub fn new(id: u64, offset: u32, length: u32, rspace_hint: RSpaceType) -> Self {
        Self {
            inner: Arc::new(ProcessData {
                id,
                bytecode_offset: offset,
                bytecode_length: length,
                environment: None,
                rspace_hint,
            }),
        }
    }

    /// Create a closure with captured environment
    pub fn with_environment(mut self, env: Arc<Environment>) -> Self {
        // Use Arc::make_mut for COW semantics
        Arc::make_mut(&mut self.inner).environment = Some(env);
        self
    }

    /// Get reference count (for debugging)
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.inner)
    }

    /// Get the process ID
    pub fn id(&self) -> u64 {
        self.inner.id
    }
}

/// Name reference (channels and quoted processes)
#[derive(Clone, Debug)]
pub struct NameRef {
    inner: Arc<NameData>,
}

#[derive(Clone, Debug)]
pub enum NameData {
    /// Unforgeable name created by 'new' operation
    Unforgeable {
        hash: [u8; 32],
        creation_context: u64,
    },

    /// Quoted process (@P)
    Quoted { process: ProcessRef },

    /// System channel
    System { channel_id: u32, name: Arc<str> },

    /// URI-based name
    Uri { uri: Arc<str> },
}

impl NameRef {
    pub fn unforgeable(hash: [u8; 32], context: u64) -> Self {
        Self {
            inner: Arc::new(NameData::Unforgeable {
                hash,
                creation_context: context,
            }),
        }
    }

    pub fn quoted(process: ProcessRef) -> Self {
        Self {
            inner: Arc::new(NameData::Quoted { process }),
        }
    }

    pub fn is_unforgeable(&self) -> bool {
        matches!(*self.inner, NameData::Unforgeable { .. })
    }
}

/// Continuation for suspended computations
#[derive(Clone, Debug)]
pub struct ContinuationRef {
    #[allow(dead_code)]
    inner: Arc<ContinuationData>,
}

#[derive(Clone, Debug)]
pub struct ContinuationData {
    /// Continuation ID for RSpace storage
    pub id: u64,

    /// Process to resume
    pub process: ProcessRef,

    /// Captured environment
    pub environment: Arc<Environment>,

    /// Pattern for binding received data
    pub pattern: Option<Arc<CompiledPattern>>,

    /// Target RSpace for resumption
    pub rspace_type: RSpaceType,
}

/// RSpace type for channel operations
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum RSpaceType {
    MemSeq = 0,
    MemConc = 1,
    StoreSeq = 2,
    StoreConc = 3,
}

impl RSpaceType {
    pub fn is_persistent(&self) -> bool {
        matches!(self, RSpaceType::StoreSeq | RSpaceType::StoreConc)
    }

    pub fn is_concurrent(&self) -> bool {
        matches!(self, RSpaceType::MemConc | RSpaceType::StoreConc)
    }
}

/// Environment for captured variables
#[derive(Clone, Debug)]
pub struct Environment {
    /// Variable bindings (variable index -> value)
    bindings: Arc<RwLock<Vec<TypeRef>>>,

    /// Parent environment for lexical scoping
    parent: Option<Arc<Environment>>,
}

impl Environment {
    /// Maximum number of bindings allowed in an environment
    pub const MAX_BINDINGS: usize = 65536;

    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            bindings: Arc::new(RwLock::new(Vec::new())),
            parent: None,
        })
    }

    pub fn with_parent(parent: Arc<Environment>) -> Arc<Self> {
        Arc::new(Self {
            bindings: Arc::new(RwLock::new(Vec::new())),
            parent: Some(parent),
        })
    }

    pub fn bind(&self, index: usize, value: TypeRef) -> Result<(), crate::error::BytecodeError> {
        // Validate index bounds
        if index >= Self::MAX_BINDINGS {
            return Err(crate::error::BytecodeError::ResourceExhaustion {
                resource_type: "environment_bindings".to_string(),
                limit: Self::MAX_BINDINGS,
                current: index,
            });
        }

        let mut bindings = self.bindings.write();

        if index >= bindings.len() && index >= Self::MAX_BINDINGS {
            return Err(crate::error::BytecodeError::ResourceExhaustion {
                resource_type: "environment_bindings".to_string(),
                limit: Self::MAX_BINDINGS,
                current: index,
            });
        }

        if index >= bindings.len() {
            // Limit growth to prevent excessive memory allocation
            let new_size = (index + 1).min(Self::MAX_BINDINGS);
            bindings.resize(new_size, TypeRef::Nil);
        }

        if index < bindings.len() {
            bindings[index] = value;
        }

        Ok(())
    }

    pub fn lookup(&self, index: usize) -> Option<TypeRef> {
        let bindings = self.bindings.read();
        if index < bindings.len() {
            let value = &bindings[index];
            // If the value is not Nil, return it; otherwise check parent
            if !matches!(value, TypeRef::Nil) {
                Some(value.clone())
            } else if let Some(parent) = &self.parent {
                parent.lookup(index)
            } else {
                None
            }
        } else if let Some(parent) = &self.parent {
            parent.lookup(index)
        } else {
            None
        }
    }
}

/// Compiled pattern for efficient matching
#[derive(Clone, Debug)]
pub struct CompiledPattern {
    pub id: u64,
    pub bytecode: Arc<[u8]>,
    pub bindings: Arc<[BindingInfo]>,
}

#[derive(Clone, Debug)]
pub struct BindingInfo {
    pub name: Arc<str>,
    pub position: u32,
    pub type_constraint: Option<TypeConstraint>,
}

#[derive(Clone, Debug)]
pub enum TypeConstraint {
    Integer,
    String,
    Boolean,
    Process,
    Name,
    List,
    Map,
}

/// Key type for RSpace operations
#[derive(Clone, Debug)]
pub enum Key {
    /// Name reference
    Name(NameRef),

    /// Direct channel hash (32 bytes)
    Hash([u8; 32]),

    /// Stack-local channel reference (2 bytes)
    Local(u16),
}

/// Value type for channel data
#[derive(Clone, Debug)]
pub enum Value {
    /// Primitive types
    Integer(IntegerRef),
    String(StringRef),
    Boolean(bool),

    /// Collection types
    List(Arc<[Value]>),
    Tuple(Arc<[Value]>),
    Map(Arc<MapData>),

    /// Process calculus specific
    Process(ProcessRef),
    Name(NameRef),

    /// Nil value
    Nil,
}

/// Unified type reference
#[derive(Clone, Debug)]
pub enum TypeRef {
    /// Nil/Unit value
    Nil,

    /// Boolean value
    Boolean(bool),

    /// Integer (inline for small values, Arc for large)
    Integer(IntegerRef),

    /// String (interned for deduplication)
    String(StringRef),

    /// Process reference
    Process(ProcessRef),

    /// Name/Channel reference
    Name(NameRef),

    /// Continuation reference
    Continuation(ContinuationRef),

    /// List
    List(Arc<[TypeRef]>),

    /// Tuple
    Tuple(Arc<[TypeRef]>),

    /// Map
    Map(Arc<MapData>),

    /// Key for RSpace operations
    Key(Key),

    /// Value for channel data
    Value(Value),
}

#[derive(Clone, Debug)]
pub enum IntegerRef {
    /// Small integer (fits in 63 bits)
    Small(i64),

    /// Large integer (arbitrary precision)
    Large(Arc<Vec<u64>>),
}

/// String representation with interning
#[derive(Clone, Debug)]
pub struct StringRef {
    /// Interned string ID
    pub id: u32,

    /// Actual string data (shared)
    pub data: Arc<str>,
}

/// Map data structure
#[derive(Clone, Debug)]
pub struct MapData {
    /// Entries stored as sorted array for cache efficiency
    #[allow(dead_code)]
    entries: Vec<(TypeRef, TypeRef)>,
}

impl Key {
    pub fn key_type(&self) -> u8 {
        match self {
            Key::Name(_) => 0,
            Key::Hash(_) => 1,
            Key::Local(_) => 2,
        }
    }
}

impl Value {
    pub fn type_tag(&self) -> u8 {
        match self {
            Value::Nil => 0,
            Value::Boolean(_) => 1,
            Value::Integer(_) => 2,
            Value::String(_) => 3,
            Value::Process(_) => 4,
            Value::Name(_) => 5,
            Value::List(_) => 6,
            Value::Tuple(_) => 7,
            Value::Map(_) => 8,
        }
    }

    pub fn to_type_ref(&self) -> TypeRef {
        match self {
            Value::Nil => TypeRef::Nil,
            Value::Boolean(b) => TypeRef::Boolean(*b),
            Value::Integer(i) => TypeRef::Integer(i.clone()),
            Value::String(s) => TypeRef::String(s.clone()),
            Value::Process(p) => TypeRef::Process(p.clone()),
            Value::Name(n) => TypeRef::Name(n.clone()),
            Value::List(items) => {
                let type_items: Vec<TypeRef> = items.iter().map(|v| v.to_type_ref()).collect();
                TypeRef::List(type_items.into())
            }
            Value::Tuple(items) => {
                let type_items: Vec<TypeRef> = items.iter().map(|v| v.to_type_ref()).collect();
                TypeRef::Tuple(type_items.into())
            }
            Value::Map(_) => todo!("Map conversion not implemented"),
        }
    }
}

impl TypeRef {
    pub fn type_tag(&self) -> u8 {
        match self {
            TypeRef::Nil => 0,
            TypeRef::Boolean(_) => 1,
            TypeRef::Integer(_) => 2,
            TypeRef::String(_) => 3,
            TypeRef::Process(_) => 4,
            TypeRef::Name(_) => 5,
            TypeRef::Continuation(_) => 6,
            TypeRef::List(_) => 7,
            TypeRef::Tuple(_) => 8,
            TypeRef::Map(_) => 9,
            TypeRef::Key(_) => 10,
            TypeRef::Value(_) => 11,
        }
    }

    pub fn is_sendable(&self) -> bool {
        // All values except continuations can be sent
        !matches!(self, TypeRef::Continuation(_))
    }

    /// Convert to Value for RSpace operations
    pub fn to_value(&self) -> Option<Value> {
        match self {
            TypeRef::Nil => Some(Value::Nil),
            TypeRef::Boolean(b) => Some(Value::Boolean(*b)),
            TypeRef::Integer(i) => Some(Value::Integer(i.clone())),
            TypeRef::String(s) => Some(Value::String(s.clone())),
            TypeRef::Process(p) => Some(Value::Process(p.clone())),
            TypeRef::Name(n) => Some(Value::Name(n.clone())),
            TypeRef::List(items) => {
                let value_items: Option<Vec<Value>> = items.iter().map(|t| t.to_value()).collect();
                value_items.map(|items| Value::List(items.into()))
            }
            TypeRef::Tuple(items) => {
                let value_items: Option<Vec<Value>> = items.iter().map(|t| t.to_value()).collect();
                value_items.map(|items| Value::Tuple(items.into()))
            }
            TypeRef::Map(_) => todo!("Map conversion not implemented"),
            TypeRef::Key(_) => None, // Keys are not values
            TypeRef::Value(v) => Some(v.clone()),
            TypeRef::Continuation(_) => None, // Continuations cannot be converted to values
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tagged_pointer() {
        let value = 42u64;
        let ptr = &value as *const u64;
        let tagged = TaggedPtr::new(ptr, 3);

        assert_eq!(tagged.tag(), 3);
        assert_eq!(tagged.as_ptr(), ptr);
        assert_eq!(tagged.get(3), Some(&value));
        assert_eq!(tagged.get(4), None);
    }

    #[test]
    fn test_process_ref_sharing() {
        let proc1 = ProcessRef::new(1, 0, 100, RSpaceType::MemSeq);
        let proc2 = proc1.clone();

        assert_eq!(proc1.ref_count(), 2);
        assert_eq!(proc2.ref_count(), 2);

        // Verify zero-copy: same Arc
        assert_eq!(
            Arc::as_ptr(&proc1.inner) as usize,
            Arc::as_ptr(&proc2.inner) as usize
        );
    }

    #[test]
    fn test_environment_binding() {
        let env = Environment::new();
        let value = TypeRef::Integer(IntegerRef::Small(42));

        env.bind(0, value.clone()).unwrap();
        assert!(matches!(env.lookup(0), Some(TypeRef::Integer(_))));
        assert!(env.lookup(1).is_none());
    }

    #[test]
    fn test_environment_bounds_checking() {
        let env = Environment::new();
        let value = TypeRef::Integer(IntegerRef::Small(42));

        // Test that exceeding MAX_BINDINGS fails
        let result = env.bind(Environment::MAX_BINDINGS, value);
        assert!(result.is_err());

        // Test that binding within bounds works
        let result = env.bind(100, TypeRef::Boolean(true));
        assert!(result.is_ok());
    }

    #[test]
    fn test_rspace_type_properties() {
        assert!(RSpaceType::StoreConc.is_persistent());
        assert!(RSpaceType::StoreConc.is_concurrent());
        assert!(!RSpaceType::MemSeq.is_persistent());
        assert!(!RSpaceType::MemSeq.is_concurrent());
    }
}
