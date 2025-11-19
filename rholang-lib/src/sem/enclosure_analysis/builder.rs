use rholang_parser::DfsEventExt;
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
        fn set_enclosing<'x>(db: &mut SemanticDb<'x>, proc: ProcRef<'x>, enclosing: PID) -> PID {
            let pid = db[proc];
            db.enclosing_pids[pid.0 as usize] = enclosing;

            pid
        }

        let mut iter = root.iter_dfs_event_with_names();
        for event in &mut iter {
            match event {
                DfsEventExt::Enter(proc) => {
                    let pid = set_enclosing(db, proc, self.current);

                    // Does this process *introduce* a new scope?
                    if db.is_scoped(pid) {
                        self.stack.push(self.current);
                        self.current = pid;
                    }
                }

                DfsEventExt::Name(name) => {
                    if let Some(quoted) = name.as_quote()
                        && db.contains(quoted)
                    {
                        quoted.iter_preorder_dfs().for_each(|proc| {
                            set_enclosing(db, proc, self.current);
                        });
                    }
                }

                DfsEventExt::Exit(proc) => {
                    let pid = db[proc];
                    if db.is_scoped(pid) {
                        // leaving that scope
                        self.current = self.stack.pop().expect("unbalanced scopes");
                    }
                }
            }
        }
    }
}
