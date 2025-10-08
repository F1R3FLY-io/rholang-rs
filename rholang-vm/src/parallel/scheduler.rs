use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;

use super::journal::{Effect, Journal};
use super::ready_queue::ReadyQueue;
use crate::vm::VM;

pub struct Scheduler {
    pub threads: usize,
}

impl Scheduler {
    pub fn new(threads: usize) -> Self {
        Self { threads }
    }

    pub fn run(&self, rq: ReadyQueue, journal: Journal) {
        let stop = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::with_capacity(self.threads);
        for _ in 0..self.threads {
            let rq_cl = rq.clone();
            let j_cl = journal.clone();
            let stop_cl = stop.clone();
            let handle = thread::spawn(move || {
                let mut vm = VM::new();
                while !stop_cl.load(Ordering::SeqCst) {
                    if let Some(item) = rq_cl.try_pop() {
                        let mut proc = (*item.process).clone();
                        match vm.execute(&mut proc) {
                            Ok(value) => {
                                j_cl.commit(Effect::Output {
                                    pid: item.pid,
                                    seq: item.seq,
                                    value,
                                });
                            }
                            Err(_) => {
                                // For MVP, ignore error details and still commit Nil to keep seq progress deterministic
                                j_cl.commit(Effect::Output {
                                    pid: item.pid,
                                    seq: item.seq,
                                    value: crate::value::Value::Nil,
                                });
                            }
                        }
                    } else {
                        // Back off to avoid busy spin
                        std::thread::yield_now();
                    }
                }
            });
            handles.push(handle);
        }

        // Wait until the queue is empty and then signal workers to stop
        while !rq.is_empty() {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        stop.store(true, Ordering::SeqCst);
        for h in handles {
            let _ = h.join();
        }
    }
}
