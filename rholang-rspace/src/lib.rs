pub mod in_memory;
pub mod path_map;

pub use in_memory::InMemoryRSpace;
pub use path_map::PathMapRSpace;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Bool(bool),
    Str(String),
    Name(String),
    List(Vec<Value>),
    Tuple(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Nil,
}

impl Value {
    #[allow(dead_code)]
    pub fn as_int(&self) -> Option<i64> {
        if let Value::Int(n) = self {
            Some(*n)
        } else {
            None
        }
    }
}

use anyhow::Result;

// Minimal abstract interface for different RSpace implementations used by the current VM
pub trait RSpace {
    // Put data into a channel queue (append), return true-like confirmation via Bool(true) at opcode level
    fn tell(&mut self, kind: u16, channel: String, data: Value) -> Result<()>;
    // Destructive read: remove and return oldest value, or Nil if empty / missing
    fn ask(&mut self, kind: u16, channel: String) -> Result<Option<Value>>;
    // Non-destructive read: return oldest value without removing
    fn peek(&self, kind: u16, channel: String) -> Result<Option<Value>>;
    // Reset storage (used by tests)
    fn reset(&mut self);
}

// Set PathMapRSpace as the default implementation
pub type DefaultRSpace = PathMapRSpace;

pub(crate) fn ensure_kind_matches_channel(kind: u16, channel: &str) -> anyhow::Result<()> {
    if !channel.starts_with(&format!("@{}:", kind)) {
        anyhow::bail!(
            "channel-kind mismatch: kind {} does not match channel {}",
            kind,
            channel
        );
    }
    Ok(())
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
        assert_eq!(Value::List(vec![Value::Int(1)]), Value::List(vec![Value::Int(1)]));
        assert_eq!(Value::Tuple(vec![Value::Int(1)]), Value::Tuple(vec![Value::Int(1)]));
        assert_eq!(Value::Map(vec![(Value::Int(1), Value::Int(2))]), Value::Map(vec![(Value::Int(1), Value::Int(2))]));
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
    fn test_default_rspace() {
        let rspace = DefaultRSpace::default();
        // Just verify it's the right type
        let _: PathMapRSpace = rspace;
    }
}
