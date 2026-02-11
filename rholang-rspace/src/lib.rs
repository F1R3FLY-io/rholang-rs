//! RSpace - Rholang's Tuple Space Storage with Dependency Injection.
//!
//! This crate provides the **single RSpace implementation** for Rholang execution
//! with built-in dependency injection for easy testing and implementation swapping.
//!
//! # Quick Start
//!
//! ```
//! use rholang_rspace::{RSpace, new_rspace, Value};
//!
//! // Create a new RSpace instance (uses default implementation)
//! let mut rspace = new_rspace();
//!
//! // Channel operations (FIFO queue)
//! rspace.tell("inbox", Value::Int(42)).unwrap();
//! assert_eq!(rspace.peek("inbox").unwrap(), Some(Value::Int(42)));
//! assert_eq!(rspace.ask("inbox").unwrap(), Some(Value::Int(42)));
//! ```
//!
//! # Dependency Injection Pattern
//!
//! RSpace uses a factory-based dependency injection pattern that allows:
//!
//! 1. **Build-time selection**: Choose implementation via Cargo features
//! 2. **Runtime injection**: Pass custom RSpace instances to components
//! 3. **Global singleton**: Shared RSpace for the entire application
//! 4. **Easy testing**: Mock implementations for unit tests
//!
//! ## Build-Time Selection (Features)
//!
//! ```toml
//! # In Cargo.toml
//!
//! # Use PathMap (default, production)
//! rholang-rspace = { path = "../rholang-rspace" }
//!
//! # Use InMemory (testing)
//! rholang-rspace = { path = "../rholang-rspace", default-features = false, features = ["inmemory-impl"] }
//! ```
//!
//! ## Runtime Injection
//!
//! ```
//! use rholang_rspace::{RSpace, new_rspace, new_shared_rspace, Value};
//!
//! // Create and pass RSpace to components
//! fn process_with_rspace(rspace: &mut dyn RSpace) {
//!     rspace.tell("result", Value::Int(42)).unwrap();
//! }
//!
//! let mut rspace = new_rspace();
//! process_with_rspace(rspace.as_mut());
//! ```
//!
//! ## Global Singleton
//!
//! ```
//! use rholang_rspace::{global_rspace, with_global_rspace, with_global_rspace_mut, Value};
//!
//! // Initialize once at application startup
//! rholang_rspace::init_global_rspace();
//!
//! // Use from anywhere
//! with_global_rspace_mut(|rspace| {
//!     rspace.tell("global_channel", Value::Int(42)).unwrap();
//! });
//!
//! with_global_rspace(|rspace| {
//!     assert_eq!(rspace.peek("global_channel").unwrap(), Some(Value::Int(42)));
//! });
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                           rholang-rspace                                    │
//! │                                                                             │
//! │  ┌───────────────────────────────────────────────────────────────────────┐ │
//! │  │                     Dependency Injection Layer                        │ │
//! │  │                                                                        │ │
//! │  │  Factory Functions:          Global Singleton:                         │ │
//! │  │  • new_rspace()              • init_global_rspace()                    │ │
//! │  │  • new_shared_rspace()       • with_global_rspace(|r| ...)             │ │
//! │  │  • shared_rspace(impl)       • with_global_rspace_mut(|r| ...)         │ │
//! │  │  • new_rspace_with::<T>()    • reset_global_rspace()                   │ │
//! │  └───────────────────────────────────────────────────────────────────────┘ │
//! │                              │                                             │
//! │  ┌───────────────────────────┴───────────────────────────────────────────┐ │
//! │  │                         RSpace Trait                                   │ │
//! │  │                                                                        │ │
//! │  │  Channel Operations:    Process Operations:    Value Operations:       │ │
//! │  │  • tell(name, value)    • register_process     • set_value             │ │
//! │  │  • ask(name) → Option   • update_process       • get_value             │ │
//! │  │  • peek(name) → Option  • get_process_state                            │ │
//! │  │                                                                        │ │
//! │  │  Utility: get_entry, is_solved, reset                                  │ │
//! │  └───────────────────────────────────────────────────────────────────────┘ │
//! │                              ▲                                             │
//! │               ┌──────────────┴──────────────┐                              │
//! │               │                             │                              │
//! │  ┌────────────┴───────────┐   ┌─────────────┴──────────┐                  │
//! │  │    PathMapRSpace       │   │   InMemoryRSpace       │                  │
//! │  │    (DEFAULT)           │   │   (TESTING/SIMPLE)     │                  │
//! │  │                        │   │                        │                  │
//! │  │  Feature: pathmap-impl │   │  Feature: inmemory-impl│                  │
//! │  │  • PathMap<Entry>      │   │  • HashMap<Entry>      │                  │
//! │  │  • Hierarchical keys   │   │  • Flat keys only      │                  │
//! │  │  • Production-ready    │   │  • Simple mock         │                  │
//! │  └────────────────────────┘   └────────────────────────┘                  │
//! │                                                                             │
//! │  ┌───────────────────────────────────────────────────────────────────────┐ │
//! │  │                         Core Types                                     │ │
//! │  │                                                                        │ │
//! │  │  Entry:         Value:              ProcessState:                      │ │
//! │  │  • Channel      • Int, Bool, Str    • Wait                             │ │
//! │  │  • Process      • Name, List        • Ready                            │ │
//! │  │  • Value        • Tuple, Map        • Value(Value)                     │ │
//! │  │                 • Par, Nil          • Error(String)                    │ │
//! │  └───────────────────────────────────────────────────────────────────────┘ │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Adding a New Implementation
//!
//! To add a new RSpace implementation:
//!
//! 1. **Create the implementation file** (e.g., `src/my_impl.rs`):
//!
//! ```ignore
//! use crate::entry::Entry;
//! use crate::rspace::RSpace;
//! use crate::value::{ProcessState, Value};
//! use anyhow::Result;
//!
//! pub struct MyRSpace {
//!     // Your storage
//! }
//!
//! impl MyRSpace {
//!     pub fn new() -> Self {
//!         Self { /* ... */ }
//!     }
//! }
//!
//! impl RSpace for MyRSpace {
//!     fn get_entry(&self, name: &str) -> Option<Entry> { /* ... */ }
//!     fn tell(&mut self, name: &str, data: Value) -> Result<()> { /* ... */ }
//!     fn ask(&mut self, name: &str) -> Result<Option<Value>> { /* ... */ }
//!     fn peek(&self, name: &str) -> Result<Option<Value>> { /* ... */ }
//!     fn register_process(&mut self, name: &str, state: ProcessState) -> Result<()> { /* ... */ }
//!     fn update_process(&mut self, name: &str, state: ProcessState) -> Result<()> { /* ... */ }
//!     fn get_process_state(&self, name: &str) -> Option<ProcessState> { /* ... */ }
//!     fn set_value(&mut self, name: &str, value: Value) -> Result<()> { /* ... */ }
//!     fn get_value(&self, name: &str) -> Option<Value> { /* ... */ }
//!     fn reset(&mut self) { /* ... */ }
//! }
//! ```
//!
//! 2. **Add feature flag** in `Cargo.toml`:
//!
//! ```toml
//! [features]
//! my-impl = ["dep:my-dependency"]  # if needed
//! ```
//!
//! 3. **Expose in lib.rs**:
//!
//! ```ignore
//! #[cfg(feature = "my-impl")]
//! mod my_impl;
//! #[cfg(feature = "my-impl")]
//! pub use my_impl::MyRSpace;
//! ```
//!
//! 4. **Test using the macro**:
//!
//! ```ignore
//! #[cfg(test)]
//! mod tests {
//!     rspace_interface_tests!(MyRSpace, my_rspace_tests);
//! }
//! ```
//!
//! # Entry Types
//!
//! | Entry | Storage | Solved When |
//! |-------|---------|-------------|
//! | `Channel(Vec<Value>)` | FIFO queue | Non-empty with resolved first value |
//! | `Process { state }` | ProcessState | `state == ProcessState::Value(_)` |
//! | `Value(Value)` | Immutable | Always |
//!
//! # Thread Safety
//!
//! For concurrent access, use `SharedRSpace`:
//!
//! ```
//! use rholang_rspace::{new_shared_rspace, RSpace, Value};
//! use std::thread;
//!
//! let rspace = new_shared_rspace();
//!
//! let rspace_clone = rspace.clone();
//! let handle = thread::spawn(move || {
//!     let mut guard = rspace_clone.lock().unwrap();
//!     guard.tell("from_thread", Value::Int(42)).unwrap();
//! });
//!
//! handle.join().unwrap();
//!
//! let guard = rspace.lock().unwrap();
//! assert_eq!(guard.peek("from_thread").unwrap(), Some(Value::Int(42)));
//! ```

