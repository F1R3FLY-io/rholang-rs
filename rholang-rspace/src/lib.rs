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
use std::collections::HashMap;

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

// Default in-memory sequential implementation that mirrors previous VM behaviour
pub struct InMemoryRSpace {
    store: HashMap<(u16, String), Vec<Value>>,
}

impl InMemoryRSpace {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }
}

impl Default for InMemoryRSpace {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_kind_matches_channel(kind: u16, channel: &str) -> anyhow::Result<()> {
    if !channel.starts_with(&format!("@{}:", kind)) {
        anyhow::bail!(
            "channel-kind mismatch: kind {} does not match channel {}",
            kind,
            channel
        );
    }
    Ok(())
}

impl RSpace for InMemoryRSpace {
    fn tell(&mut self, kind: u16, channel: String, data: Value) -> Result<()> {
        ensure_kind_matches_channel(kind, &channel)?;
        let key = (kind, channel);
        self.store.entry(key).or_default().push(data);
        Ok(())
    }

    fn ask(&mut self, kind: u16, channel: String) -> Result<Option<Value>> {
        ensure_kind_matches_channel(kind, &channel)?;
        let key = (kind, channel);
        Ok(self.store.get_mut(&key).and_then(|q| {
            if q.is_empty() {
                None
            } else {
                Some(q.remove(0))
            }
        }))
    }

    fn peek(&self, kind: u16, channel: String) -> Result<Option<Value>> {
        ensure_kind_matches_channel(kind, &channel)?;
        let key = (kind, channel);
        Ok(self.store.get(&key).and_then(|q| q.first()).cloned())
    }

    fn reset(&mut self) {
        self.store.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_rspace_basic() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();
        let channel = "@0:test".to_string();
        let val = Value::Int(42);

        rspace.tell(0, channel.clone(), val.clone())?;
        assert_eq!(rspace.peek(0, channel.clone())?, Some(val.clone()));
        assert_eq!(rspace.ask(0, channel.clone())?, Some(val));
        assert_eq!(rspace.ask(0, channel.clone())?, None);

        Ok(())
    }

    #[test]
    fn test_mismatch_kind() {
        let mut rspace = InMemoryRSpace::new();
        let channel = "@0:test".to_string();
        let val = Value::Int(42);

        assert!(rspace.tell(1, channel.clone(), val).is_err());
    }
}
