use crate::sem::{Diagnostic, DiagnosticPass, Pass, SemanticDb, WarningKind};
use std::borrow::Cow;

impl Pass for super::UnusedVarsPass {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("UnusedVar")
    }
}

impl DiagnosticPass for super::UnusedVarsPass {
    fn run(&self, db: &SemanticDb) -> Vec<Diagnostic> {
        let mut result = Vec::new();
        db.scopes_full()
            .filter(|(_, scope)| !scope.all_used())
            .for_each(|(pid, scope)| {
                let unused = scope.unused();
                result.extend(unused.map(|bid| {
                    let unused_binder = &db[bid];
                    Diagnostic::warning(
                        pid,
                        WarningKind::UnusedVariable(bid, unused_binder.name),
                        Some(unused_binder.source_position),
                    )
                }));
            });

        result
    }
}
