use crate::process::Process;
use std::sync::Arc;

#[derive(Clone)]
pub struct WorkItem {
    pub pid: u64,
    pub process: Arc<Process>,
    pub ip: usize,
    pub budget: u32,
    pub seq: u64,
}

impl WorkItem {
    pub fn new(pid: u64, process: Arc<Process>, budget: u32) -> Self {
        Self {
            pid,
            process,
            ip: 0,
            budget,
            seq: 0,
        }
    }
}
