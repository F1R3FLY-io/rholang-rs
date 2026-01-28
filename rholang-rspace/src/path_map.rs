use crate::{ensure_kind_matches_channel, RSpace, Value};
use anyhow::Result;
use pathmap::PathMap;

pub struct PathMapRSpace {
    pub(crate) store: PathMap<Vec<Value>>,
}

impl PathMapRSpace {
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
    fn tell(&mut self, kind: u16, channel: String, data: Value) -> Result<()> {
        ensure_kind_matches_channel(kind, &channel)?;
        let key = format!("{}/{}", kind, channel);
        if let Some(q) = self.store.get_mut(&key) {
            q.push(data);
        } else {
            self.store.insert(&key, vec![data]);
        }
        Ok(())
    }

    fn ask(&mut self, kind: u16, channel: String) -> Result<Option<Value>> {
        ensure_kind_matches_channel(kind, &channel)?;
        let key = format!("{}/{}", kind, channel);
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
        let key = format!("{}/{}", kind, channel);
        Ok(self.store.get(&key).and_then(|q| q.first()).cloned())
    }

    fn reset(&mut self) {
        self.store = PathMap::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_map_rspace_basic() -> Result<()> {
        let mut rspace = PathMapRSpace::new();
        let channel = "@0:test".to_string();
        let val = Value::Int(42);

        rspace.tell(0, channel.clone(), val.clone())?;
        assert_eq!(rspace.peek(0, channel.clone())?, Some(val.clone()));
        assert_eq!(rspace.ask(0, channel.clone())?, Some(val));
        assert_eq!(rspace.ask(0, channel.clone())?, None);

        Ok(())
    }

    #[test]
    fn test_path_map_rspace_fifo() -> Result<()> {
        let mut rspace = PathMapRSpace::new();
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
    fn test_path_map_rspace_reset() -> Result<()> {
        let mut rspace = PathMapRSpace::new();
        let channel = "@0:reset".to_string();

        rspace.tell(0, channel.clone(), Value::Int(1))?;
        rspace.reset();
        assert_eq!(rspace.ask(0, channel.clone())?, None);

        Ok(())
    }
}
