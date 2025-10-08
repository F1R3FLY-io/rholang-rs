#![cfg(feature = "parallel-exec")]
use std::sync::Arc;

use super::journal::{Effect, Journal};
use super::ready_queue::ReadyQueue;
use super::scheduler::Scheduler;
use super::work::WorkItem;
use crate::process::Process;
use crate::value::Value;

#[derive(Clone)]
pub struct VmBuilder {
    threads: Option<usize>,
    default_budget: u32,
}

impl Default for VmBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl VmBuilder {
    pub fn new() -> Self {
        Self {
            threads: None,
            default_budget: 10_000,
        }
    }
    pub fn threads(mut self, n: usize) -> Self {
        self.threads = Some(n);
        self
    }
    pub fn default_budget(mut self, b: u32) -> Self {
        self.default_budget = b;
        self
    }
    pub fn build(self) -> VmParallel {
        VmParallel::with_config(
            self.threads.unwrap_or_else(num_cpus::get),
            self.default_budget,
        )
    }
}

#[derive(Clone)]
pub struct VmParallel {
    threads: usize,
    budget: u32,
    rq: ReadyQueue,
    journal: Journal,
    next_pid: u64,
}

impl VmParallel {
    pub fn builder() -> VmBuilder {
        VmBuilder::new()
    }

    pub fn with_config(threads: usize, budget: u32) -> Self {
        let rq = ReadyQueue::new();
        let first_seq = 1; // ReadyQueue starts at seq=1; first enqueued work will take 1
        Self {
            threads,
            budget,
            rq: rq.clone(),
            journal: Journal::new(first_seq),
            next_pid: 1,
        }
    }

    pub fn spawn_process(&mut self, process: Arc<Process>) -> u64 {
        let pid = self.next_pid;
        self.next_pid += 1;
        let item = WorkItem::new(pid, process, self.budget);
        self.rq.enqueue(item);
        pid
    }

    pub fn run_until_quiescence(&self) -> Vec<(u64, Value)> {
        let sched = Scheduler::new(self.threads);
        sched.run(self.rq.clone(), self.journal.clone());
        // Map committed effects to (pid, value) ordered by seq due to journal
        self.journal
            .committed()
            .into_iter()
            .filter_map(|e| match e {
                Effect::Output { pid, value, .. } => Some((pid, value)),
            })
            .collect()
    }
}