mod entry;
mod error;
mod in_memory;
mod rspace;
mod value;

#[cfg(feature = "pathmap-impl")]
mod path_map;

use std::sync::{Arc, Mutex, OnceLock};

// ============================================================================
// Public API - Core Types
// ============================================================================

pub use entry::Entry;
pub use error::ExecError;
pub use rspace::RSpace;
pub use value::{ProcessHolder, ProcessState, Value};

// ============================================================================
// Public API - Implementations
// ============================================================================

/// PathMapRSpace - THE DEFAULT PRODUCTION IMPLEMENTATION.
///
/// Uses PathMap for efficient hierarchical key storage. Ideal for
/// Rholang's path-like channel names (e.g., "inbox/messages/1").
///
/// # Feature Flag
///
/// Enabled by default with feature `pathmap-impl`.
#[cfg(feature = "pathmap-impl")]
pub use path_map::PathMapRSpace;

/// InMemoryRSpace - TESTING AND SIMPLE USE CASES.
///
/// Simple HashMap-based implementation for unit tests and documentation.
/// Always available regardless of features.
pub use in_memory::InMemoryRSpace;

// ============================================================================
// Public API - Type Aliases
// ============================================================================

/// Shared RSpace type for concurrent access.
///
/// Wraps any RSpace implementation in `Arc<Mutex<>>` for thread-safe access.
pub type SharedRSpace = Arc<Mutex<Box<dyn RSpace>>>;

