use std::borrow::Cow;

mod builder;
use super::PID;
use builder::Builder;

/// Computes the *enclosure relation* between processes and their nearest enclosing scope.
///
/// This pass performs a depth-first traversal of the process tree,
/// maintaining a stack of active scopes. Whenever a process is entered,
/// it records which scope encloses it.
///
/// ```text
/// Example process structure:
///
///     P0 ──┐        (top-level)
///          │
///          ├── P1 (introduces new scope S1)
///          │     ├── P2
///          │     └── P3 (introduces new scope S2)
///          │           └── P4
///          └── P5
///
/// Resulting `enclosing_pids` table:
///
///     PID   Enclosing Scope
///     ─────────────────────
///     P0 → TOP_LEVEL
///     P1 → TOP_LEVEL
///     P2 → P1
///     P3 → P1
///     P4 → P3
///     P5 → TOP_LEVEL
///
/// Legend:
///   - Arrows denote "is enclosed by"
///   - Only processes introducing scopes push/pop the stack
/// ```
///
/// The algorithm runs in O(n) time and requires a single linear traversal.
///
pub struct EnclosureAnalysisPass {
    root: PID,
}

impl EnclosureAnalysisPass {
    pub fn new(root: PID) -> Self {
        Self { root }
    }
}

impl super::Pass for EnclosureAnalysisPass {
    fn name(&self) -> Cow<'static, str> {
        Cow::Owned(format!("EnclosureAnalysis({})", self.root))
    }
}

impl super::FactPass for EnclosureAnalysisPass {
    fn run(&self, db: &mut super::SemanticDb) {
        db.enclosing_pids.resize(db.pid_count(), PID::TOP_LEVEL);

        let builder = Builder::new();
        let root_proc = db[self.root];
        builder.build(db, root_proc);
    }
}
