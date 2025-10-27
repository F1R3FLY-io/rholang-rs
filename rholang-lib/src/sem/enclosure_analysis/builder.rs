use rholang_parser::DfsEvent;
use smallvec::SmallVec;

use crate::sem::{PID, ProcRef, SemanticDb};

pub(super) struct Builder {
    current: PID,
    stack: SmallVec<[PID; 8]>,
}

impl Builder {
    pub(super) fn new() -> Self {
        Self {
            current: PID::TOP_LEVEL, // top-level sentinel
            stack: SmallVec::new(),
        }
    }

    pub(super) fn build<'db>(mut self, db: &mut SemanticDb<'db>, root: ProcRef<'db>) {
        let mut iter = root.iter_dfs_event();
        for event in &mut iter {
            match event {
                DfsEvent::Enter(proc) => {
                    let pid = db[proc];
                    db.enclosing_pids[pid.0 as usize] = self.current;

                    // Does this process *introduce* a new scope?
                    if db.get_scope(pid).is_some() {
                        self.stack.push(self.current);
                        self.current = pid;
                    }
                }

                DfsEvent::Exit(proc) => {
                    let pid = db[proc];
                    if db.get_scope(pid).is_some() {
                        // leaving that scope
                        self.current = self.stack.pop().expect("unbalanced scopes");
                    }
                }
            }
        }
    }
}
