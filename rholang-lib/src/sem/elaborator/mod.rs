//! Elaborator performs semantic checks that go beyond basic scope resolution.
//! It **requires** that `ResolverPass` has already run to build scopes and resolve
//! The elaborator is stateless and can be reused for multiple for-comprehensions.
//! ## Usage
//!
//! ```ignore
//! let mut db = SemanticDb::new();
//! let pid = db.build_index(&ast);
//!
//! // Step 1: Run ResolverPass (REQUIRED)
//! let resolver = ResolverPass::new(pid);
//! resolver.run(&mut db);
//!
//! // Step 2: Run elaborator
//! let elaborator = ForCompElaborator::new(&mut db);
//! elaborator.elaborate_and_finalize(pid)?;
//! ```

use crate::sem::{Diagnostic, ErrorKind, PID, SemanticDb};
use rholang_parser::ast::Proc;

pub mod arrow_validator;

pub struct ForCompElaborator<'a, 'ast> {
    db: &'a mut SemanticDb<'ast>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a, 'ast> ForCompElaborator<'a, 'ast> {
    pub fn new(db: &'a mut SemanticDb<'ast>) -> Self {
        Self {
            db,
            diagnostics: Vec::new(),
        }
    }

    /// Elaborate a for-comprehension
    ///
    /// Prerequisites: ResolverPass MUST have run
    pub fn elaborate_and_finalize(mut self, pid: PID) -> Result<(), Vec<Diagnostic>> {
        if let Some(diagnostic) = self.verify_for_comprehension(pid) {
            self.diagnostics.push(diagnostic);
            return self.finalize();
        }

        if let Some(diagnostic) = self.validate_arrow_types(pid) {
            self.diagnostics.push(diagnostic);
        }

        self.finalize()
    }

    /// Verify that PID is a for-comprehension
    fn verify_for_comprehension(&self, pid: PID) -> Option<Diagnostic> {
        let proc_ref = match self.db.get(pid) {
            Some(p) => p,
            None => return Some(Diagnostic::error(pid, ErrorKind::InvalidPid, None)),
        };

        match proc_ref.proc {
            Proc::ForComprehension { .. } => None,
            _ => Some(Diagnostic::error(
                pid,
                ErrorKind::IncompleteAstNode,
                Some(proc_ref.span.start),
            )),
        }
    }

    /// Validate arrow type homogeneity
    fn validate_arrow_types(&mut self, pid: PID) -> Option<Diagnostic> {
        let proc = self.db.get(pid)?;

        match proc.proc {
            Proc::ForComprehension { receipts, .. } => {
                let validator = arrow_validator::ArrowTypeValidator::new(self.db);
                validator.validate(pid, receipts)
            }
            _ => Some(Diagnostic::error(pid, ErrorKind::IncompleteAstNode, None)),
        }
    }

    /// Finalize and emit diagnostics
    fn finalize(self) -> Result<(), Vec<Diagnostic>> {
        for diagnostic in &self.diagnostics {
            self.db.emit_diagnostic(*diagnostic);
        }

        if !self.diagnostics.is_empty() {
            Err(self.diagnostics)
        } else {
            Ok(())
        }
    }
}

/// Pipeline pass for for-comprehension elaboration
pub struct ForCompElaborationPass {
    root: PID,
}

impl ForCompElaborationPass {
    pub fn new(root: PID) -> Self {
        Self { root }
    }

    pub fn root(&self) -> PID {
        self.root
    }
}

impl crate::sem::Pass for ForCompElaborationPass {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(format!("ForCompElaborationPass({})", self.root))
    }
}

