//! In-memory RSpace implementation using HashMap-based Entry storage.
//!
//! This is a simple, fast implementation suitable for testing and single-process
//! execution. For production use with hierarchical channel names, consider
//! PathMapRSpace from the rholang-rspace-pathmap crate.

use crate::entry::Entry;
use crate::rspace::RSpace;
use crate::value::{ProcessState, Value};
use anyhow::{bail, Result};
use std::collections::HashMap;

/// In-memory RSpace implementation using HashMap-based Entry storage.
///
/// This implementation provides O(1) average-case lookup, insertion, and deletion
/// for flat channel names. It's ideal for:
///
/// - Unit testing
/// - Simple scripts
/// - Development and debugging
///
/// For production use with hierarchical channel names like `inbox/messages/1`,
/// consider using `PathMapRSpace` from the `rholang-rspace-pathmap` crate.
///
/// # Example
///
/// ```
/// use rholang_rspace::{InMemoryRSpace, RSpace, Value};
///
/// let mut rspace = InMemoryRSpace::new();
/// rspace.tell("test", Value::Int(42)).unwrap();
/// assert_eq!(rspace.peek("test").unwrap(), Some(Value::Int(42)));
/// ```
#[derive(Default)]
pub struct InMemoryRSpace {
    store: HashMap<String, Entry>,
}

impl InMemoryRSpace {
    /// Create a new empty InMemoryRSpace.
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }
}

impl RSpace for InMemoryRSpace {
    // === Entry-based API ===

    fn get_entry(&self, name: &str) -> Option<Entry> {
        self.store.get(name).cloned()
    }

    // === Channel operations ===

    fn tell(&mut self, name: &str, data: Value) -> Result<()> {
        match self.store.get_mut(name) {
            Some(Entry::Channel(queue)) => {
                queue.push(data);
                Ok(())
            }
            Some(_) => {
                bail!("entry '{}' exists but is not a channel", name)
            }
            None => {
                self.store
                    .insert(name.to_string(), Entry::Channel(vec![data]));
                Ok(())
            }
        }
    }

