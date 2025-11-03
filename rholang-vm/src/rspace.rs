use crate::value::Value;
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

// Build a canonical key for RSpace storage when using a path-like map
fn rspace_key(kind: u16, channel: &str) -> String {
    format!("/rspace/{}/{}", kind, channel)
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

#[cfg(feature = "rspace-pathmap")]
mod pathmap_rspace {
    use super::{ensure_kind_matches_channel, rspace_key, RSpace};
    use crate::value::Value;
    use anyhow::Result;
    use pathmap::PathMap;
    use std::collections::VecDeque;
    use std::sync::RwLock;

    pub struct PathMapRSpace {
        inner: RwLock<PathMap<VecDeque<Value>>>,
    }

    impl PathMapRSpace {
        pub fn new() -> Self {
            Self {
                inner: RwLock::new(PathMap::new()),
            }
        }
    }

    impl super::RSpace for PathMapRSpace {
        fn tell(&mut self, kind: u16, channel: String, data: Value) -> Result<()> {
            ensure_kind_matches_channel(kind, &channel)?;
            let key = rspace_key(kind, &channel);
            let mut pm = self.inner.write().expect("poisoned");
            let mut q = pm.get(&key).cloned().unwrap_or_default();
            q.push_back(data);
            pm.insert(&key, q);
            Ok(())
        }
        fn ask(&mut self, kind: u16, channel: String) -> Result<Option<Value>> {
            ensure_kind_matches_channel(kind, &channel)?;
            let key = rspace_key(kind, &channel);
            let mut pm = self.inner.write().expect("poisoned");
            let mut q = pm.get(&key).cloned().unwrap_or_default();
            let res = q.pop_front();
            if res.is_some() {
                pm.insert(&key, q);
            }
            Ok(res)
        }
        fn peek(&self, kind: u16, channel: String) -> Result<Option<Value>> {
            ensure_kind_matches_channel(kind, &channel)?;
            let key = rspace_key(kind, &channel);
            let pm = self.inner.read().expect("poisoned");
            Ok(pm.get(&key).and_then(|q| q.front().cloned()))
        }
        fn reset(&mut self) {
            let mut pm = self.inner.write().expect("poisoned");
            *pm = PathMap::new();
        }
    }

    pub fn default_rspace() -> Box<dyn RSpace> {
        Box::new(PathMapRSpace::new())
    }
}

#[cfg(feature = "rspace-pathmap")]
pub use pathmap_rspace::default_rspace;

#[cfg(not(feature = "rspace-pathmap"))]
pub fn default_rspace() -> Box<dyn RSpace> {
    Box::new(InMemoryRSpace::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chan(k: u16, s: &str) -> (u16, String) { (k, format!("@{}:{}", k, s)) }

    #[test]
    fn inmem_fifo_and_peek_and_reset() {
        let (k, ch) = chan(1, "alpha");
        let (k2, ch2) = chan(1, "beta");
        let mut rs = InMemoryRSpace::new();

        rs.tell(k, ch.clone(), Value::Int(1)).unwrap();
        rs.tell(k, ch.clone(), Value::Int(2)).unwrap();
        rs.tell(k2, ch2.clone(), Value::Int(9)).unwrap();

        assert_eq!(rs.peek(k, ch.clone()).unwrap(), Some(Value::Int(1)));
        assert_eq!(rs.ask(k, ch.clone()).unwrap(), Some(Value::Int(1)));
        assert_eq!(rs.peek(k, ch.clone()).unwrap(), Some(Value::Int(2)));
        assert_eq!(rs.ask(k, ch.clone()).unwrap(), Some(Value::Int(2)));
        assert_eq!(rs.ask(k, ch.clone()).unwrap(), None);

        // Other channel unaffected
        assert_eq!(rs.peek(k2, ch2.clone()).unwrap(), Some(Value::Int(9)));

        rs.reset();
        assert_eq!(rs.peek(k2, ch2.clone()).unwrap(), None);
    }

    #[test]
    fn inmem_kind_channel_mismatch_errors() {
        let mut rs = InMemoryRSpace::new();
        // Wrong prefix vs kind
        let bad = rs.tell(2, "@1:alpha".to_string(), Value::Nil);
        assert!(bad.is_err());
    }

    #[cfg(feature = "rspace-pathmap")]
    #[test]
    fn pathmap_fifo_peek_reset() {
        let (k, ch) = chan(7, "q");
        let mut rs = crate::rspace::pathmap_rspace::PathMapRSpace::new();
        rs.tell(k, ch.clone(), Value::Str("a".into())).unwrap();
        rs.tell(k, ch.clone(), Value::Str("b".into())).unwrap();
        assert_eq!(rs.peek(k, ch.clone()).unwrap(), Some(Value::Str("a".into())));
        assert_eq!(rs.ask(k, ch.clone()).unwrap(), Some(Value::Str("a".into())));
        assert_eq!(rs.ask(k, ch.clone()).unwrap(), Some(Value::Str("b".into())));
        assert_eq!(rs.ask(k, ch.clone()).unwrap(), None);
        rs.reset();
        assert_eq!(rs.peek(k, ch.clone()).unwrap(), None);
    }
}