impl crate::sem::FactPass for ForCompElaborationPass {
    fn run(&self, db: &mut SemanticDb) {
        // Find all for-comprehensions in the subtree
        let for_comprehensions: Vec<PID> = db
            .filter_procs(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .collect();

        // Elaborate each for-comprehension
        // Errors are already emitted as diagnostics; we don't propagate them
        // because this pass should not halt the entire pipeline
        for for_comp_pid in for_comprehensions {
            let elaborator = ForCompElaborator::new(db);
            let _ = elaborator.elaborate_and_finalize(for_comp_pid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::{FactPass, SemanticDb};
    use rholang_parser::RholangParser;

    /// Helper to count diagnostics in the database
    fn count_diagnostics(db: &SemanticDb) -> usize {
        db.diagnostics().len()
    }

    #[test]
    fn test_pass_finds_single_for_comprehension() {
        let parser = RholangParser::new();
        let ast = parser
            .parse("for(@x <- ch1 & @y <- ch2) { Nil }")
            .expect("parse error");
        let mut db = SemanticDb::new();
        let pid = db.build_index(&ast[0]);

        let pass = ForCompElaborationPass::new(pid);
        pass.run(&mut db);

        // Should have no errors for valid for-comprehension
        assert_eq!(count_diagnostics(&db), 0);
    }

    #[test]
    fn test_pass_finds_multiple_for_comprehensions() {
        let parser = RholangParser::new();
        let code = r#"
            for(@x <- ch1) { Nil } |
            for(@y <= ch2) { Nil } |
            for(@z <<- ch3) { Nil }
        "#;
        let ast = parser.parse(code).expect("parse error");
        let mut db = SemanticDb::new();
        let pid = db.build_index(&ast[0]);

        let pass = ForCompElaborationPass::new(pid);
        pass.run(&mut db);

        // All three for-comprehensions are valid, no errors expected
        assert_eq!(count_diagnostics(&db), 0);
    }

    #[test]
    fn test_pass_emits_diagnostic_for_mixed_arrows() {
        let parser = RholangParser::new();
        let ast = parser
            .parse("for(@x <- ch1 & @y <= ch2) { Nil }")
            .expect("parse error");
        let mut db = SemanticDb::new();
        let pid = db.build_index(&ast[0]);

        let pass = ForCompElaborationPass::new(pid);
        pass.run(&mut db);

        // Should emit diagnostic for mixed arrow types
        assert_eq!(count_diagnostics(&db), 1);
    }

    #[test]
    fn test_pass_emits_multiple_diagnostics() {
        let parser = RholangParser::new();
        let code = r#"
            for(@x <- ch1 & @y <= ch2) { Nil } |
            for(@a <= ch3 & @b <<- ch4) { Nil }
        "#;
        let ast = parser.parse(code).expect("parse error");
        let mut db = SemanticDb::new();
        let pid = db.build_index(&ast[0]);

        let pass = ForCompElaborationPass::new(pid);
        pass.run(&mut db);

        // Should emit 2 diagnostics, one for each invalid for-comprehension
        assert_eq!(count_diagnostics(&db), 2);
    }

    #[test]
    fn test_pass_handles_nested_for_comprehensions() {
        let parser = RholangParser::new();
        let code = r#"
            for(@x <- ch1) {
                for(@y <- ch2 & @z <- ch3) { Nil }
            }
        "#;
        let ast = parser.parse(code).expect("parse error");
        let mut db = SemanticDb::new();
        let pid = db.build_index(&ast[0]);

        let pass = ForCompElaborationPass::new(pid);
        pass.run(&mut db);

        // Both for-comprehensions are valid
        assert_eq!(count_diagnostics(&db), 0);
    }

    #[test]
    fn test_pass_handles_no_for_comprehensions() {
        let parser = RholangParser::new();
        let ast = parser
            .parse("Nil | stdout!(\"hello\")")
            .expect("parse error");
        let mut db = SemanticDb::new();
        let pid = db.build_index(&ast[0]);

        let pass = ForCompElaborationPass::new(pid);
        pass.run(&mut db);

        // No for-comprehensions, so no diagnostics
        assert_eq!(count_diagnostics(&db), 0);
    }

    #[test]
    fn test_pass_name_includes_pid() {
        use crate::sem::Pass;
        let pass = ForCompElaborationPass::new(PID(42));
        let name = pass.name();
        assert!(name.contains("ForCompElaborationPass"));
        assert!(name.contains("42"));
    }

    #[test]
    fn test_elaborator_invalid_pid() {
        let mut db = SemanticDb::new();
        let elaborator = ForCompElaborator::new(&mut db);

        // Use a non-existent PID
        let result = elaborator.elaborate_and_finalize(PID(9999));

        assert!(result.is_err());
        let diagnostics = result.unwrap_err();
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_elaborator_non_for_comprehension_node() {
        let parser = RholangParser::new();
        let ast = parser.parse("Nil").expect("parse error");
        let mut db = SemanticDb::new();
        let pid = db.build_index(&ast[0]);

        let elaborator = ForCompElaborator::new(&mut db);
        let result = elaborator.elaborate_and_finalize(pid);

        assert!(result.is_err());
        let diagnostics = result.unwrap_err();
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_elaborator_valid_for_comprehension() {
        let parser = RholangParser::new();
        let ast = parser
            .parse("for(@x <- ch1 & @y <- ch2) { Nil }")
            .expect("parse error");
        let mut db = SemanticDb::new();
        let _root = db.build_index(&ast[0]);

        // Find the for-comprehension PID
        let for_comp_pid = db
            .filter_procs(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .next()
            .expect("should find for-comprehension");

        let elaborator = ForCompElaborator::new(&mut db);
        let result = elaborator.elaborate_and_finalize(for_comp_pid);

        assert!(result.is_ok());
    }

    #[test]
    fn test_elaborator_mixed_arrows_error() {
        let parser = RholangParser::new();
        let ast = parser
            .parse("for(@x <- ch1 & @y <= ch2) { Nil }")
            .expect("parse error");
        let mut db = SemanticDb::new();
        let _root = db.build_index(&ast[0]);

        // Find the for-comprehension PID
        let for_comp_pid = db
            .filter_procs(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .next()
            .expect("should find for-comprehension");

        let elaborator = ForCompElaborator::new(&mut db);
        let result = elaborator.elaborate_and_finalize(for_comp_pid);

        assert!(result.is_err());
        let diagnostics = result.unwrap_err();
        assert_eq!(diagnostics.len(), 1);
    }
}