/// Boxed RSpace for dynamic dispatch.
pub type BoxedRSpace = Box<dyn RSpace>;

// ============================================================================
// Dependency Injection - Factory Functions
// ============================================================================

/// Create a new RSpace instance using the default implementation.
///
/// The default implementation is determined by feature flags:
/// - `pathmap-impl` (default): Returns `PathMapRSpace`
/// - `inmemory-impl`: Returns `InMemoryRSpace`
///
/// This is the **recommended way** to create an RSpace for production use.
///
/// # Example
///
/// ```
/// use rholang_rspace::{new_rspace, RSpace, Value};
///
/// let mut rspace = new_rspace();
/// rspace.tell("channel", Value::Int(42)).unwrap();
/// assert_eq!(rspace.ask("channel").unwrap(), Some(Value::Int(42)));
/// ```
pub fn new_rspace() -> BoxedRSpace {
    #[cfg(feature = "pathmap-impl")]
    {
        Box::new(PathMapRSpace::new())
    }
    #[cfg(all(not(feature = "pathmap-impl"), feature = "inmemory-impl"))]
    {
        Box::new(InMemoryRSpace::new())
    }
    #[cfg(all(not(feature = "pathmap-impl"), not(feature = "inmemory-impl")))]
    {
        // Fallback to InMemory if no feature is enabled
        Box::new(InMemoryRSpace::new())
    }
}

/// Create a new RSpace instance of a specific type.
///
/// Use this when you need a specific implementation regardless of features.
///
/// # Example
///
/// ```
/// use rholang_rspace::{new_rspace_with, InMemoryRSpace, RSpace, Value};
///
/// // Always use InMemoryRSpace, regardless of default
/// let mut rspace = new_rspace_with::<InMemoryRSpace>();
/// rspace.tell("test", Value::Int(42)).unwrap();
/// ```
pub fn new_rspace_with<T: RSpace + Default + 'static>() -> BoxedRSpace {
    Box::new(T::default())
}

/// Create a new shared RSpace instance (thread-safe).
///
/// This is the **recommended way** to create an RSpace for concurrent access.
///
/// # Example
///
/// ```
/// use rholang_rspace::{new_shared_rspace, RSpace, Value};
///
/// let rspace = new_shared_rspace();
/// {
///     let mut guard = rspace.lock().unwrap();
///     guard.tell("test", Value::Int(42)).unwrap();
/// }
/// ```
pub fn new_shared_rspace() -> SharedRSpace {
    Arc::new(Mutex::new(new_rspace()))
}

