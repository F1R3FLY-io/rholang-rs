#![cfg(feature = "parallel-exec")]
use crossbeam::queue::SegQueue;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::work::WorkItem;

#[derive(Clone)]
pub struct ReadyQueue {
    seq: Arc<AtomicU64>,
    q: Arc<SegQueue<WorkItem>>,
}

impl ReadyQueue {
    pub fn new() -> Self {
        Self { seq: Arc::new(AtomicU64::new(1)), q: Arc::new(SegQueue::new()) }
    }

    pub fn next_seq(&self) -> u64 { self.seq.fetch_add(1, Ordering::SeqCst) }

    pub fn enqueue(&self, mut item: WorkItem) {
        if item.seq == 0 { item.seq = self.next_seq(); }
        self.q.push(item);
    }

    pub fn try_pop(&self) -> Option<WorkItem> { self.q.pop() }

    pub fn is_empty(&self) -> bool { self.q.is_empty() }
}
