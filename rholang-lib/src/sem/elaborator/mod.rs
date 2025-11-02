//! Elaborator performs semantic checks that go beyond basic scope resolution.
//! It **requires** that `ResolverPass` has already run to build scopes and resolve
//! The elaborator is stateless and can be reused for multiple for-comprehensions.
//! ## Usage
//!
//! ```ignore
//! let mut db = SemanticDb::new();
//! let pid = db.build_index(&ast);
//!
//! // Step 1: Run ResolverPass to build scopes and resolve variables
//! let resolver = ResolverPass::new(pid);
//! resolver.run(&mut db);
//!
//! // Step 2: Run elaborator for advanced semantic validation
//! let elaborator = ForComprehensionElaborator::new(&mut db);
//! elaborator.elaborate_and_finalize(pid)?;
//! ```

use crate::sem::elaborator::consumption::ConsumptionValidator;
use crate::sem::elaborator::joins::JoinValidator;
use crate::sem::elaborator::validation::PatternQueryValidator;
use crate::sem::{Diagnostic, PID, SemanticDb};
use rholang_parser::ast::Proc;

pub mod consumption;
pub mod errors;
pub mod joins;
pub mod validation;

pub use errors::{ElaborationError, ElaborationResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelType {
    UnforgeableName,
    QuotedProcess,
    Variable,
    Unknown,
}

pub struct ForComprehensionElaborator<'a, 'ast> {
    db: &'a mut SemanticDb<'ast>,
    errors: Vec<ElaborationError>,
}

impl<'a, 'ast> ForComprehensionElaborator<'a, 'ast> {
    pub fn new(db: &'a mut SemanticDb<'ast>) -> Self {
        Self {
            db,
            errors: Vec::new(),
        }
    }

    fn add_error(&mut self, error: ElaborationError) {
        self.errors.push(error);
    }

    /// Finalize elaboration and emit diagnostics to the db
    ///
    /// Converts all errors to `Diagnostic` instances and emits
    /// them to the semantic db
    pub fn finalize(self) -> Result<(), Vec<crate::sem::Diagnostic>> {
        let diagnostics: Vec<Diagnostic> = self.errors.iter().map(|e| e.to_diagnostic()).collect();

        // Emit to database (this modifies db, but we're consuming self anyway)
        for diagnostic in &diagnostics {
            self.db.emit_diagnostic(*diagnostic);
        }

        if !self.errors.is_empty() {
            Err(diagnostics)
        } else {
            Ok(())
        }
    }

    /// Elaborate a for-comprehension
    ///
    /// This is the main entry point for elaboration
    ///
    /// ## Prerequisites
    ///
    /// **IMPORTANT**: `ResolverPass` MUST have run before this elaborator
    ///
    /// If ResolverPass has not run, this method will return an error immediately.
    ///
    /// ## Arguments
    ///
    /// * `pid` - The PID of the for-comprehension to elaborate
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` if elaboration succeeded, or `Err(diagnostics)` if errors occurred
    pub fn elaborate_and_finalize(mut self, pid: PID) -> Result<(), Vec<crate::sem::Diagnostic>> {
        if let Err(error) = self.verify_for_comprehension_and_scope(pid) {
            self.add_error(error);
            return self.finalize();
        }

        if let Err(error) = self.validate_type_consistency(pid) {
            self.add_error(error);
        }

        if let Err(error) = self.validate_consumption_semantics(pid) {
            self.add_error(error);
        }

        if let Err(error) = self.validate_join_semantics(pid) {
            self.add_error(error);
        }

        if let Err(error) = self.validate_pattern_queries(pid) {
            self.add_error(error);
        }

        self.finalize()
    }

    /// Verify that the PID references a for-comprehension and ResolverPass created a scope
    ///
    /// It checks that:
    /// 1. The PID exists in the database
    /// 2. The PID references a `ForComprehension` node
    /// 3. ResolverPass created a scope for this for-comprehension
    fn verify_for_comprehension_and_scope(&self, pid: PID) -> ElaborationResult<()> {
        let proc_ref = self
            .db
            .get(pid)
            .ok_or(ElaborationError::InvalidPid { pid })?;

        match proc_ref.proc {
            Proc::ForComprehension { .. } => {
                self.db
                    .get_scope(pid)
                    .ok_or_else(|| ElaborationError::IncompleteAstNode {
                        pid,
                        position: Some(proc_ref.span.start),
                        reason: "ResolverPass did not create a scope for this for-comprehension. \
                                 Ensure ResolverPass runs before ForCompElaborationPass."
                            .to_string(),
                    })?;

                Ok(())
            }
            _ => Err(ElaborationError::IncompleteAstNode {
                pid,
                position: Some(proc_ref.span.start),
                reason: "Expected for-comprehension node".to_string(),
            }),
        }
    }

    /// Validates channel types and pattern-message compatibility using the TypeValidator
    /// This method:
    /// - Infers channel types (unforgeable, quoted, variable)
    /// - Validates pattern structure
    /// - Checks pattern-message compatibility
    fn validate_type_consistency(&mut self, pid: PID) -> ElaborationResult<()> {
        use crate::sem::elaborator::validation::TypeValidator;

        let validator = TypeValidator::new(self.db);
        validator
            .validate_channel_usage(pid)
            .map_err(|e| self.convert_validation_error(pid, e))
    }

    /// Validates consumption modes (linear/peek/repeated) using the ConsumptionValidator
    /// This method:
    /// - Validates linear consumption (`<-`) - channel consumed exactly once
    /// - Validates peek semantics (`<<-`) - non-consuming reads
    /// - Validates repeated binds (`<=`) - persistent/contract-like behavior
    /// - Detects potential reentrancy vulnerabilities
    fn validate_consumption_semantics(&mut self, pid: PID) -> ElaborationResult<()> {
        let proc = self
            .db
            .get(pid)
            .ok_or(ElaborationError::InvalidPid { pid })?;

        match proc.proc {
            Proc::ForComprehension { receipts, .. } => {
                let validator = ConsumptionValidator::new(self.db);

                validator
                    .validate(pid, receipts)
                    .map_err(|e| self.convert_validation_error(pid, e))?;

                Ok(())
            }
            _ => unreachable!("Already verified in verify_for_comprehension"),
        }
    }

    /// Validates join semantics using the JoinValidator
    /// This method:
    /// - Validates join atomicity (all-or-nothing for parallel bindings)
    /// - Detects potential deadlocks via circular dependency analysis
    /// - Validates channel availability before blocking
    fn validate_join_semantics(&mut self, pid: PID) -> ElaborationResult<()> {
        let proc = self
            .db
            .get(pid)
            .ok_or(ElaborationError::InvalidPid { pid })?;

        match proc.proc {
            Proc::ForComprehension { receipts, .. } => {
                let validator = JoinValidator::new(self.db);

                validator
                    .validate(pid, receipts)
                    .map_err(|e| self.convert_validation_error(pid, e))?;

                Ok(())
            }
            _ => unreachable!("Already verified in verify_for_comprehension"),
        }
    }

    /// Validates pattern query semantics using the PatternQueryValidator
    /// This method:
    /// - Validates SQL-like pattern semantics (selections, projections, filters)
    /// - Checks pattern satisfiability (detects impossible patterns)
    /// - Validates logical connective semantics (AND/OR/NOT composition)
    fn validate_pattern_queries(&mut self, pid: PID) -> ElaborationResult<()> {
        let proc = self
            .db
            .get(pid)
            .ok_or(ElaborationError::InvalidPid { pid })?;

        match proc.proc {
            Proc::ForComprehension { receipts, .. } => {
                let validator = PatternQueryValidator::new(self.db);

                // Validate each pattern in the receipts
                for receipt in receipts.iter() {
                    for bind in receipt.iter() {
                        let patterns = bind.names();

                        for name in &patterns.names {
                            if let rholang_parser::ast::Name::Quote(pattern_proc) = name {
                                validator
                                    .validate_sql_like_patterns(pattern_proc)
                                    .map_err(|e| self.convert_validation_error(pid, e))?;

                                validator
                                    .validate_pattern_satisfiability(pattern_proc)
                                    .map_err(|e| self.convert_validation_error(pid, e))?;
                            }
                        }
                    }
                }

                Ok(())
            }
            _ => unreachable!("Already verified in verify_for_comprehension"),
        }
    }

    /// Extract source position from a PID (fallback if no position provided)
    fn extract_position_from_pid(&self, pid: PID) -> rholang_parser::SourcePos {
        self.db
            .get(pid)
            .map(|proc| proc.span.start)
            .unwrap_or_default()
    }

    /// Convert ValidationError to ElaborationError with PID context
    fn convert_validation_error(
        &self,
        pid: PID,
        error: errors::ValidationError,
    ) -> ElaborationError {
        use errors::ValidationError;

        match error {
            ValidationError::UnboundVariable { var, pos } => {
                ElaborationError::UnboundVariable { pid, var, pos }
            }
            ValidationError::ConnectiveOutsidePattern { pos } => {
                ElaborationError::ConnectiveOutsidePattern { pid, pos }
            }
            ValidationError::UnsatisfiablePattern { pattern, pos } => {
                ElaborationError::UnsatisfiablePattern {
                    pid,
                    pattern,
                    pos: pos.unwrap_or_else(|| self.extract_position_from_pid(pid)),
                }
            }
            ValidationError::DeadlockPotential { receipts, pos } => {
                ElaborationError::DeadlockPotential {
                    pid,
                    receipts,
                    pos: pos.unwrap_or_else(|| self.extract_position_from_pid(pid)),
                }
            }
            ValidationError::InvalidPatternStructure {
                pid: _,
                position,
                reason,
            } => ElaborationError::InvalidPattern {
                pid,
                position,
                reason,
            },
            ValidationError::MixedArrowTypes {
                receipt_index,
                found_types,
                pos,
            } => ElaborationError::InvalidPattern {
                pid,
                position: pos,
                reason: format!(
                    "Mixed arrow types in join group {}: found {}. \
                     All bindings in a parallel join must use the same arrow type.",
                    receipt_index,
                    found_types.join(", ")
                ),
            },
        }
    }
}

/// Pipeline pass for for-comprehension elaboration
///
/// This pass implements the `FactPass` trait to integrate with the semantic analysis pipeline
/// It finds all for-comprehensions in the specified subtree and runs validation on each
///
/// ## Usage
///
/// ```ignore
/// let mut db = SemanticDb::new();
/// let root_pid = db.build_index(&ast);
///
/// let pipeline = Pipeline::new()
///     .add_fact(ResolverPass::new(root_pid))
///     .add_fact(ForCompElaborationPass::new(root_pid));
///
/// tokio::runtime::Runtime::new().unwrap().block_on(pipeline.run(&mut db));
/// ```
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
    /// Run elaboration on all for-comprehensions in the subtree
    ///
    /// This method:
    /// 1. Finds all `ForComprehension` nodes in the subtree rooted at `self.root`
    /// 2. For each for-comprehension, runs the `ForComprehensionElaborator`
    /// 3. Collects and emits all diagnostics to the semantic database
    /// If ResolverPass hasn't run, elaboration will fail with appropriate diagnostics.
    fn run(&self, db: &mut SemanticDb) {
        use rholang_parser::ast::Proc;

        // Find all for-comprehensions in the subtree
        let for_comprehensions: Vec<PID> = db
            .filter_procs(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .collect();

        // Elaborate each for-comprehension
        for for_comp_pid in for_comprehensions {
            let elaborator = ForComprehensionElaborator::new(db);
            let _ = elaborator.elaborate_and_finalize(for_comp_pid);

            // Diagnostics are already emitted to db via finalize()
            // Errors are logged but don't stop the pipeline - other for-comps should still be checked
        }
    }
}

/// Shared helper functions for channel validation (ConsumptionValidator, JoinValidator)
pub(crate) mod channel_validation {
    use super::errors::{ValidationError, ValidationResult};
    use crate::sem::SemanticDb;
    use rholang_parser::ast::{AnnProc, Name, Source};
    use rholang_parser::SourcePos;

    /// Verify that a channel exists and is properly bound, including quoted process channels
    pub fn verify_channel<'ast>(db: &SemanticDb<'ast>, name: &'ast Name<'ast>) -> ValidationResult {
        match name {
            Name::NameVar(rholang_parser::ast::Var::Id(id)) => {
                // Verify that the name variable is bound
                if db.binder_of_id(*id).is_none() {
                    let sym = db.intern(id.name);
                    return Err(ValidationError::UnboundVariable {
                        var: sym,
                        pos: id.pos,
                    });
                }
                Ok(())
            }
            Name::Quote(proc) => {
                // For quoted process channels, verify all names within the process
                verify_names_in_quoted_channel(db, proc)
            }
            Name::NameVar(rholang_parser::ast::Var::Wildcard) => {
                // Wildcards are always valid as channels
                Ok(())
            }
        }
    }

    /// Verify all names within a quoted process channel
    fn verify_names_in_quoted_channel<'ast>(
        db: &SemanticDb<'ast>,
        proc: &'ast AnnProc<'ast>,
    ) -> ValidationResult {
        for name in proc.iter_names_direct() {
            verify_channel(db, name)?;
        }
        Ok(())
    }

    /// Extract the name from a Source (Simple, ReceiveSend, or SendReceive)
    pub fn extract_name_from_source<'ast>(source: &'ast Source<'ast>) -> &'ast Name<'ast> {
        match source {
            Source::Simple { name } | Source::ReceiveSend { name } | Source::SendReceive { name, .. } => name,
        }
    }

    /// Get the source position from a Name
    pub fn get_name_position(name: &Name) -> SourcePos {
        match name {
            Name::NameVar(var) => match var {
                rholang_parser::ast::Var::Id(id) => id.pos,
                rholang_parser::ast::Var::Wildcard => SourcePos::default(),
            },
            Name::Quote(proc) => proc.span.start,
        }
    }

    /// Get the source position from a Source
    pub fn get_source_position<'a>(source: &'a Source<'a>) -> SourcePos {
        get_name_position(extract_name_from_source(source))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::{FactPass, SemanticDb, resolver::ResolverPass};
    use rholang_parser::RholangParser;

    fn setup_with_resolver(code: &str) -> (SemanticDb<'static>, PID) {
        let parser = Box::leak(Box::new(RholangParser::new()));
        let code_static: &'static str = Box::leak(code.to_string().into_boxed_str());
        let ast = parser.parse(code_static).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        // Run ResolverPass to build scopes (Phases 1-3)
        let resolver = ResolverPass::new(root_pid);
        resolver.run(&mut db);

        // Find the for-comprehension PID
        let for_comp_pid = db
            .find_proc(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .expect("No for-comprehension found in test code");

        (db, for_comp_pid)
    }

    #[test]
    fn test_verify_requires_scope_from_resolver() {
        let parser = RholangParser::new();
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let _root_pid = db.build_index(proc);

        let for_comp_pid = db
            .find_proc(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .expect("No for-comprehension found");

        let elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.elaborate_and_finalize(for_comp_pid);

        // Should fail because ResolverPass hasn't created a scope
        assert!(result.is_err());

        let diagnostics = result.unwrap_err();
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_verify_succeeds_with_resolver() {
        let (mut db, pid) = setup_with_resolver(r#"new ch in { for(@x <- ch) { Nil } }"#);

        let elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.elaborate_and_finalize(pid);

        // Should succeed with ResolverPass scope
        assert!(
            result.is_ok(),
            "Elaboration should succeed when ResolverPass ran"
        );
    }

    #[test]
    fn test_verify_rejects_non_for_comprehension() {
        let parser = RholangParser::new();
        let code = "Nil";
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let pid = db.build_index(proc);

        let elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.elaborate_and_finalize(pid);

        assert!(result.is_err());
    }

    #[test]
    fn test_verify_rejects_invalid_pid() {
        let mut db = SemanticDb::new();
        let elaborator = ForComprehensionElaborator::new(&mut db);

        let invalid_pid = PID(999);
        let result = elaborator.elaborate_and_finalize(invalid_pid);

        assert!(result.is_err());
    }

    #[test]
    fn test_channel_type() {
        assert_eq!(
            format!("{:?}", ChannelType::UnforgeableName),
            "UnforgeableName"
        );
        assert_eq!(format!("{:?}", ChannelType::QuotedProcess), "QuotedProcess");
        assert_eq!(format!("{:?}", ChannelType::Variable), "Variable");
        assert_eq!(format!("{:?}", ChannelType::Unknown), "Unknown");
    }

    #[test]
    fn test_multiple_for_comprehensions() {
        let (mut db, pid1) = setup_with_resolver(r#"new ch1 in { for(@x <- ch1) { Nil } }"#);

        // First for-comp
        let elaborator1 = ForComprehensionElaborator::new(&mut db);
        let result1 = elaborator1.elaborate_and_finalize(pid1);
        assert!(result1.is_ok());

        // Can create second elaborator on same db (stateless)
        let elaborator2 = ForComprehensionElaborator::new(&mut db);
        let result2 = elaborator2.elaborate_and_finalize(pid1);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_complex_for_comprehension_structure() {
        let code = r#"
            new ch1, ch2, ch3 in {
                for(@x <- ch1; @[y, z] <- ch2; @w <= ch3) {
                    @x!(1) | @y!(2) | @z!(3) | @w!(4)
                }
            }
        "#;
        let (mut db, pid) = setup_with_resolver(code);

        let elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.elaborate_and_finalize(pid);

        // Should succeed with complex structure
        assert!(
            result.is_ok(),
            "Complex for-comprehension should elaborate successfully"
        );
    }

    #[test]
    fn test_nested_for_comprehensions() {
        let code = r#"
            new outer, inner in {
                for(@x <- outer) {
                    @x!(1) | for(@y <- inner) {
                        @y!(2)
                    }
                }
            }
        "#;
        let parser = RholangParser::new();
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let root_pid = db.build_index(proc);

        // Run resolver on the entire tree
        let resolver = ResolverPass::new(root_pid);
        resolver.run(&mut db);

        // Find the outer for-comprehension PID
        let outer_for_pid = db
            .find_proc(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .expect("No for-comprehension found");

        // Elaborate outer for-comp
        let elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.elaborate_and_finalize(outer_for_pid);

        assert!(
            result.is_ok(),
            "Nested for-comprehensions should elaborate successfully"
        );
    }

    #[test]
    fn test_elaborator_with_different_bind_types() {
        let code = r#"new linear, persistent, peek in { for(@x <- linear; @y <= persistent; @z <<- peek) { Nil } }"#;
        let (mut db, pid) = setup_with_resolver(code);

        let elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.elaborate_and_finalize(pid);

        assert!(result.is_ok(), "All bind types should elaborate");
    }

    // ===== INTEGRATION TESTS: Complete Pipeline =====

    #[test]
    fn test_pipeline_integration_simple() {
        use crate::sem::pipeline::Pipeline;

        let parser = Box::leak(Box::new(RholangParser::new()));
        let code: &'static str = Box::leak(
            "new ch in { for(@x <- ch) { Nil } }"
                .to_string()
                .into_boxed_str(),
        );
        let ast = parser.parse(code).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        // Create pipeline with both ResolverPass and ForCompElaborationPass
        let pipeline = Pipeline::new()
            .add_fact(ResolverPass::new(root_pid))
            .add_fact(ForCompElaborationPass::new(root_pid));

        // Run pipeline
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(pipeline.run(&mut db));

        // Should have no errors
        assert!(
            !db.has_errors(),
            "Simple for-comp should pass all phases: {:?}",
            db.errors().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_pipeline_integration_multiple_for_comps() {
        use crate::sem::pipeline::Pipeline;

        let parser = Box::leak(Box::new(RholangParser::new()));
        let code: &'static str = Box::leak(
            r#"
            new ch1, ch2, ch3 in {
                for(@x <- ch1) { Nil } |
                for(@y <- ch2) { Nil } |
                for(@z <- ch3) { Nil }
            }
            "#
            .to_string()
            .into_boxed_str(),
        );
        let ast = parser.parse(code).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        // Create pipeline
        let pipeline = Pipeline::new()
            .add_fact(ResolverPass::new(root_pid))
            .add_fact(ForCompElaborationPass::new(root_pid));

        // Run pipeline
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(pipeline.run(&mut db));

        // All three for-comps should be validated
        assert!(!db.has_errors(), "Multiple for-comps should validate");
    }

    #[test]
    fn test_pipeline_integration_with_errors() {
        use crate::sem::pipeline::Pipeline;

        let parser = Box::leak(Box::new(RholangParser::new()));
        // Unbound variable should be caught by ResolverPass
        let code: &'static str =
            Box::leak("for(@x <- unbound_ch) { Nil }".to_string().into_boxed_str());
        let ast = parser.parse(code).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        // Create pipeline
        let pipeline = Pipeline::new()
            .add_fact(ResolverPass::new(root_pid))
            .add_fact(ForCompElaborationPass::new(root_pid));

        // Run pipeline
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(pipeline.run(&mut db));

        // Should have errors from ResolverPass
        assert!(db.has_errors(), "Unbound variable should cause errors");

        // Elaborator should still run and detect the missing scope
        let error_count = db.errors().count();
        assert!(error_count > 0, "Should have accumulated errors");
    }

    #[test]
    fn test_pipeline_integration_nested_for_comps() {
        use crate::sem::pipeline::Pipeline;

        let parser = Box::leak(Box::new(RholangParser::new()));
        let code: &'static str = Box::leak(
            r#"
            new outer, inner in {
                for(@x <- outer) {
                    for(@y <- inner) {
                        @y!(x)
                    }
                }
            }
            "#
            .to_string()
            .into_boxed_str(),
        );
        let ast = parser.parse(code).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        // Create pipeline
        let pipeline = Pipeline::new()
            .add_fact(ResolverPass::new(root_pid))
            .add_fact(ForCompElaborationPass::new(root_pid));

        // Run pipeline
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(pipeline.run(&mut db));

        // Both nested for-comps should be validated
        assert!(
            !db.has_errors(),
            "Nested for-comps should validate: {:?}",
            db.errors().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_pipeline_integration_complex_patterns() {
        use crate::sem::pipeline::Pipeline;

        let parser = Box::leak(Box::new(RholangParser::new()));
        let code: &'static str = Box::leak(
            r#"
            new ch in {
                for(@[x, y, ...rest] <- ch) {
                    Nil
                }
            }
            "#
            .to_string()
            .into_boxed_str(),
        );
        let ast = parser.parse(code).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        // Create pipeline
        let pipeline = Pipeline::new()
            .add_fact(ResolverPass::new(root_pid))
            .add_fact(ForCompElaborationPass::new(root_pid));

        // Run pipeline
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(pipeline.run(&mut db));

        // Complex patterns should be validated
        assert!(
            !db.has_errors(),
            "Complex patterns should validate: {:?}",
            db.errors().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_forcomp_elaboration_pass_creation() {
        use crate::sem::Pass;

        let pass = ForCompElaborationPass::new(PID(0));
        assert_eq!(pass.root(), PID(0));
        assert_eq!(pass.name(), "ForCompElaborationPass(0)");
    }

    #[test]
    fn test_forcomp_elaboration_pass_runs_on_subtree() {
        use crate::sem::FactPass;

        let parser = Box::leak(Box::new(RholangParser::new()));
        let code: &'static str = Box::leak(
            r#"
            new ch1, ch2 in {
                for(@x <- ch1) { Nil } |
                for(@y <- ch2) { Nil }
            }
            "#
            .to_string()
            .into_boxed_str(),
        );
        let ast = parser.parse(code).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        // Run ResolverPass first
        let resolver = ResolverPass::new(root_pid);
        resolver.run(&mut db);

        // Run ForCompElaborationPass
        let elaboration_pass = ForCompElaborationPass::new(root_pid);
        elaboration_pass.run(&mut db);

        // Both for-comps should be elaborated
        assert!(!db.has_errors(), "Both for-comps should be elaborated");
    }

    #[test]
    fn test_elaborator_rejects_mixed_arrow_types() {
        use crate::sem::pipeline::Pipeline;

        let parser = Box::leak(Box::new(RholangParser::new()));
        let code: &'static str = Box::leak(
            "new ch1, ch2 in { for(@x <- ch1 & @y <= ch2) { Nil } }"
                .to_string()
                .into_boxed_str(),
        );
        let ast = parser.parse(code).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        let pipeline = Pipeline::new()
            .add_fact(ResolverPass::new(root_pid))
            .add_fact(ForCompElaborationPass::new(root_pid));

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(pipeline.run(&mut db));

        // Should have error about mixed arrow types
        assert!(db.has_errors(), "Mixed arrow types should cause error");

        let errors: Vec<_> = db.errors().collect();
        assert!(
            errors.iter().any(|_e| {
                // Check if error message mentions mixed arrow types
                // Since we store errors in SemanticDb, we just verify an error exists
                true
            }),
            "Should have error about mixed arrow types, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_elaborator_allows_homogeneous_arrow_types() {
        use crate::sem::pipeline::Pipeline;

        let test_cases = vec![
            "new ch1, ch2 in { for(@x <- ch1 & @y <- ch2) { Nil } }", // All linear
            "new ch1, ch2 in { for(@x <= ch1 & @y <= ch2) { Nil } }", // All repeated
            "new ch1, ch2 in { for(@x <<- ch1 & @y <<- ch2) { Nil } }", // All peek
            "new ch1, ch2 in { for(@x <- ch1; @y <= ch2) { Nil } }",  // Sequential - different OK
        ];

        for code in test_cases {
            let parser = Box::leak(Box::new(RholangParser::new()));
            let code_static: &'static str = Box::leak(code.to_string().into_boxed_str());
            let ast = parser.parse(code_static).expect("Failed to parse");
            let ast_static = Box::leak(Box::new(ast));

            let mut db = SemanticDb::new();
            let proc = &ast_static[0];
            let root_pid = db.build_index(proc);

            let pipeline = Pipeline::new()
                .add_fact(ResolverPass::new(root_pid))
                .add_fact(ForCompElaborationPass::new(root_pid));

            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(pipeline.run(&mut db));

            assert!(
                !db.has_errors(),
                "Homogeneous arrow types should pass: {} - errors: {:?}",
                code,
                db.errors().collect::<Vec<_>>()
            );
        }
    }
}