/// Create a shared RSpace from any RSpace implementation.
///
/// # Example
///
/// ```
/// use rholang_rspace::{shared_rspace, InMemoryRSpace, RSpace, Value};
///
/// let mut rspace = InMemoryRSpace::new();
/// rspace.tell("pre_loaded", Value::Int(42)).unwrap();
///
/// let shared = shared_rspace(rspace);
/// let guard = shared.lock().unwrap();
/// assert_eq!(guard.peek("pre_loaded").unwrap(), Some(Value::Int(42)));
/// ```
pub fn shared_rspace<R: RSpace + 'static>(rspace: R) -> SharedRSpace {
    Arc::new(Mutex::new(Box::new(rspace)))
}

/// Create a shared RSpace from a boxed RSpace.
///
/// # Example
///
/// ```
/// use rholang_rspace::{shared_rspace_from_box, new_rspace, RSpace, Value};
///
/// let mut rspace = new_rspace();
/// rspace.tell("test", Value::Int(42)).unwrap();
///
/// let shared = shared_rspace_from_box(rspace);
/// ```
pub fn shared_rspace_from_box(rspace: BoxedRSpace) -> SharedRSpace {
    Arc::new(Mutex::new(rspace))
}

// ============================================================================
// Dependency Injection - Global Singleton
// ============================================================================

/// Global RSpace singleton for application-wide access.
static GLOBAL_RSPACE: OnceLock<SharedRSpace> = OnceLock::new();

/// Initialize the global RSpace singleton.
///
/// This function should be called once at application startup.
/// Subsequent calls are no-ops (the first initialization wins).
///
/// # Example
///
/// ```
/// use rholang_rspace::{init_global_rspace, with_global_rspace_mut, Value};
///
/// // Initialize at startup
/// init_global_rspace();
///
/// // Use throughout the application
/// with_global_rspace_mut(|rspace| {
///     rspace.tell("app_channel", Value::Int(42)).unwrap();
/// });
/// ```
pub fn init_global_rspace() {
    let _ = GLOBAL_RSPACE.get_or_init(new_shared_rspace);
}

/// Initialize the global RSpace with a custom implementation.
///
/// # Example
///
/// ```
/// use rholang_rspace::{init_global_rspace_with, InMemoryRSpace, with_global_rspace, Value};
///
/// // Initialize with a custom implementation
/// init_global_rspace_with(InMemoryRSpace::new());
/// ```
pub fn init_global_rspace_with<R: RSpace + 'static>(rspace: R) {
    let _ = GLOBAL_RSPACE.get_or_init(|| shared_rspace(rspace));
}

/// Get a reference to the global RSpace.
///
/// Returns `None` if the global RSpace has not been initialized.
/// Use `init_global_rspace()` to initialize it first.
///
/// # Example
///
/// ```
/// use rholang_rspace::{init_global_rspace, global_rspace, Value};
///
/// init_global_rspace();
/// let rspace = global_rspace().expect("global RSpace not initialized");
/// let guard = rspace.lock().unwrap();
/// // Use guard...
/// ```
pub fn global_rspace() -> Option<&'static SharedRSpace> {
    GLOBAL_RSPACE.get()
}

/// Execute a closure with read access to the global RSpace.
///
/// Automatically initializes the global RSpace if not already initialized.
///
/// # Panics
///
/// Panics if the mutex is poisoned.
///
/// # Example
///
/// ```
/// use rholang_rspace::{init_global_rspace, with_global_rspace_mut, with_global_rspace, Value};
///
/// init_global_rspace();
/// with_global_rspace_mut(|rspace| {
///     rspace.tell("test", Value::Int(42)).unwrap();
/// });
///
/// with_global_rspace(|rspace| {
///     assert_eq!(rspace.peek("test").unwrap(), Some(Value::Int(42)));
/// });
/// ```
pub fn with_global_rspace<F, R>(f: F) -> R
where
    F: FnOnce(&dyn RSpace) -> R,
{
    init_global_rspace();
    let rspace = GLOBAL_RSPACE.get().expect("global RSpace not initialized");
    let guard = rspace.lock().expect("global RSpace mutex poisoned");
    f(guard.as_ref())
}

