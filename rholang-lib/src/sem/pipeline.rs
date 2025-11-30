use super::{DiagnosticPass, FactPass, Pass, SemanticDb};
use as_any::Downcast;
use nonempty_collections::NEVec;
use std::{borrow::Cow, fmt, num::NonZeroUsize};

pub struct Pipeline {
    passes: Vec<Box<dyn Pass>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn add_fact<F: FactPass>(mut self, pass: F) -> Self {
        self.passes.push(Box::new(FactPassWrapper::new(pass)));
        self
    }

    pub fn add_diagnostic<D: DiagnosticPass>(mut self, pass: D) -> Self {
        if let Some(diag_group) = self
            .passes
            .last_mut()
            .and_then(|pass| pass.as_mut().downcast_mut::<DiagnosticGroup>())
        {
            diag_group.push(pass)
        } else {
            self.passes.push(Box::new(DiagnosticGroup::single(pass)))
        }

        self
    }

    pub fn add_diagnostic_ungrouped<D: DiagnosticPass>(mut self, pass: D) -> Self {
        self.passes.push(Box::new(DiagnosticPassWrapper::new(pass)));
        self
    }

    pub async fn run(&self, db: &mut super::SemanticDb<'_>) {
        let mut all_diags = Vec::new();

        for pass in &self.passes {
            // Try FactPass
            if let Some(fact) = pass.as_any().downcast_ref::<FactPassWrapper>() {
                fact.run(db);
                continue;
            }

            // Try DiagnosticGroup
            if let Some(diag_group) = pass.as_any().downcast_ref::<DiagnosticGroup>() {
                let diags = diag_group.run_async(db).await;
                all_diags.extend(diags);
                continue;
            }

            // Try standalone diagnostic
            if let Some(diag) = pass.as_any().downcast_ref::<DiagnosticPassWrapper>() {
                let diags = diag.run(db);
                all_diags.extend(diags);
                continue;
            }

            panic!("unknown pass type: {}", pass.name())
        }

        db.push_diagnostics(all_diags);
    }

    /// Produces a tree-like textual description of all passes.
    pub fn describe(&self) -> String {
        use std::fmt::Write;

        let mut out = String::new();
        let mut iter = self.passes.iter().peekable();

        // helper: render a diagnostic group
        fn render_group(out: &mut String, group: &DiagnosticGroup, indent: &str) {
            let group_iter = group.passes.iter();
            let group_len = group_iter.len();
            for (i, diag) in group_iter.enumerate() {
                let connector = if i + 1 == group_len {
                    "└─"
                } else {
                    "├─"
                };
                let _ = writeln!(out, "{indent}{connector} {:<22} (Diagnostic)", diag.name());
            }
        }

        while let Some(pass) = iter.next() {
            // Try to downcast to a FactPass
            if let Some(fact) = pass.as_any().downcast_ref::<FactPassWrapper>() {
                let _ = writeln!(out, "{:<25} (Fact)", fact.name());

                // If the next pass is a diagnostic group, display its members under this fact
                if let Some(next) = iter.peek()
                    && let Some(group) = next.as_any().downcast_ref::<DiagnosticGroup>()
                {
                    render_group(&mut out, group, " ");
                    iter.next(); // consume group
                }
            }
            // Diagnostic group that stands on its own
            else if let Some(group) = pass.as_any().downcast_ref::<DiagnosticGroup>() {
                render_group(&mut out, group, "");
            }
            // Single ungrouped diagnostic pass
            else if let Some(diag) = pass.as_any().downcast_ref::<DiagnosticPassWrapper>() {
                let _ = writeln!(out, "{:<25} (Diagnostic)", diag.name());
            }
            // Unknown or unrecognized type
            else {
                let _ = writeln!(out, "{:<25} (Unknown)", pass.name());
            }
        }

        out
    }
}

impl fmt::Display for Pipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.describe())
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Diagnostic passes run in parallel
struct DiagnosticGroup {
    passes: NEVec<Box<dyn DiagnosticPass>>,
}

impl DiagnosticGroup {
    fn single<D: DiagnosticPass>(pass: D) -> Self {
        Self {
            passes: NEVec::new(Box::new(pass)),
        }
    }

    fn push<D: DiagnosticPass>(&mut self, pass: D) {
        self.passes.push(Box::new(pass));
    }

    /// Run all diagnostics concurrently (native) or sequentially (wasm)
    #[cfg(not(target_arch = "wasm32"))]
    async fn run_async<'d>(&self, db: &SemanticDb<'d>) -> Vec<super::Diagnostic> {
        if self.passes.len() == NonZeroUsize::MIN {
            return self.passes.first().run(db);
        }

        let mut all = Vec::new();
        let (_, results) = async_scoped::TokioScope::scope_and_block(|scope| {
            for pass in &self.passes {
                scope.spawn(async { pass.run(db) });
            }
        });

        for res in results {
            match res {
                Ok(diags) => all.extend(diags),
                Err(err) if err.is_panic() => std::panic::resume_unwind(err.into_panic()),
                Err(err) => panic!("diagnostic task failed: {err}"),
            }
        }

        all
    }

    #[cfg(target_arch = "wasm32")]
    async fn run_async<'d>(&self, db: &SemanticDb<'d>) -> Vec<super::Diagnostic> {
        // No multi-threading on wasm; run sequentially
        let mut all = Vec::new();
        for pass in &self.passes {
            all.extend(pass.run(db));
        }
        all
    }
}

impl Pass for DiagnosticGroup {
    fn name(&self) -> Cow<'static, str> {
        let nz_len = self.passes.len();

        if nz_len == NonZeroUsize::MIN {
            Cow::Owned(format!("DiagnosticGroup[{}]", self.passes.first().name()))
        } else {
            let joined = self
                .passes
                .iter()
                .map(|p| p.name())
                .collect::<Vec<_>>()
                .join(", ");
            Cow::Owned(format!("DiagnosticGroup({nz_len}): [{}]", joined))
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl DiagnosticPass for DiagnosticGroup {
    fn run(&self, db: &SemanticDb) -> Vec<super::Diagnostic> {
        tokio::runtime::Handle::current().block_on(self.run_async(db))
    }
}

#[cfg(target_arch = "wasm32")]
impl DiagnosticPass for DiagnosticGroup {
    fn run(&self, db: &SemanticDb) -> Vec<super::Diagnostic> {
        // Synchronous sequential execution on wasm
        let mut all = Vec::new();
        for pass in &self.passes {
            all.extend(pass.run(db));
        }
        all
    }
}

struct FactPassWrapper {
    pass: Box<dyn FactPass>,
}

impl FactPassWrapper {
    fn new<F: FactPass>(pass: F) -> Self {
        Self {
            pass: Box::new(pass),
        }
    }
}

impl Pass for FactPassWrapper {
    fn name(&self) -> Cow<'static, str> {
        self.pass.name()
    }
}

impl FactPass for FactPassWrapper {
    fn run(&self, db: &mut super::SemanticDb) {
        self.pass.run(db);
    }
}

struct DiagnosticPassWrapper {
    pass: Box<dyn DiagnosticPass>,
}

impl DiagnosticPassWrapper {
    fn new<D: DiagnosticPass>(pass: D) -> Self {
        Self {
            pass: Box::new(pass),
        }
    }
}

impl Pass for DiagnosticPassWrapper {
    fn name(&self) -> Cow<'static, str> {
        self.pass.name()
    }
}

impl DiagnosticPass for DiagnosticPassWrapper {
    fn run(&self, db: &super::SemanticDb) -> Vec<super::Diagnostic> {
        self.pass.run(db)
    }
}
