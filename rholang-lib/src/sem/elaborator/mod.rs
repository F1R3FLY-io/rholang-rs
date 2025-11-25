//! Elaborator performs semantic checks that go beyond basic scope resolution.
//! It **requires** that `ResolverPass` has already run to build scopes and resolve
//! The elaborator is stateless and can be reused for multiple for-comprehensions.
//! ## Usage
//!
//! ```ignore
//! let db = SemanticDb::new();
//! let pid = db.build_index(&ast);
//!
//! // Step 1: Run ResolverPass (REQUIRED)
//! let resolver = ResolverPass::new(pid);
//! resolver.run(&mut db);
//!
//! // Step 2: Run elaborator
//! let elaborator = ForCompElaborator::new(&db);
//! let diagnostics = elaborator.elaborate(pid);
//! ```

use crate::sem::{Diagnostic, PID, SemanticDb};
use rholang_parser::ast::{Proc, Receipts};

pub mod arrow_validator;

pub struct ForCompElaborator<'a, 'ast> {
    db: &'a SemanticDb<'ast>,
}

impl<'a, 'ast> ForCompElaborator<'a, 'ast> {
    pub fn new(db: &'a SemanticDb<'ast>) -> Self {
        Self { db }
    }

    /// Elaborate a for-comprehension
    ///
    /// Prerequisites: ResolverPass MUST have run
    ///
    /// # Panics
    ///
    /// Panics if `pid` is invalid or does not refer to a for-comprehension.
    /// This indicates a programming error in the semantic analyzer.
    pub fn elaborate(self, pid: PID) -> Vec<Diagnostic> {
        self.verify_for_comprehension(pid);

        let mut diagnostics = Vec::new();
        self.validate_arrow_types(pid, &mut diagnostics);

        diagnostics
    }

    /// Elaborate a for-comprehension with pre-extracted receipts
    pub fn elaborate_with_receipts(
        self,
        pid: PID,
        receipts: &'ast Receipts<'ast>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        self.validate_arrow_types_with_receipts(pid, receipts, &mut diagnostics);

        diagnostics
    }

    /// Verify that PID is a for-comprehension
    ///
    /// # Panics
    ///
    /// Panics if PID is invalid or does not refer to a for-comprehension.
    /// This is a programming error in the semantic analyzer.
    fn verify_for_comprehension(&self, pid: PID) {
        let proc_ref = self
            .db
            .get(pid)
            .expect("ForCompElaborator called with invalid PID");

        assert!(
            matches!(proc_ref.proc, Proc::ForComprehension { .. }),
            "ForCompElaborator called on non-for-comprehension node at PID {pid}"
        );
    }

    /// Validate arrow type homogeneity
    ///
    /// # Panics
    ///
    /// Panics if PID is invalid or does not refer to a for-comprehension.
    /// This is a programming error in the semantic analyzer.
    fn validate_arrow_types(&self, pid: PID, diagnostics: &mut Vec<Diagnostic>) {
        let proc = self
            .db
            .get(pid)
            .expect("validate_arrow_types called with invalid PID");

        match proc.proc {
            Proc::ForComprehension { receipts, .. } => {
                let validator = arrow_validator::ArrowTypeValidator::new(self.db);
                validator.validate(pid, receipts, diagnostics);
            }
            _ => panic!("validate_arrow_types called on non-for-comprehension node at PID {pid}"),
        }
    }

    /// Validate arrow type homogeneity with pre-extracted receipts
    fn validate_arrow_types_with_receipts(
        &self,
        pid: PID,
        receipts: &'ast Receipts<'ast>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let validator = arrow_validator::ArrowTypeValidator::new(self.db);
        validator.validate(pid, receipts, diagnostics);
    }
}

/// Pipeline pass for for-comprehension elaboration
pub struct ForCompElaborationPass;

impl crate::sem::Pass for ForCompElaborationPass {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("ForCompElaborationPass")
    }
}

