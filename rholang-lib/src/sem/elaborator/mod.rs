//! For-Comprehension Elaborator - Phase 4 Semantic Validation
//!
//! This module provides **advanced semantic validation** for Rholang for-comprehensions.
//! It implements **Phase 4 ONLY** - all scope/binding/pattern resolution (Phases 1-3)
//! is handled by the `ResolverPass` which must run before this elaborator.
//!
//! ## Architecture
//!
//! The elaborator validates semantics that go beyond basic scope resolution:
//! - **Phase 4.1**: Type consistency checking (channel types, pattern-message compatibility)
//! - **Phase 4.2**: Consumption semantics (linear/peek/repeated validation)
//! - **Phase 4.3**: Join semantics (atomicity, deadlock detection)
//! - **Phase 4.4**: Pattern query validation (SQL-like patterns, satisfiability)
//! - **Phase 4.5**: Object-capability validation (unforgeable names, privacy)
//!
//! ## Usage
//!
//! ```ignore
//! use rholang_lib::sem::{SemanticDb, resolver::ResolverPass, FactPass};
//! use rholang_lib::sem::elaborator::ForComprehensionElaborator;
//!
//! let mut db = SemanticDb::new();
//! let pid = db.build_index(&ast);
//!
//! // Step 1: Run ResolverPass to build scopes and resolve variables (Phases 1-3)
//! let resolver = ResolverPass::new(pid);
//! resolver.run(&mut db);
//!
//! // Step 2: Run elaborator for advanced semantic validation (Phase 4)
//! let elaborator = ForComprehensionElaborator::new(&mut db);
//! elaborator.elaborate_and_finalize(pid)?;
//! ```
//!
//! This module was refactored to remove redundant functionality that duplicated
//! the `ResolverPass`. The following phases are **NO LONGER** handled here:
//! - ❌ Phase 2.1: Pattern analysis (use `ResolverPass::PatternResolver`)
//! - ❌ Phase 2.2: Binding classification (use `ResolverPass` binder tracking)
//! - ❌ Phase 2.3: Source validation (use `ResolverPass` name resolution)
//! - ❌ Phase 3.1: Scope building (use `ResolverPass::LexicallyScoped`)
//! - ❌ Phase 3.2: Variable resolution (use `ResolverPass::resolve_var()`)
//!
//! See `docs/for-comp-elaborator-plan.md` for the complete refactored architecture.

use crate::sem::{PID, SemanticDb};

pub mod consumption;
pub mod errors;
pub mod for_comp;
pub mod joins;
pub mod validation;

pub use errors::{ElaborationError, ElaborationResult, ElaborationWarning};

/// Consumption mode for channel bindings in for-comprehensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsumptionMode {
    /// Standard linear consumption (`<-`)
    /// Channel is consumed exactly once
    Linear,
    /// Persistent/repeated consumption (`<=-`)
    /// Contract-like behavior, channel is re-instantiated
    Persistent,
    /// Non-consuming read/peek operation (`<<-`)
    /// Channel is read but not consumed
    Peek,
}

/// Type of a channel in Rholang
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelType {
    /// Unforgeable name (created with `new` declarations)
    UnforgeableName,
    /// Quoted process (`@P`)
    QuotedProcess,
    /// Variable reference
    Variable,
    /// Unknown or complex channel type
    Unknown,
}

/// Main elaborator for for-comprehension advanced semantic validation (Phase 4)
///
/// This elaborator performs semantic checks that go beyond basic scope resolution.
/// It **requires** that `ResolverPass` has already run to build scopes and resolve
/// variables (Phases 1-3).
///
/// The elaborator is stateless and can be reused for multiple for-comprehensions.
pub struct ForComprehensionElaborator<'a, 'ast> {
    db: &'a mut SemanticDb<'ast>,
    errors: Vec<ElaborationError>,
    warnings: Vec<ElaborationWarning>,
}

impl<'a, 'ast> ForComprehensionElaborator<'a, 'ast> {
    /// Create a new elaborator instance
    pub fn new(db: &'a mut SemanticDb<'ast>) -> Self {
        Self {
            db,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Get immutable reference to the semantic database
    pub fn db(&self) -> &SemanticDb<'ast> {
        self.db
    }

    /// Get mutable reference to the semantic database
    pub fn db_mut(&mut self) -> &mut SemanticDb<'ast> {
        self.db
    }

    /// Get all accumulated errors
    pub fn errors(&self) -> &[ElaborationError] {
        &self.errors
    }

    /// Get all accumulated warnings
    pub fn warnings(&self) -> &[ElaborationWarning] {
        &self.warnings
    }

