//! Core value types for RSpace storage.

use crate::ExecError;
use std::any::Any;
use std::fmt;

/// Process execution state.
///
/// Tracks the lifecycle of a process from creation to completion.
#[derive(Clone, Debug, PartialEq)]
pub enum ProcessState {
    /// Process is blocked, must not be executed.
    Wait,
    /// Process is eligible for execution.
    Ready,
    /// Process finished successfully with a final value (terminal state).
    Value(Value),
    /// Process failed with an error message (terminal state).
    Error(String),
}

/// Opaque process holder for `Value::Par`.
///
/// This trait allows `Process` to be defined in a downstream crate (rholang-process)
/// while `Value` stays in the core abstraction layer (rholang-rspace).
///
/// # SOLID Principles
/// - **Interface Segregation**: Minimal interface for process abstraction
/// - **Dependency Inversion**: High-level Value depends on abstraction, not concrete Process
pub trait ProcessHolder: Send + Sync + fmt::Debug {
    /// Downcast to concrete type.
    fn as_any(&self) -> &dyn Any;

    /// Downcast to concrete type mutably.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Clone into a boxed trait object.
    fn clone_box(&self) -> Box<dyn ProcessHolder>;

    /// Compare equality with another process holder.
    fn eq_box(&self, other: &dyn ProcessHolder) -> bool;

    /// Check if the process is ready to execute.
    fn is_ready(&self) -> bool;

    /// Execute the process and return its result value.
    fn execute(&mut self) -> Result<Value, ExecError>;

    /// Get the source reference (debug/provenance tag).
    fn source_ref(&self) -> &str;

    /// Get the current process state.
    fn state(&self) -> &ProcessState;
}

impl Clone for Box<dyn ProcessHolder> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl PartialEq for Box<dyn ProcessHolder> {
    fn eq(&self, other: &Self) -> bool {
        self.eq_box(other.as_ref())
    }
}

/// Runtime value in RSpace.
///
/// Values are the fundamental data type stored in RSpace channels,
/// passed between processes, and returned from computations.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// 64-bit signed integer.
    Int(i64),
    /// Boolean value.
    Bool(bool),
    /// UTF-8 string.
    Str(String),
    /// Channel/name reference.
    Name(String),
    /// Ordered list of values.
    List(Vec<Value>),
    /// Fixed-size tuple of values.
    Tuple(Vec<Value>),
    /// Key-value map (preserves insertion order).
    Map(Vec<(Value, Value)>),
    /// Parallel composition of processes.
    /// Use rholang-process utilities to work with these.
    Par(Vec<Box<dyn ProcessHolder>>),
    /// Null/unit value.
    Nil,
}

impl Value {
    /// Try to extract an integer value.
    #[allow(dead_code)]
    pub fn as_int(&self) -> Option<i64> {
        if let Value::Int(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Try to extract a boolean value.
    #[allow(dead_code)]
    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// Try to extract a string value.
    #[allow(dead_code)]
    pub fn as_str(&self) -> Option<&str> {
        if let Value::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_as_int() {
        assert_eq!(Value::Int(42).as_int(), Some(42));
        assert_eq!(Value::Bool(true).as_int(), None);
        assert_eq!(Value::Str("test".to_string()).as_int(), None);
        assert_eq!(Value::Nil.as_int(), None);
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Int(1), Value::Int(1));
        assert_ne!(Value::Int(1), Value::Int(2));
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
        assert_eq!(Value::Str("a".into()), Value::Str("a".into()));
        assert_eq!(Value::Name("x".into()), Value::Name("x".into()));
        assert_eq!(Value::Nil, Value::Nil);
        assert_eq!(
            Value::List(vec![Value::Int(1)]),
            Value::List(vec![Value::Int(1)])
        );
        assert_eq!(
            Value::Tuple(vec![Value::Int(1)]),
            Value::Tuple(vec![Value::Int(1)])
        );
        assert_eq!(
            Value::Map(vec![(Value::Int(1), Value::Int(2))]),
            Value::Map(vec![(Value::Int(1), Value::Int(2))])
        );
    }

    #[test]
    fn test_value_clone() {
        let val = Value::List(vec![Value::Int(1), Value::Str("s".into())]);
        assert_eq!(val.clone(), val);
    }

    #[test]
    fn test_value_debug() {
        let val = Value::Int(1);
        assert!(!format!("{:?}", val).is_empty());
    }

    #[test]
    fn test_process_state_equality() {
        assert_eq!(ProcessState::Wait, ProcessState::Wait);
        assert_eq!(ProcessState::Ready, ProcessState::Ready);
        assert_eq!(
            ProcessState::Value(Value::Int(1)),
            ProcessState::Value(Value::Int(1))
        );
        assert_ne!(
            ProcessState::Value(Value::Int(1)),
            ProcessState::Value(Value::Int(2))
        );
        assert_eq!(
            ProcessState::Error("e".into()),
            ProcessState::Error("e".into())
        );
    }
}