    fn ask(&mut self, name: &str) -> Result<Option<Value>> {
        match self.store.get_mut(name) {
            Some(Entry::Channel(queue)) => {
                if queue.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(queue.remove(0)))
                }
            }
            Some(_) => {
                bail!("entry '{}' exists but is not a channel", name)
            }
            None => Ok(None),
        }
    }

    fn peek(&self, name: &str) -> Result<Option<Value>> {
        match self.store.get(name) {
            Some(Entry::Channel(queue)) => Ok(queue.first().cloned()),
            Some(_) => {
                bail!("entry '{}' exists but is not a channel", name)
            }
            None => Ok(None),
        }
    }

    // === Process operations ===

    fn register_process(&mut self, name: &str, state: ProcessState) -> Result<()> {
        if self.store.contains_key(name) {
            bail!("entry '{}' already exists", name)
        }
        self.store
            .insert(name.to_string(), Entry::Process { state });
        Ok(())
    }

    fn update_process(&mut self, name: &str, state: ProcessState) -> Result<()> {
        match self.store.get_mut(name) {
            Some(Entry::Process { state: s }) => {
                *s = state;
                Ok(())
            }
            Some(_) => {
                bail!("entry '{}' exists but is not a process", name)
            }
            None => {
                bail!("no process registered with name '{}'", name)
            }
        }
    }

    fn get_process_state(&self, name: &str) -> Option<ProcessState> {
        match self.store.get(name) {
            Some(Entry::Process { state }) => Some(state.clone()),
            _ => None,
        }
    }

    // === Value operations ===

    fn set_value(&mut self, name: &str, value: Value) -> Result<()> {
        if self.store.contains_key(name) {
            bail!("entry '{}' already exists", name)
        }
        self.store.insert(name.to_string(), Entry::Value(value));
        Ok(())
    }

    fn get_value(&self, name: &str) -> Option<Value> {
        match self.store.get(name) {
            Some(Entry::Value(val)) => Some(val.clone()),
            _ => None,
        }
    }

    // === Utility ===

    fn reset(&mut self) {
        self.store.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Channel operations
    // =========================================================================

    #[test]
    fn test_channel_basic() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();

        rspace.tell("inbox", Value::Int(42))?;
        assert_eq!(rspace.peek("inbox")?, Some(Value::Int(42)));
        assert_eq!(rspace.ask("inbox")?, Some(Value::Int(42)));
        assert_eq!(rspace.ask("inbox")?, None);

        Ok(())
    }

    #[test]
    fn test_channel_fifo() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();

        rspace.tell("queue", Value::Int(1))?;
        rspace.tell("queue", Value::Int(2))?;
        rspace.tell("queue", Value::Int(3))?;

        assert_eq!(rspace.ask("queue")?, Some(Value::Int(1)));
        assert_eq!(rspace.ask("queue")?, Some(Value::Int(2)));
        assert_eq!(rspace.ask("queue")?, Some(Value::Int(3)));
        assert_eq!(rspace.ask("queue")?, None);

        Ok(())
    }

    #[test]
    fn test_channel_is_solved() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();

        assert!(!rspace.is_solved("inbox")); // Doesn't exist

        rspace.tell("inbox", Value::Int(42))?;
        assert!(rspace.is_solved("inbox")); // Non-empty channel

        rspace.ask("inbox")?;
        assert!(!rspace.is_solved("inbox")); // Empty channel

        Ok(())
    }

    // =========================================================================
    // Process operations
    // =========================================================================

    #[test]
    fn test_process_registration() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();

        rspace.register_process("worker", ProcessState::Ready)?;
        assert_eq!(
            rspace.get_process_state("worker"),
            Some(ProcessState::Ready)
        );
        assert!(!rspace.is_solved("worker")); // Ready is not solved

        rspace.update_process("worker", ProcessState::Value(Value::Int(100)))?;
        assert!(rspace.is_solved("worker")); // Value is solved

        Ok(())
    }

    #[test]
    fn test_process_already_exists() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();

        rspace.register_process("worker", ProcessState::Ready)?;
        assert!(rspace
            .register_process("worker", ProcessState::Ready)
            .is_err());

        Ok(())
    }

    // =========================================================================
    // Value operations
    // =========================================================================

    #[test]
    fn test_value_storage() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();

        rspace.set_value("config", Value::Str("production".to_string()))?;
        assert_eq!(
            rspace.get_value("config"),
            Some(Value::Str("production".to_string()))
        );
        assert!(rspace.is_solved("config")); // Value is always solved

        Ok(())
    }

    #[test]
    fn test_value_already_exists() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();

        rspace.set_value("config", Value::Int(1))?;
        assert!(rspace.set_value("config", Value::Int(2)).is_err());

        Ok(())
    }

    // =========================================================================
    // Entry type checking
    // =========================================================================

    #[test]
    fn test_entry_type_mismatch() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();

        // Create a channel
        rspace.tell("mixed", Value::Int(1))?;

        // Try to use it as a process - should fail
        assert!(rspace
            .register_process("mixed", ProcessState::Ready)
            .is_err());
        assert!(rspace.update_process("mixed", ProcessState::Ready).is_err());
        assert!(rspace.set_value("mixed", Value::Int(2)).is_err());

        Ok(())
    }

    #[test]
    fn test_get_entry() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();

        rspace.tell("channel", Value::Int(1))?;
        rspace.register_process("process", ProcessState::Ready)?;
        rspace.set_value("value", Value::Bool(true))?;

        assert!(rspace.get_entry("channel").unwrap().is_channel());
        assert!(rspace.get_entry("process").unwrap().is_process());
        assert!(rspace.get_entry("value").unwrap().is_value());
        assert!(rspace.get_entry("nonexistent").is_none());

        Ok(())
    }

    #[test]
    fn test_reset() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();

        rspace.tell("channel", Value::Int(1))?;
        rspace.register_process("process", ProcessState::Ready)?;
        rspace.set_value("value", Value::Bool(true))?;

        rspace.reset();

        assert!(rspace.get_entry("channel").is_none());
        assert!(rspace.get_entry("process").is_none());
        assert!(rspace.get_entry("value").is_none());

        Ok(())
    }

    #[test]
    fn test_default() {
        let rspace: InMemoryRSpace = Default::default();
        assert!(rspace.store.is_empty());
    }
}
