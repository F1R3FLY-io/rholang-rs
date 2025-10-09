//! Pipeline integration for for-comprehension elaboration
//!
//! This module provides the bridge between the for-comprehension elaborator
//! and the semantic analysis pipeline. It implements the `FactPass` trait
//! to integrate elaboration into the standard pipeline workflow.

use super::ForComprehensionElaborator;
use crate::sem::{FactPass, Pass, SemanticDb};
use std::borrow::Cow;

/// A fact pass that elaborates all for-comprehensions in the semantic database.
///
/// This pass iterates over all for-comprehension processes indexed in the database
/// and performs semantic elaboration on each one. It emits diagnostics for any
/// validation errors or warnings encountered.
///
/// # Implementation Status
///
/// Currently implements:
/// - **Phase 1.3**: Pre-validation (AST completeness, PID validation, child indexing)
/// - **Phase 2.1**: Pattern analysis (via pattern visitors)
///
/// Future phases (2.2-4.5) will be added as the elaborator is extended.
///
/// # Pipeline Integration
///
/// This pass should be added to the pipeline after any passes that build the
/// process index, but before diagnostic passes that depend on for-comprehension
/// validation results.
///
/// # Example
///
/// ```ignore
/// use rholang_lib::sem::{SemanticDb, Pipeline};
/// use rholang_lib::sem::elaborator::ForCompElaborationPass;
///
/// let mut db = SemanticDb::new();
/// // ... build index ...
///
/// let pipeline = Pipeline::new()
///     .add_fact(ForCompElaborationPass)
///     // ... other passes ...
///     ;
///
/// pipeline.run(&mut db).await;
/// ```
pub struct ForCompElaborationPass;

impl Pass for ForCompElaborationPass {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("ForCompElaboration")
    }
}

impl FactPass for ForCompElaborationPass {
    fn run(&self, db: &mut SemanticDb) {
        // Collect all for-comprehension PIDs first to avoid borrowing issues
        // during elaboration (elaborator needs mutable access to db)
        let for_comp_pids: Vec<_> = db.iter_for_comprehensions().map(|(pid, _)| pid).collect();

        // Elaborate each for-comprehension
        for pid in for_comp_pids {
            // Create a new elaborator for each for-comprehension
            // This ensures clean state for each elaboration
            let elaborator = ForComprehensionElaborator::new(db);

            // Elaborate and emit diagnostics
            // Errors are logged to the database, we continue processing other for-comps
            let _ = elaborator.elaborate_and_finalize(pid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::SemanticDb;
    use rholang_parser::RholangParser;

    #[test]
    fn test_for_comp_elaboration_pass_basic() {
        let parser = RholangParser::new();
        let code = r#"for(x <- @"channel") { Nil }"#;
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let _pid = db.build_index(&ast[0]);

        // Run the elaboration pass
        let pass = ForCompElaborationPass;
        pass.run(&mut db);

        // Should not have errors for valid for-comprehension
        assert!(!db.has_errors(), "Valid for-comp should not produce errors");
    }

    #[test]
    fn test_for_comp_elaboration_pass_with_errors() {
        // This test would need a malformed AST, which is hard to construct
        // For now, we test that the pass runs without panicking
        let parser = RholangParser::new();
        let code = r#"for(x <- @"ch1"; y <- @"ch2") { x!(y) }"#;
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let _pid = db.build_index(&ast[0]);

        let pass = ForCompElaborationPass;
        pass.run(&mut db);

        // Should complete without panicking
    }

    #[test]
    fn test_for_comp_elaboration_pass_multiple_for_comps() {
        let parser = RholangParser::new();
        let code = r#"
            for(x <- @"ch1") { Nil } |
            for(y <- @"ch2") { Nil }
        "#;
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let _pid = db.build_index(&ast[0]);

        // Count how many for-comps we have
        let count = db.iter_for_comprehensions().count();
        assert_eq!(count, 2, "Should have 2 for-comprehensions");

        // Run the elaboration pass
        let pass = ForCompElaborationPass;
        pass.run(&mut db);

        // Should process both without errors
        assert!(
            !db.has_errors(),
            "Valid for-comps should not produce errors"
        );
    }

    #[test]
    fn test_for_comp_elaboration_pass_no_for_comps() {
        let parser = RholangParser::new();
        let code = "Nil | ()";
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let _pid = db.build_index(&ast[0]);

        // Should have no for-comps
        assert_eq!(db.iter_for_comprehensions().count(), 0);

        // Run the elaboration pass
        let pass = ForCompElaborationPass;
        pass.run(&mut db);

        // Should complete without errors
        assert!(!db.has_errors());
    }

    #[test]
    fn test_for_comp_elaboration_pass_nested() {
        let parser = RholangParser::new();
        let code = r#"
            for(x <- @"outer") {
                for(y <- @"inner") { Nil }
            }
        "#;
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let _pid = db.build_index(&ast[0]);

        // Should have 2 for-comps (outer and inner)
        let count = db.iter_for_comprehensions().count();
        assert_eq!(count, 2, "Should have 2 for-comprehensions (nested)");

        // Run the elaboration pass
        let pass = ForCompElaborationPass;
        pass.run(&mut db);

        // Should process both
        assert!(
            !db.has_errors(),
            "Valid nested for-comps should not produce errors"
        );
    }

    #[tokio::test]
    async fn test_pipeline_integration_basic() {
        use crate::sem::pipeline::Pipeline;

        let parser = RholangParser::new();
        let code = r#"for(x <- @"channel") { Nil }"#;
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let _pid = db.build_index(&ast[0]);

        // Create pipeline with for-comp elaboration pass
        let pipeline = Pipeline::new().add_fact(ForCompElaborationPass);

        // Run the pipeline
        pipeline.run(&mut db).await;

        // Should complete without errors
        assert!(!db.has_errors(), "Pipeline should complete without errors");
    }

    #[tokio::test]
    async fn test_pipeline_integration_multiple_for_comps() {
        use crate::sem::pipeline::Pipeline;

        let parser = RholangParser::new();
        let code = r#"
            for(x <- @"ch1") { Nil } |
            for(y <- @"ch2") { Nil } |
            for(z <- @"ch3") { Nil }
        "#;
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let _pid = db.build_index(&ast[0]);

        assert_eq!(db.iter_for_comprehensions().count(), 3);

        // Create and run pipeline
        let pipeline = Pipeline::new().add_fact(ForCompElaborationPass);
        pipeline.run(&mut db).await;

        // All three should be elaborated
        assert!(!db.has_errors());
    }

    #[tokio::test]
    async fn test_pipeline_pass_name() {
        let pass = ForCompElaborationPass;
        assert_eq!(pass.name(), "ForCompElaboration");
    }
}
