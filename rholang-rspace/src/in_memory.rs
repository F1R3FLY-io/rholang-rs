use crate::{ensure_kind_matches_channel, RSpace, Value};
use anyhow::Result;
use std::collections::HashMap;

// Default in-memory sequential implementation that mirrors previous VM behaviour
pub struct InMemoryRSpace {
    pub(crate) store: HashMap<(u16, String), Vec<Value>>,
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
    fn test_in_memory_rspace_fifo() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();
        let channel = "@0:fifo".to_string();
        
        rspace.tell(0, channel.clone(), Value::Int(1))?;
        rspace.tell(0, channel.clone(), Value::Int(2))?;
        
        assert_eq!(rspace.peek(0, channel.clone())?, Some(Value::Int(1)));
        assert_eq!(rspace.ask(0, channel.clone())?, Some(Value::Int(1)));
        assert_eq!(rspace.ask(0, channel.clone())?, Some(Value::Int(2)));
        assert_eq!(rspace.ask(0, channel.clone())?, None);
        
        Ok(())
    }

    #[test]
    fn test_in_memory_rspace_reset() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();
        let channel = "@0:reset".to_string();
        
        rspace.tell(0, channel.clone(), Value::Int(1))?;
        rspace.reset();
        assert_eq!(rspace.ask(0, channel.clone())?, None);
        
        Ok(())
    }

    #[test]
    fn test_in_memory_rspace_default() {
        let rspace: InMemoryRSpace = Default::default();
        assert!(rspace.store.is_empty());
    }

    #[test]
    fn test_mismatch_kind() {
        let mut rspace = InMemoryRSpace::new();
        let channel = "@0:test".to_string();
        let val = Value::Int(42);

        assert!(rspace.tell(1, channel.clone(), val.clone()).is_err());
        assert!(rspace.ask(1, channel.clone()).is_err());
        assert!(rspace.peek(1, channel.clone()).is_err());
    }

    #[test]
    fn test_ask_peek_empty_channel() -> Result<()> {
        let mut rspace = InMemoryRSpace::new();
        let channel = "@0:empty".to_string();
        
        // Peek/Ask on non-existent channel
        assert_eq!(rspace.peek(0, channel.clone())?, None);
        assert_eq!(rspace.ask(0, channel.clone())?, None);
        
        // Peek/Ask on channel that was once populated but now empty
        rspace.tell(0, channel.clone(), Value::Nil)?;
        rspace.ask(0, channel.clone())?;
        assert_eq!(rspace.peek(0, channel.clone())?, None);
        assert_eq!(rspace.ask(0, channel.clone())?, None);
        
        Ok(())
    }
}