impl crate::sem::DiagnosticPass for ForCompElaborationPass {
    fn run(&self, db: &SemanticDb) -> Vec<Diagnostic> {
        // Find all for-comprehensions and elaborate them directly
        db.filter_procs(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .flat_map(|(pid, proc_ref)| {
                if let Proc::ForComprehension { receipts, .. } = proc_ref.proc {
                    let elaborator = ForCompElaborator::new(db);
                    elaborator.elaborate_with_receipts(pid, receipts)
                } else {
                    // Should never happen due to filter, but handle defensively
                    Vec::new()
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::{DiagnosticPass, SemanticDb};
    use rholang_parser::ast::AnnProc;
    use test_macros::test_rholang_code;

    #[test_rholang_code("for(@x <- ch1 & @y <- ch2) { Nil }")]
    fn test_pass_finds_single_for_comprehension(_procs: &[AnnProc], db: &SemanticDb) {
        let pass = ForCompElaborationPass;
        let diagnostics = pass.run(db);

        // Should have no errors for valid for-comprehension
        assert_eq!(diagnostics.len(), 0);
    }

    #[test_rholang_code(
        "for(@x <- ch1) { Nil } | for(@y <= ch2) { Nil } | for(@z <<- ch3) { Nil }"
    )]
    fn test_pass_finds_multiple_for_comprehensions(_procs: &[AnnProc], db: &SemanticDb) {
        let pass = ForCompElaborationPass;
        let diagnostics = pass.run(db);

        // All three for-comprehensions are valid, no errors expected
        assert_eq!(diagnostics.len(), 0);
    }

    #[test_rholang_code("for(@x <- ch1 & @y <= ch2) { Nil }")]
    fn test_pass_emits_diagnostic_for_mixed_arrows(_procs: &[AnnProc], db: &SemanticDb) {
        let pass = ForCompElaborationPass;
        let diagnostics = pass.run(db);

        // Should emit diagnostic for mixed arrow types
        assert_eq!(diagnostics.len(), 1);
    }

    #[test_rholang_code("for(@x <- ch1 & @y <= ch2) { Nil } | for(@a <= ch3 & @b <<- ch4) { Nil }")]
    fn test_pass_emits_multiple_diagnostics(_procs: &[AnnProc], db: &SemanticDb) {
        let pass = ForCompElaborationPass;
        let diagnostics = pass.run(db);

        // Should emit 2 diagnostics, one for each invalid for-comprehension
        assert_eq!(diagnostics.len(), 2);
    }

    #[test_rholang_code("for(@x <- ch1) { for(@y <- ch2 & @z <- ch3) { Nil } }")]
    fn test_pass_handles_nested_for_comprehensions(_procs: &[AnnProc], db: &SemanticDb) {
        let pass = ForCompElaborationPass;
        let diagnostics = pass.run(db);

        // Both for-comprehensions are valid
        assert_eq!(diagnostics.len(), 0);
    }

    #[test_rholang_code("Nil | stdout!(\"hello\")")]
    fn test_pass_handles_no_for_comprehensions(_procs: &[AnnProc], db: &SemanticDb) {
        let pass = ForCompElaborationPass;
        let diagnostics = pass.run(db);

        // No for-comprehensions, so no diagnostics
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_pass_name() {
        use crate::sem::Pass;
        let pass = ForCompElaborationPass;
        let name = pass.name();
        assert_eq!(name, "ForCompElaborationPass");
    }

    #[test_rholang_code("for(@x <- ch1 & @y <- ch2) { Nil }")]
    fn test_elaborator_valid_for_comprehension(_procs: &[AnnProc], db: &SemanticDb) {
        // Find the for-comprehension PID
        let for_comp_pid = db
            .filter_procs(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .next()
            .expect("should find for-comprehension");

        let elaborator = ForCompElaborator::new(db);
        let diagnostics = elaborator.elaborate(for_comp_pid);

        assert_eq!(diagnostics.len(), 0);
    }

    #[test_rholang_code("for(@x <- ch1 & @y <= ch2) { Nil }")]
    fn test_elaborator_mixed_arrows_error(_procs: &[AnnProc], db: &SemanticDb) {
        // Find the for-comprehension PID
        let for_comp_pid = db
            .filter_procs(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .next()
            .expect("should find for-comprehension");

        let elaborator = ForCompElaborator::new(db);
        let diagnostics = elaborator.elaborate(for_comp_pid);

        assert_eq!(diagnostics.len(), 1);
    }
}
