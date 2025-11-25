use crate::value::Value;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Output { pid: u64, seq: u64, value: Value },
}

#[derive(Clone)]
pub struct Journal {
    inner: Arc<Mutex<InnerJournal>>,
}

#[derive(Default)]
struct InnerJournal {
    // Buffer effects keyed by seq until we can flush in order
    pending: BTreeMap<u64, Effect>,
    next_seq_to_flush: u64,
    // Flushed, ordered effects
    committed: Vec<Effect>,
}

impl Journal {
    pub fn new(first_seq: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerJournal {
                pending: BTreeMap::new(),
                next_seq_to_flush: first_seq,
                committed: Vec::new(),
            })),
        }
    }

    pub fn commit(&self, effect: Effect) {
        let mut g = self.inner.lock().unwrap();
        match &effect {
            Effect::Output { seq, .. } => {
                g.pending.insert(*seq, effect);
                // Flush in order
                loop {
                    let next = g.next_seq_to_flush;
                    if let Some(e) = g.pending.remove(&next) {
                        g.committed.push(e);
                        g.next_seq_to_flush += 1;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    pub fn committed(&self) -> Vec<Effect> {
        self.inner.lock().unwrap().committed.clone()
    }
}
