//! PathMap-based RSpace implementation - THE DEFAULT PRODUCTION IMPLEMENTATION.

use crate::entry::Entry;
use crate::rspace::RSpace;
use crate::value::{ProcessState, Value};
use anyhow::{bail, Result};
use pathmap::PathMap;

/// PathMap-based RSpace - THE DEFAULT PRODUCTION IMPLEMENTATION.
///
/// Uses PathMap for efficient hierarchical key storage. This is optimal for
/// Rholang's channel naming conventions which often use path-like structures.
///
/// # Storage Model
///
/// ```text
/// PathMap<Entry>
///     │
///     ├── "inbox"
///     │     └── Entry::Channel([Value::Int(1), Value::Int(2)])
///     │
///     ├── "inbox/messages/1"
///     │     └── Entry::Channel([Value::Str("hello")])
///     │
///     ├── "@0:worker"
///     │     └── Entry::Process { state: ProcessState::Ready }
///     │
///     └── "config/timeout"
///           └── Entry::Value(Value::Int(30))
/// ```
///
/// # Performance
///
/// - **Lookup**: O(path_depth) - efficient for hierarchical keys
/// - **Insert**: O(path_depth) - creates path nodes as needed
/// - **Memory**: Shared prefixes reduce memory usage
///
/// # Example
///
/// ```
/// use rholang_rspace::{PathMapRSpace, RSpace, Value};
///
/// let mut rspace = PathMapRSpace::new();
///
/// // Hierarchical channel names work efficiently
/// rspace.tell("inbox/messages/1", Value::Int(1)).unwrap();
/// rspace.tell("inbox/messages/2", Value::Int(2)).unwrap();
///
/// assert_eq!(rspace.peek("inbox/messages/1").unwrap(), Some(Value::Int(1)));
/// ```
pub struct PathMapRSpace {
    store: PathMap<Entry>,
}

impl PathMapRSpace {
    /// Create a new empty PathMapRSpace.
    pub fn new() -> Self {
        Self {
            store: PathMap::new(),
        }
    }
}

impl Default for PathMapRSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl RSpace for PathMapRSpace {
    fn get_entry(&self, name: &str) -> Option<Entry> {
        self.store.get(name).cloned()
    }

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
                self.store.insert(name, Entry::Channel(vec![data]));
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

    fn register_process(&mut self, name: &str, state: ProcessState) -> Result<()> {
        if self.store.get(name).is_some() {
            bail!("entry '{}' already exists", name)
        }
        self.store.insert(name, Entry::Process { state });
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

    fn set_value(&mut self, name: &str, value: Value) -> Result<()> {
        if self.store.get(name).is_some() {
            bail!("entry '{}' already exists", name)
        }
        self.store.insert(name, Entry::Value(value));
        Ok(())
    }

    fn get_value(&self, name: &str) -> Option<Value> {
        match self.store.get(name) {
            Some(Entry::Value(val)) => Some(val.clone()),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.store = PathMap::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_operations() -> Result<()> {
        let mut rspace = PathMapRSpace::new();

        rspace.tell("inbox", Value::Int(42))?;
        assert_eq!(rspace.peek("inbox")?, Some(Value::Int(42)));
        assert_eq!(rspace.ask("inbox")?, Some(Value::Int(42)));
        assert_eq!(rspace.ask("inbox")?, None);

        Ok(())
    }

    #[test]
    fn test_channel_fifo() -> Result<()> {
        let mut rspace = PathMapRSpace::new();

        rspace.tell("queue", Value::Int(1))?;
        rspace.tell("queue", Value::Int(2))?;
        rspace.tell("queue", Value::Int(3))?;

        assert_eq!(rspace.ask("queue")?, Some(Value::Int(1)));
        assert_eq!(rspace.ask("queue")?, Some(Value::Int(2)));
        assert_eq!(rspace.ask("queue")?, Some(Value::Int(3)));

        Ok(())
    }

    #[test]
    fn test_hierarchical_paths() -> Result<()> {
        let mut rspace = PathMapRSpace::new();

        rspace.tell("inbox/messages/1", Value::Int(1))?;
        rspace.tell("inbox/messages/2", Value::Int(2))?;
        rspace.tell("@0:proc_1", Value::Str("process".into()))?;

        assert_eq!(rspace.peek("inbox/messages/1")?, Some(Value::Int(1)));
        assert_eq!(rspace.peek("inbox/messages/2")?, Some(Value::Int(2)));
        assert_eq!(
            rspace.peek("@0:proc_1")?,
            Some(Value::Str("process".into()))
        );

        Ok(())
    }

    #[test]
    fn test_process_operations() -> Result<()> {
        let mut rspace = PathMapRSpace::new();

        rspace.register_process("worker", ProcessState::Ready)?;
        assert_eq!(
            rspace.get_process_state("worker"),
            Some(ProcessState::Ready)
        );
        assert!(!rspace.is_solved("worker"));

        rspace.update_process("worker", ProcessState::Value(Value::Int(100)))?;
        assert!(rspace.is_solved("worker"));

        Ok(())
    }

    #[test]
    fn test_value_operations() -> Result<()> {
        let mut rspace = PathMapRSpace::new();

        rspace.set_value("config", Value::Str("prod".into()))?;
        assert_eq!(rspace.get_value("config"), Some(Value::Str("prod".into())));
        assert!(rspace.is_solved("config"));

        Ok(())
    }

    #[test]
    fn test_entry_types() -> Result<()> {
        let mut rspace = PathMapRSpace::new();

        rspace.tell("channel", Value::Int(1))?;
        rspace.register_process("process", ProcessState::Ready)?;
        rspace.set_value("value", Value::Bool(true))?;

        assert!(rspace.get_entry("channel").unwrap().is_channel());
        assert!(rspace.get_entry("process").unwrap().is_process());
        assert!(rspace.get_entry("value").unwrap().is_value());

        Ok(())
    }

    #[test]
    fn test_reset() -> Result<()> {
        let mut rspace = PathMapRSpace::new();

        rspace.tell("test", Value::Int(1))?;
        rspace.reset();

        assert!(rspace.get_entry("test").is_none());

        Ok(())
    }
}