    /// Check if any errors have been accumulated
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Add an error to the accumulator
    pub fn add_error(&mut self, error: ElaborationError) {
        self.errors.push(error);
    }

    /// Add a warning to the accumulator
    pub fn add_warning(&mut self, warning: ElaborationWarning) {
        self.warnings.push(warning);
    }

    /// Clear all accumulated diagnostics
    pub fn clear_diagnostics(&mut self) {
        self.errors.clear();
        self.warnings.clear();
    }

    /// Finalize elaboration and emit diagnostics to the database
    ///
    /// Converts all errors and warnings to `Diagnostic` instances and emits
    /// them to the semantic database. Returns an error if any errors were accumulated.
    pub fn finalize(self) -> Result<(), Vec<crate::sem::Diagnostic>> {
        use crate::sem::Diagnostic;

        // Convert all errors and warnings to diagnostics
        let diagnostics: Vec<Diagnostic> = self
            .errors
            .iter()
            .map(|e| e.to_diagnostic())
            .chain(self.warnings.iter().map(|w| w.to_diagnostic()))
            .collect();

        // Emit to database (this modifies db, but we're consuming self anyway)
        for diagnostic in &diagnostics {
            self.db.emit_diagnostic(*diagnostic);
        }

        // Return error if there were any errors
        if !self.errors.is_empty() {
            Err(diagnostics)
        } else {
            Ok(())
        }
    }