/// Execute a closure with mutable access to the global RSpace.
///
/// Automatically initializes the global RSpace if not already initialized.
///
/// # Panics
///
/// Panics if the mutex is poisoned.
///
/// # Example
///
/// ```
/// use rholang_rspace::{with_global_rspace_mut, Value};
///
/// with_global_rspace_mut(|rspace| {
///     rspace.tell("channel", Value::Int(42)).unwrap();
/// });
/// ```
pub fn with_global_rspace_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut dyn RSpace) -> R,
{
    init_global_rspace();
    let rspace = GLOBAL_RSPACE.get().expect("global RSpace not initialized");
    let mut guard = rspace.lock().expect("global RSpace mutex poisoned");
    f(guard.as_mut())
}

/// Reset the global RSpace (clears all data but keeps the instance).
///
/// # Example
///
/// ```
/// use rholang_rspace::{init_global_rspace, with_global_rspace_mut, reset_global_rspace, Value};
///
/// init_global_rspace();
/// with_global_rspace_mut(|rspace| {
///     rspace.tell("test", Value::Int(42)).unwrap();
/// });
///
/// reset_global_rspace();
///
/// with_global_rspace_mut(|rspace| {
///     assert_eq!(rspace.peek("test").unwrap(), None);
/// });
/// ```
pub fn reset_global_rspace() {
    if let Some(rspace) = GLOBAL_RSPACE.get() {
        let mut guard = rspace.lock().expect("global RSpace mutex poisoned");
        guard.reset();
    }
}

// ============================================================================
// Testing Utilities
// ============================================================================

/// Create a mock RSpace for testing (always InMemoryRSpace).
///
/// Use this in tests to ensure consistent behavior regardless of feature flags.
///
/// # Example
///
/// ```
/// use rholang_rspace::{mock_rspace, RSpace, Value};
///
/// let mut rspace = mock_rspace();
/// rspace.tell("test", Value::Int(42)).unwrap();
/// assert_eq!(rspace.ask("test").unwrap(), Some(Value::Int(42)));
/// ```
pub fn mock_rspace() -> BoxedRSpace {
    Box::new(InMemoryRSpace::new())
}

