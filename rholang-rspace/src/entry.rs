//! Entry type for unified RSpace storage.
//!
//! Each name in RSpace identifies exactly one Entry.

use crate::value::{ProcessState, Value};

/// Entry types that can be stored in RSpace.
///
/// RSpace stores entries identified by unique names. Each name maps to exactly
/// one entry, which can be a channel, a process, or a direct value.
///
/// # Entry Types
///
/// - **Channel**: FIFO queue of values, supports tell/ask/peek operations
/// - **Process**: Registered process with state tracking
/// - **Value**: Direct terminal value, immutable once set
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    /// Channel with FIFO queue of values.
    /// Supports tell (append), ask (pop first), and peek (read first).
    Channel(Vec<Value>),

    /// Registered process with state tracking.
    /// Solved when state is `ProcessState::Value`.
    Process { state: ProcessState },

    /// Direct terminal value.
    /// Immutable once set, always considered solved.
    Value(Value),
}

impl Entry {
    /// Create a new empty channel entry.
    pub fn channel() -> Self {
        Entry::Channel(Vec::new())
    }

    /// Create a new channel entry with initial values.
    pub fn channel_with(values: Vec<Value>) -> Self {
        Entry::Channel(values)
    }

    /// Create a new process entry with the given state.
    pub fn process(state: ProcessState) -> Self {
        Entry::Process { state }
    }

    /// Create a new value entry.
    pub fn value(val: Value) -> Self {
        Entry::Value(val)
    }

    /// Check if this entry is in a "solved" state.
    ///
    /// - Channel: solved if queue is non-empty AND first value is resolved
    ///   - Par values: resolved if all processes are in Value state
    ///   - Other values: always resolved
    /// - Process: solved if in `ProcessState::Value` state
    /// - Value: always solved
    pub fn is_solved(&self) -> bool {
        match self {
            Entry::Channel(queue) => {
                // Channel is solved if non-empty AND first value is resolved
                if queue.is_empty() {
                    return false;
                }
                Self::value_is_resolved(&queue[0])
            }
            Entry::Process { state } => matches!(state, ProcessState::Value(_)),
            Entry::Value(_) => true,
        }
    }

    /// Check if a Value is fully resolved.
    ///
    /// - Par: resolved if all processes are in Value state (empty Par is resolved)
    /// - Other values: always resolved
    fn value_is_resolved(value: &Value) -> bool {
        match value {
            Value::Par(procs) => procs
                .iter()
                .all(|p| matches!(p.state(), ProcessState::Value(_))),
            _ => true, // All non-Par values are resolved
        }
    }

    /// Check if this entry is a channel.
    pub fn is_channel(&self) -> bool {
        matches!(self, Entry::Channel(_))
    }

    /// Check if this entry is a process.
    pub fn is_process(&self) -> bool {
        matches!(self, Entry::Process { .. })
    }

    /// Check if this entry is a direct value.
    pub fn is_value(&self) -> bool {
        matches!(self, Entry::Value(_))
    }

    /// Get the channel queue if this is a channel entry.
    pub fn as_channel(&self) -> Option<&Vec<Value>> {
        match self {
            Entry::Channel(queue) => Some(queue),
            _ => None,
        }
    }

    /// Get the channel queue mutably if this is a channel entry.
    pub fn as_channel_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Entry::Channel(queue) => Some(queue),
            _ => None,
        }
    }

    /// Get the process state if this is a process entry.
    pub fn as_process_state(&self) -> Option<&ProcessState> {
        match self {
            Entry::Process { state } => Some(state),
            _ => None,
        }
    }

    /// Get the value if this is a value entry.
    pub fn as_value(&self) -> Option<&Value> {
        match self {
            Entry::Value(val) => Some(val),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_entry() {
        let mut entry = Entry::channel();
        assert!(entry.is_channel());
        assert!(!entry.is_solved()); // Empty channel is not solved

        if let Some(queue) = entry.as_channel_mut() {
            queue.push(Value::Int(42));
        }
        assert!(entry.is_solved()); // Non-empty channel is solved
    }

    #[test]
    fn test_process_entry() {
        let entry = Entry::process(ProcessState::Ready);
        assert!(entry.is_process());
        assert!(!entry.is_solved()); // Ready process is not solved

        let entry = Entry::process(ProcessState::Value(Value::Int(42)));
        assert!(entry.is_solved()); // Value process is solved

        let entry = Entry::process(ProcessState::Wait);
        assert!(!entry.is_solved());

        let entry = Entry::process(ProcessState::Error("err".to_string()));
        assert!(!entry.is_solved());
    }

    #[test]
    fn test_value_entry() {
        let entry = Entry::value(Value::Int(42));
        assert!(entry.is_value());
        assert!(entry.is_solved()); // Value is always solved
        assert_eq!(entry.as_value(), Some(&Value::Int(42)));
    }

    #[test]
    fn test_channel_with_values() {
        let entry = Entry::channel_with(vec![Value::Int(1), Value::Int(2)]);
        assert!(entry.is_channel());
        assert!(entry.is_solved());
        assert_eq!(entry.as_channel().unwrap().len(), 2);
    }
}
