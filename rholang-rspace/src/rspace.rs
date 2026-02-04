//! RSpace trait definition - the core abstraction for Rholang's tuple space.
//!
//! # SOLID Principles
//!
//! - **Single Responsibility**: RSpace handles only storage operations
//! - **Open/Closed**: New implementations can be added without modifying existing code
//! - **Liskov Substitution**: Any RSpace implementation is interchangeable
//! - **Interface Segregation**: Focused interface with clear operation categories
//! - **Dependency Inversion**: Consumers depend on this abstraction, not concrete implementations

use crate::entry::Entry;
use crate::value::{ProcessState, Value};
use anyhow::Result;

/// Unified storage interface for channels, processes, and values.
///
/// RSpace (Rholang Space) is the core tuple space abstraction for storing and
/// retrieving data during Rholang execution. It provides a unified interface
/// for three types of entries:
///
/// - **Channels**: FIFO queues for message passing
/// - **Processes**: Registered processes with state tracking
/// - **Values**: Direct immutable values
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` to support concurrent access
/// across multiple threads/processes.
///
/// # Example
///
/// ```ignore
/// use rholang_rspace::{RSpace, InMemoryRSpace, Value};
///
/// let mut rspace = InMemoryRSpace::new();
///
/// // Channel operations
/// rspace.tell("inbox", Value::Int(42))?;
/// let value = rspace.ask("inbox")?;
///
/// // Process operations
/// rspace.register_process("worker", ProcessState::Ready)?;
/// rspace.update_process("worker", ProcessState::Value(Value::Nil))?;
///
/// // Value operations
/// rspace.set_value("config", Value::Str("prod".into()))?;
/// ```
pub trait RSpace: Send + Sync {
    // =========================================================================
    // Entry-based API (unified)
    // =========================================================================

    /// Get entry by name.
    ///
    /// Returns `None` if no entry exists with the given name.
    fn get_entry(&self, name: &str) -> Option<Entry>;

    /// Check if an entry exists and is in a solved state.
    ///
    /// An entry is solved when:
    /// - Channel: non-empty with resolved first value
    /// - Process: in `ProcessState::Value` state
    /// - Value: always solved
    fn is_solved(&self, name: &str) -> bool {
        self.get_entry(name).is_some_and(|e| e.is_solved())
    }

    // =========================================================================
    // Channel operations (for Entry::Channel)
    // =========================================================================

    /// Put data into a channel (creates Entry::Channel if not exists).
    ///
    /// # Errors
    ///
    /// Returns error if entry exists but is not a channel.
    fn tell(&mut self, name: &str, data: Value) -> Result<()>;

    /// Destructive read: remove and return oldest value from channel.
    ///
    /// Returns `None` if channel is empty or doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns error if entry exists but is not a channel.
    fn ask(&mut self, name: &str) -> Result<Option<Value>>;

    /// Non-destructive read: return oldest value without removing.
    ///
    /// Returns `None` if channel is empty or doesn't exist.
    ///
    /// # Errors
    ///
    /// Returns error if entry exists but is not a channel.
    fn peek(&self, name: &str) -> Result<Option<Value>>;

    // =========================================================================
    // Process operations (for Entry::Process)
    // =========================================================================

    /// Register a process by name with initial state.
    ///
    /// # Errors
    ///
    /// Returns error if entry already exists with that name.
    fn register_process(&mut self, name: &str, state: ProcessState) -> Result<()>;

    /// Update a registered process's state.
    ///
    /// # Errors
    ///
    /// Returns error if entry doesn't exist or is not a process.
    fn update_process(&mut self, name: &str, state: ProcessState) -> Result<()>;

    /// Get a registered process's state.
    ///
    /// Returns `None` if entry doesn't exist or is not a process.
    fn get_process_state(&self, name: &str) -> Option<ProcessState>;

    // =========================================================================
    // Value operations (for Entry::Value)
    // =========================================================================

    /// Store a direct value (terminal, immutable).
    ///
    /// # Errors
    ///
    /// Returns error if entry already exists with that name.
    fn set_value(&mut self, name: &str, value: Value) -> Result<()>;

    /// Get a stored value.
    ///
    /// Returns `None` if entry doesn't exist or is not a value.
    fn get_value(&self, name: &str) -> Option<Value>;

    // =========================================================================
    // Utility
    // =========================================================================

    /// Reset all storage, clearing all entries.
    fn reset(&mut self);
}