/// Create a shared mock RSpace for testing (always InMemoryRSpace).
///
/// # Example
///
/// ```
/// use rholang_rspace::{mock_shared_rspace, RSpace, Value};
///
/// let rspace = mock_shared_rspace();
/// {
///     let mut guard = rspace.lock().unwrap();
///     guard.tell("test", Value::Int(42)).unwrap();
/// }
/// ```
pub fn mock_shared_rspace() -> SharedRSpace {
    Arc::new(Mutex::new(Box::new(InMemoryRSpace::new())))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_rspace() {
        let mut rspace = new_rspace();
        rspace.tell("test", Value::Int(42)).unwrap();
        assert_eq!(rspace.peek("test").unwrap(), Some(Value::Int(42)));
        assert_eq!(rspace.ask("test").unwrap(), Some(Value::Int(42)));
        assert_eq!(rspace.ask("test").unwrap(), None);
    }

    #[test]
    fn test_new_rspace_with() {
        let mut rspace = new_rspace_with::<InMemoryRSpace>();
        rspace.tell("test", Value::Int(42)).unwrap();
        assert_eq!(rspace.ask("test").unwrap(), Some(Value::Int(42)));
    }

    #[test]
    fn test_new_shared_rspace() {
        let rspace = new_shared_rspace();
        {
            let mut guard = rspace.lock().unwrap();
            guard.tell("test", Value::Int(42)).unwrap();
        }
        {
            let guard = rspace.lock().unwrap();
            assert_eq!(guard.peek("test").unwrap(), Some(Value::Int(42)));
        }
    }

    #[test]
    fn test_mock_rspace() {
        let mut rspace = mock_rspace();
        rspace.tell("test", Value::Int(42)).unwrap();
        assert_eq!(rspace.ask("test").unwrap(), Some(Value::Int(42)));
    }

    #[test]
    fn test_channel_fifo() {
        let mut rspace = new_rspace();
        rspace.tell("queue", Value::Int(1)).unwrap();
        rspace.tell("queue", Value::Int(2)).unwrap();
        rspace.tell("queue", Value::Int(3)).unwrap();

        assert_eq!(rspace.ask("queue").unwrap(), Some(Value::Int(1)));
        assert_eq!(rspace.ask("queue").unwrap(), Some(Value::Int(2)));
        assert_eq!(rspace.ask("queue").unwrap(), Some(Value::Int(3)));
        assert_eq!(rspace.ask("queue").unwrap(), None);
    }

    #[test]
    fn test_process_operations() {
        let mut rspace = new_rspace();

        rspace
            .register_process("worker", ProcessState::Ready)
            .unwrap();
        assert!(!rspace.is_solved("worker"));

        rspace
            .update_process("worker", ProcessState::Value(Value::Int(100)))
            .unwrap();
        assert!(rspace.is_solved("worker"));
    }

    #[test]
    fn test_value_operations() {
        let mut rspace = new_rspace();

        rspace
            .set_value("config", Value::Str("prod".into()))
            .unwrap();
        assert!(rspace.is_solved("config"));
        assert_eq!(rspace.get_value("config"), Some(Value::Str("prod".into())));
    }

    #[test]
    fn test_hierarchical_paths() {
        let mut rspace = new_rspace();

        rspace.tell("inbox/messages/1", Value::Int(1)).unwrap();
        rspace.tell("inbox/messages/2", Value::Int(2)).unwrap();
        rspace.tell("@0:proc_1", Value::Str("proc".into())).unwrap();

        assert_eq!(
            rspace.peek("inbox/messages/1").unwrap(),
            Some(Value::Int(1))
        );
        assert_eq!(
            rspace.peek("inbox/messages/2").unwrap(),
            Some(Value::Int(2))
        );
        assert_eq!(
            rspace.peek("@0:proc_1").unwrap(),
            Some(Value::Str("proc".into()))
        );
    }

    #[test]
    fn test_entry_types() {
        let mut rspace = new_rspace();

        rspace.tell("channel", Value::Int(1)).unwrap();
        rspace
            .register_process("process", ProcessState::Ready)
            .unwrap();
        rspace.set_value("value", Value::Bool(true)).unwrap();

        assert!(rspace.get_entry("channel").unwrap().is_channel());
        assert!(rspace.get_entry("process").unwrap().is_process());
        assert!(rspace.get_entry("value").unwrap().is_value());
    }

    #[test]
    fn test_reset() {
        let mut rspace = new_rspace();

        rspace.tell("test", Value::Int(1)).unwrap();
        rspace.reset();

        assert!(rspace.get_entry("test").is_none());
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let rspace = new_shared_rspace();

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let rspace_clone = rspace.clone();
                thread::spawn(move || {
                    let channel = format!("ch{}", i);
                    let mut guard = rspace_clone.lock().unwrap();
                    guard.tell(&channel, Value::Int(i)).unwrap();
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        let mut guard = rspace.lock().unwrap();
        for i in 0..10 {
            let channel = format!("ch{}", i);
            assert_eq!(guard.ask(&channel).unwrap(), Some(Value::Int(i)));
        }
    }

    #[test]
    fn test_shared_rspace_from_impl() {
        let mut rspace = InMemoryRSpace::new();
        rspace.tell("pre", Value::Int(42)).unwrap();

        let shared = shared_rspace(rspace);
        let guard = shared.lock().unwrap();
        assert_eq!(guard.peek("pre").unwrap(), Some(Value::Int(42)));
    }

    #[test]
    fn test_shared_rspace_from_box() {
        let mut rspace = new_rspace();
        rspace.tell("pre", Value::Int(42)).unwrap();

        let shared = shared_rspace_from_box(rspace);
        let guard = shared.lock().unwrap();
        assert_eq!(guard.peek("pre").unwrap(), Some(Value::Int(42)));
    }

    // Note: Global singleton tests are tricky because OnceLock can only be set once.
    // These tests work in isolation but may interfere with each other.
    // In production, initialize once at startup.
}