    /// Elaborate a for-comprehension with Phase 4 semantic validation
    ///
    /// This is the main entry point for elaboration. It performs **Phase 4 validation only**:
    ///
    /// - **Phase 4.1**: Type consistency checking
    /// - **Phase 4.2**: Consumption semantics validation
    /// - **Phase 4.3**: Join semantics validation
    /// - **Phase 4.4**: Pattern query validation
    /// - **Phase 4.5**: Object-capability validation
    ///
    /// ## Prerequisites
    ///
    /// **IMPORTANT**: `ResolverPass` MUST have run before this elaborator to:
    /// - Build scopes for the for-comprehension (Phase 3.1)
    /// - Resolve all variable references (Phase 3.2)
    /// - Validate basic pattern structure
    ///
    /// If ResolverPass has not run, this method will return an error immediately.
    ///
    /// ## Arguments
    ///
    /// * `pid` - The PID of the for-comprehension to elaborate
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` if elaboration succeeded, or `Err(diagnostics)` if errors occurred.
    pub fn elaborate_and_finalize(mut self, pid: PID) -> Result<(), Vec<crate::sem::Diagnostic>> {
        // Minimal pre-validation: verify it's a for-comprehension and ResolverPass ran
        if let Err(error) = self.verify_for_comprehension_and_scope(pid) {
            self.add_error(error);
            return self.finalize();
        }

        // TODO Phase 4.1: Type consistency checking
        // if let Err(error) = self.validate_type_consistency(pid) {
        //     self.add_error(error);
        //     // Continue to collect all errors
        // }

        // TODO Phase 4.2: Consumption semantics validation
        // if let Err(error) = self.validate_consumption_semantics(pid) {
        //     self.add_error(error);
        // }

        // TODO Phase 4.3: Join semantics validation
        // if let Err(error) = self.validate_join_semantics(pid) {
        //     self.add_error(error);
        // }

        // TODO Phase 4.4: Pattern query validation
        // if let Err(error) = self.validate_pattern_queries(pid) {
        //     self.add_error(error);
        // }

        // TODO Phase 4.5: Object-capability validation
        // if let Err(error) = self.validate_capabilities(pid) {
        //     self.add_error(error);
        // }

        self.finalize()
    }

    /// Verify that the PID references a for-comprehension and ResolverPass created a scope
    ///
    /// This is the minimal pre-validation needed before Phase 4 validation.
    /// It checks that:
    /// 1. The PID exists in the database
    /// 2. The PID references a `ForComprehension` node
    /// 3. ResolverPass created a scope for this for-comprehension
    fn verify_for_comprehension_and_scope(&self, pid: PID) -> ElaborationResult<()> {
        use rholang_parser::ast::Proc;

        // Check PID exists
        let proc = self
            .db
            .get(pid)
            .ok_or(ElaborationError::InvalidPid { pid })?;

        // Check it's a for-comprehension
        match proc.proc {
            Proc::ForComprehension { .. } => {
                // Check that ResolverPass created a scope
                self.db.get_scope(pid).ok_or_else(|| {
                    ElaborationError::IncompleteAstNode {
                        pid,
                        position: Some(proc.span.start),
                        reason: "ResolverPass did not create a scope for this for-comprehension. \
                                 Ensure ResolverPass runs before ForCompElaborationPass."
                            .to_string(),
                    }
                })?;

                Ok(())
            }
            _ => Err(ElaborationError::IncompleteAstNode {
                pid,
                position: Some(proc.span.start),
                reason: "Expected for-comprehension node".to_string(),
            }),
        }
    }
}

/// Configuration for the elaborator
///
/// **Note**: Currently unused, reserved for future Phase 4 implementation
#[derive(Debug, Clone)]
pub struct ElaboratorConfig {
    /// Whether to perform strict type checking
    pub strict_typing: bool,
    /// Whether to warn about unused pattern variables
    pub warn_unused_patterns: bool,
    /// Whether to suggest pattern optimizations
    pub suggest_optimizations: bool,
    /// Maximum depth for pattern analysis
    pub max_pattern_depth: usize,
}

impl Default for ElaboratorConfig {
    fn default() -> Self {
        Self {
            strict_typing: true,
            warn_unused_patterns: true,
            suggest_optimizations: false,
            max_pattern_depth: 32,
        }
    }
}

impl ElaboratorConfig {
    pub fn strict() -> Self {
        Self {
            strict_typing: true,
            warn_unused_patterns: true,
            suggest_optimizations: true,
            max_pattern_depth: 32,
        }
    }

    pub fn lenient() -> Self {
        Self {
            strict_typing: false,
            warn_unused_patterns: false,
            suggest_optimizations: false,
            max_pattern_depth: 64,
        }
    }

    pub fn with_strict_typing(mut self, strict: bool) -> Self {
        self.strict_typing = strict;
        self
    }

    pub fn with_unused_pattern_warnings(mut self, warn: bool) -> Self {
        self.warn_unused_patterns = warn;
        self
    }

    pub fn with_optimization_suggestions(mut self, suggest: bool) -> Self {
        self.suggest_optimizations = suggest;
        self
    }

    pub fn with_max_pattern_depth(mut self, depth: usize) -> Self {
        self.max_pattern_depth = depth;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::{resolver::ResolverPass, FactPass, SemanticDb};
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
            .find_proc(|p| matches!(p.proc, rholang_parser::ast::Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .expect("No for-comprehension found in test code");

        (db, for_comp_pid)
    }

    #[test]
    fn test_elaborator_creation() {
        let mut db = SemanticDb::new();
        let elaborator = ForComprehensionElaborator::new(&mut db);

        assert!(!elaborator.has_errors());
        assert!(elaborator.errors().is_empty());
        assert!(elaborator.warnings().is_empty());
    }

    #[test]
    fn test_verify_requires_scope_from_resolver() {
        let parser = RholangParser::new();
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let _root_pid = db.build_index(proc);

        // Find the for-comprehension PID
        let for_comp_pid = db
            .find_proc(|p| matches!(p.proc, rholang_parser::ast::Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .expect("No for-comprehension found");

        // WITHOUT running ResolverPass
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
        let code = "Nil"; // Not a for-comprehension
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
    fn test_elaborator_accumulates_errors() {
        let mut db = SemanticDb::new();
        let mut elaborator = ForComprehensionElaborator::new(&mut db);

        assert!(!elaborator.has_errors());

        elaborator.add_error(ElaborationError::InvalidPid { pid: PID(0) });
        assert!(elaborator.has_errors());
        assert_eq!(elaborator.errors().len(), 1);

        elaborator.clear_diagnostics();
        assert!(!elaborator.has_errors());
    }

    #[test]
    fn test_elaborator_config() {
        let config = ElaboratorConfig::default()
            .with_strict_typing(false)
            .with_unused_pattern_warnings(true)
            .with_max_pattern_depth(16);

        assert!(!config.strict_typing);
        assert!(config.warn_unused_patterns);
        assert_eq!(config.max_pattern_depth, 16);
    }

    #[test]
    fn test_consumption_mode() {
        assert_eq!(format!("{:?}", ConsumptionMode::Linear), "Linear");
        assert_eq!(format!("{:?}", ConsumptionMode::Persistent), "Persistent");
        assert_eq!(format!("{:?}", ConsumptionMode::Peek), "Peek");
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
                    x!(1) | for(@y <- inner) {
                        y!(2)
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
            .find_proc(|p| matches!(p.proc, rholang_parser::ast::Proc::ForComprehension { .. }))
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

        // All bind types should be accepted
        assert!(result.is_ok(), "All bind types should elaborate");
    }
}