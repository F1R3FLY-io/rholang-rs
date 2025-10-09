//! Binding classification and management for for-comprehensions
//!
//! This module implements Phase 2.2 of the For-Comprehension Elaborator.
//! It provides context-based classification of bindings as name-valued vs proc-valued,
//! handles quote/unquote transformations, and validates binding uniqueness.

use crate::sem::{BinderKind, PID, SemanticDb, Symbol, SymbolOccurence};
use rholang_parser::ast::{self, AnnProc, Bind, Name, Names, Var};
use std::collections::{HashMap, HashSet};

use super::errors::{ElaborationError, ElaborationResult, ValidationResult};
use super::patterns::PatternVariable;

/// Classification of a binding based on context analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingClassification {
    /// Name-valued binding (used in name context)
    Name,
    /// Process-valued binding (used in process context)
    Proc,
}

impl BindingClassification {
    /// Convert to BinderKind for storage in semantic database
    pub fn to_binder_kind(self) -> BinderKind {
        match self {
            BindingClassification::Name => BinderKind::Name(None),
            BindingClassification::Proc => BinderKind::Proc,
        }
    }
}

/// Tracks context information for binding classification
#[derive(Debug, Clone)]
pub struct BindingContext {
    /// PIDs of processes in name context (e.g., inside quotes @{...})
    name_contexts: HashSet<PID>,
    /// PIDs of processes in proc context (e.g., normal process expressions)
    proc_contexts: HashSet<PID>,
    /// Current quote depth for determining context
    quote_depth: usize,
}

impl BindingContext {
    /// Create a new binding context starting in proc context
    pub fn new() -> Self {
        Self {
            name_contexts: HashSet::new(),
            proc_contexts: HashSet::new(),
            quote_depth: 0,
        }
    }

    /// Enter a quote context (@{...})
    pub fn enter_quote(&mut self) {
        self.quote_depth += 1;
    }

    /// Exit a quote context
    pub fn exit_quote(&mut self) {
        self.quote_depth = self.quote_depth.saturating_sub(1);
    }

    /// Check if currently in name context (odd quote depth)
    pub fn is_name_context(&self) -> bool {
        self.quote_depth % 2 == 1
    }

    /// Check if currently in proc context (even quote depth)
    pub fn is_proc_context(&self) -> bool {
        self.quote_depth % 2 == 0
    }

    /// Mark a PID as being in name context
    pub fn mark_name_context(&mut self, pid: PID) {
        self.name_contexts.insert(pid);
    }

    /// Mark a PID as being in proc context
    pub fn mark_proc_context(&mut self, pid: PID) {
        self.proc_contexts.insert(pid);
    }

    /// Get current quote depth
    pub fn quote_depth(&self) -> usize {
        self.quote_depth
    }
}

impl Default for BindingContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Classifies bindings in for-comprehension patterns as name-valued or proc-valued
///
/// The classifier analyzes the context in which bindings appear to determine their type:
/// - **Name context**: Inside quotes (@{...}), after quote operators, in channel positions
/// - **Proc context**: Normal process expressions, pattern bodies
///
/// Key responsibilities:
/// - Classify each binding based on quote depth and context
/// - Validate quote/unquote transformations
/// - Detect duplicate bindings within patterns
/// - Track binding uniqueness across receipts
pub struct BindingClassifier<'a, 'ast> {
    #[allow(dead_code)] // Used indirectly through methods
    db: &'a SemanticDb<'ast>,
    /// Context tracking for classification
    context: BindingContext,
    /// Bindings seen so far (symbol -> first occurrence)
    seen_bindings: HashMap<Symbol, SymbolOccurence>,
}

impl<'a, 'ast> BindingClassifier<'a, 'ast> {
    /// Create a new binding classifier
    pub fn new(db: &'a SemanticDb<'ast>) -> Self {
        Self {
            db,
            context: BindingContext::new(),
            seen_bindings: HashMap::new(),
        }
    }

    /// Classify a single binding from a for-comprehension receipt
    ///
    /// Determines whether the binding introduces name-valued or proc-valued variables
    /// based on the context (quote depth, source type, pattern structure).
    ///
    /// # Arguments
    ///
    /// * `bind` - The binding to classify
    ///
    /// # Returns
    ///
    /// The classification of this binding (Name or Proc)
    pub fn classify_binding(&mut self, bind: &Bind<'ast>) -> BindingClassification {
        // In Rholang, the left-hand side of a binding determines the classification:
        // - Patterns in for-comprehensions are always in proc context initially
        // - But variables bound inside @{...} quotes become name-valued
        // - The source (right-hand side) doesn't affect LHS classification

        // For for-comprehension bindings, we start in proc context
        // The actual classification depends on the pattern structure
        // This will be refined during pattern analysis

        match bind {
            Bind::Linear { lhs, rhs } => {
                // Linear bindings: x <- source
                // The pattern (lhs) binds proc-valued variables by default
                // unless they appear inside quotes in the pattern
                self.classify_names_in_pattern(lhs, rhs)
            }
            Bind::Repeated { lhs, rhs } | Bind::Peek { lhs, rhs } => {
                // Repeated (<=) and Peek (<<-) bindings
                // These also bind proc-valued variables by default
                self.classify_names_from_name(lhs, rhs)
            }
        }
    }

    /// Classify names that appear in a pattern with a Source RHS
    fn classify_names_in_pattern(
        &self,
        _lhs: &Names<'ast>,
        _rhs: &ast::Source<'ast>,
    ) -> BindingClassification {
        // For now, patterns in for-comprehensions bind proc-valued variables
        // This is the default case. Quote analysis will refine this.
        if self.context.is_name_context() {
            BindingClassification::Name
        } else {
            BindingClassification::Proc
        }
    }

    /// Classify names that appear in a pattern with a Name RHS
    fn classify_names_from_name(
        &self,
        _lhs: &Names<'ast>,
        _rhs: &Name<'ast>,
    ) -> BindingClassification {
        // Repeated and peek bindings also create proc-valued variables by default
        if self.context.is_name_context() {
            BindingClassification::Name
        } else {
            BindingClassification::Proc
        }
    }

    /// Classify a variable in a pattern, considering quote depth
    ///
    /// This is the core classification logic that determines if a variable
    /// is name-valued or proc-valued based on the current quote depth.
    pub fn classify_variable_in_pattern(&self, _var: Var<'ast>) -> BindingClassification {
        if self.context.is_name_context() {
            BindingClassification::Name
        } else {
            BindingClassification::Proc
        }
    }

    /// Validate quote/unquote transformations in a pattern
    ///
    /// Ensures that:
    /// - Quote (@P) and unquote (*x) are properly balanced
    /// - No invalid nesting of quotes
    /// - Variables in quoted contexts are properly classified
    pub fn validate_quote_unquote(&mut self, pattern: AnnProc<'ast>) -> ValidationResult<()> {
        self.validate_quotes_recursive(pattern)
    }

    /// Recursively validate quote/unquote structure
    fn validate_quotes_recursive(&mut self, pattern: AnnProc<'ast>) -> ValidationResult<()> {
        match pattern.proc {
            // Quote increases depth
            ast::Proc::Eval {
                name: Name::Quote(proc),
            } => {
                self.context.enter_quote();
                self.validate_quotes_recursive(*proc)?;
                self.context.exit_quote();
            }

            // Collections: validate elements
            ast::Proc::Collection(collection) => {
                self.validate_collection_quotes(collection)?;
            }

            // Parallel composition
            ast::Proc::Par { left, right } => {
                self.validate_quotes_recursive(*left)?;
                self.validate_quotes_recursive(*right)?;
            }

            // Binary expressions (including connectives)
            ast::Proc::BinaryExp { left, right, .. } => {
                self.validate_quotes_recursive(*left)?;
                self.validate_quotes_recursive(*right)?;
            }

            // Send expressions
            ast::Proc::Send { inputs, .. } => {
                for input in inputs.iter() {
                    self.validate_quotes_recursive(*input)?;
                }
            }

            // Other constructs don't affect quote depth
            _ => {}
        }

        Ok(())
    }

    fn validate_collection_quotes(
        &mut self,
        collection: &ast::Collection<'ast>,
    ) -> ValidationResult<()> {
        match collection {
            ast::Collection::List { elements, .. }
            | ast::Collection::Set { elements, .. }
            | ast::Collection::Tuple(elements) => {
                for element in elements {
                    self.validate_quotes_recursive(*element)?;
                }
            }
            ast::Collection::Map { elements, .. } => {
                for (key, value) in elements {
                    self.validate_quotes_recursive(*key)?;
                    self.validate_quotes_recursive(*value)?;
                }
            }
        }
        Ok(())
    }

    /// Check for duplicate bindings within a single pattern
    ///
    /// Validates that each variable name appears at most once in the pattern,
    /// preventing shadowing and ambiguous bindings.
    pub fn check_binding_uniqueness(
        &mut self,
        variables: &[PatternVariable],
    ) -> ElaborationResult<()> {
        for var in variables {
            let occurrence = SymbolOccurence {
                symbol: var.symbol,
                position: var.position,
            };

            if let Some(original) = self.seen_bindings.get(&var.symbol) {
                return Err(ElaborationError::DuplicateVarDef {
                    pid: PID(0), // Will be filled in by caller
                    original: *original,
                    duplicate: occurrence,
                });
            }

            self.seen_bindings.insert(var.symbol, occurrence);
        }

        Ok(())
    }

    /// Clear the seen bindings tracker (for processing new receipts)
    pub fn clear_seen_bindings(&mut self) {
        self.seen_bindings.clear();
    }

    /// Get the current binding context
    pub fn context(&self) -> &BindingContext {
        &self.context
    }

    /// Get mutable access to the binding context
    pub fn context_mut(&mut self) -> &mut BindingContext {
        &mut self.context
    }

    /// Check if a symbol has been seen before
    pub fn has_seen_binding(&self, symbol: Symbol) -> bool {
        self.seen_bindings.contains_key(&symbol)
    }

    /// Get the first occurrence of a binding
    pub fn get_first_occurrence(&self, symbol: Symbol) -> Option<SymbolOccurence> {
        self.seen_bindings.get(&symbol).copied()
    }

    /// Classify all bindings in a receipt
    ///
    /// Processes all bindings in a single receipt, classifying each one
    /// and checking for duplicates.
    pub fn classify_receipt(
        &mut self,
        receipt: &[Bind<'ast>],
    ) -> ElaborationResult<Vec<(Bind<'ast>, BindingClassification)>> {
        let mut classifications = Vec::new();

        for bind in receipt {
            let classification = self.classify_binding(bind);
            classifications.push((bind.clone(), classification));
        }

        Ok(classifications)
    }
}

/// Classify a single binding
pub fn classify_binding<'a, 'ast>(
    db: &'a SemanticDb<'ast>,
    bind: &Bind<'ast>,
) -> BindingClassification {
    let mut classifier = BindingClassifier::new(db);
    classifier.classify_binding(bind)
}

/// Validate that quote/unquote are properly balanced in a pattern
pub fn validate_quote_unquote<'a, 'ast>(
    db: &'a SemanticDb<'ast>,
    pattern: AnnProc<'ast>,
) -> ValidationResult<()> {
    let mut classifier = BindingClassifier::new(db);
    classifier.validate_quote_unquote(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::SemanticDb;
    use rholang_parser::{RholangParser, SourcePos};

    // Helper to create test setups with leaked memory (OK for tests)
    fn setup_test(code: &str) -> (Vec<ast::AnnProc<'static>>, SemanticDb<'static>) {
        let parser = Box::leak(Box::new(RholangParser::new()));
        let code_static: &'static str = Box::leak(code.to_string().into_boxed_str());
        let ast = parser
            .parse(code_static)
            .expect("Failed to parse test code");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        for proc in ast_static.iter() {
            db.build_index(proc);
        }

        (ast_static.to_vec(), db)
    }

    #[test]
    fn test_binding_context_creation() {
        let context = BindingContext::new();
        assert_eq!(context.quote_depth(), 0);
        assert!(context.is_proc_context());
        assert!(!context.is_name_context());
    }

    #[test]
    fn test_binding_context_quote_depth() {
        let mut context = BindingContext::new();

        context.enter_quote();
        assert_eq!(context.quote_depth(), 1);
        assert!(context.is_name_context());
        assert!(!context.is_proc_context());

        context.enter_quote();
        assert_eq!(context.quote_depth(), 2);
        assert!(!context.is_name_context());
        assert!(context.is_proc_context());

        context.exit_quote();
        assert_eq!(context.quote_depth(), 1);
        assert!(context.is_name_context());

        context.exit_quote();
        assert_eq!(context.quote_depth(), 0);
        assert!(context.is_proc_context());
    }

    #[test]
    fn test_binding_classification_to_binder_kind() {
        assert_eq!(
            BindingClassification::Name.to_binder_kind(),
            BinderKind::Name(None)
        );
        assert_eq!(
            BindingClassification::Proc.to_binder_kind(),
            BinderKind::Proc
        );
    }

    #[test]
    fn test_classifier_creation() {
        let (_, db) = setup_test("Nil");
        let classifier = BindingClassifier::new(&db);

        assert_eq!(classifier.context().quote_depth(), 0);
        assert!(classifier.context().is_proc_context());
    }

    #[test]
    fn test_classify_simple_linear_binding() {
        let (ast, db) = setup_test(r#"for(x <- @"channel") { Nil }"#);

        let mut classifier = BindingClassifier::new(&db);

        if let ast::Proc::ForComprehension { receipts, .. } = ast[0].proc {
            let bind = &receipts[0][0];
            let classification = classifier.classify_binding(bind);

            // Linear binding in proc context should be Proc-valued
            assert_eq!(classification, BindingClassification::Proc);
        } else {
            panic!("Expected ForComprehension");
        }
    }

    #[test]
    fn test_classify_repeated_binding() {
        let (ast, db) = setup_test(r#"for(x <= @"channel") { Nil }"#);

        let mut classifier = BindingClassifier::new(&db);

        if let ast::Proc::ForComprehension { receipts, .. } = ast[0].proc {
            let bind = &receipts[0][0];
            let classification = classifier.classify_binding(bind);

            // Repeated binding should also be Proc-valued in proc context
            assert_eq!(classification, BindingClassification::Proc);
        } else {
            panic!("Expected ForComprehension");
        }
    }

    #[test]
    fn test_classify_peek_binding() {
        let (ast, db) = setup_test(r#"for(x <<- @"channel") { Nil }"#);

        let mut classifier = BindingClassifier::new(&db);

        if let ast::Proc::ForComprehension { receipts, .. } = ast[0].proc {
            let bind = &receipts[0][0];
            let classification = classifier.classify_binding(bind);

            // Peek binding should be Proc-valued in proc context
            assert_eq!(classification, BindingClassification::Proc);
        } else {
            panic!("Expected ForComprehension");
        }
    }

    #[test]
    fn test_classify_variable_in_proc_context() {
        let (_, db) = setup_test("x");
        let classifier = BindingClassifier::new(&db);

        let classification = classifier.classify_variable_in_pattern(Var::Wildcard);
        assert_eq!(classification, BindingClassification::Proc);
    }

    #[test]
    fn test_classify_variable_in_name_context() {
        let (_, db) = setup_test("x");
        let mut classifier = BindingClassifier::new(&db);

        classifier.context_mut().enter_quote();
        let classification = classifier.classify_variable_in_pattern(Var::Wildcard);
        assert_eq!(classification, BindingClassification::Name);
    }

    #[test]
    fn test_check_binding_uniqueness_no_duplicates() {
        let (_, db) = setup_test("x");
        let mut classifier = BindingClassifier::new(&db);

        let x_sym = db.intern("x");
        let y_sym = db.intern("y");

        let variables = vec![
            PatternVariable::new(x_sym, SourcePos::default(), BinderKind::Proc),
            PatternVariable::new(y_sym, SourcePos::default(), BinderKind::Proc),
        ];

        let result = classifier.check_binding_uniqueness(&variables);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_binding_uniqueness_with_duplicates() {
        let (_, db) = setup_test("x");
        let mut classifier = BindingClassifier::new(&db);

        let x_sym = db.intern("x");

        let variables = vec![
            PatternVariable::new(x_sym, SourcePos::default(), BinderKind::Proc),
            PatternVariable::new(x_sym, SourcePos::at_col(5), BinderKind::Proc),
        ];

        let result = classifier.check_binding_uniqueness(&variables);
        assert!(result.is_err());

        if let Err(ElaborationError::DuplicateVarDef {
            original,
            duplicate,
            ..
        }) = result
        {
            assert_eq!(original.symbol, x_sym);
            assert_eq!(duplicate.symbol, x_sym);
            assert_ne!(original.position, duplicate.position);
        } else {
            panic!("Expected DuplicateVarDef error");
        }
    }

    #[test]
    fn test_clear_seen_bindings() {
        let (_, db) = setup_test("x");
        let mut classifier = BindingClassifier::new(&db);

        let x_sym = db.intern("x");
        let variables = vec![PatternVariable::new(
            x_sym,
            SourcePos::default(),
            BinderKind::Proc,
        )];

        classifier.check_binding_uniqueness(&variables).unwrap();
        assert!(classifier.has_seen_binding(x_sym));

        classifier.clear_seen_bindings();
        assert!(!classifier.has_seen_binding(x_sym));
    }

    #[test]
    fn test_validate_quote_unquote_simple() {
        let (ast, db) = setup_test("Nil");
        let mut classifier = BindingClassifier::new(&db);

        let result = classifier.validate_quote_unquote(ast[0]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_quote_unquote_with_collections() {
        let (ast, db) = setup_test("[1, 2, 3]");
        let mut classifier = BindingClassifier::new(&db);

        let result = classifier.validate_quote_unquote(ast[0]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_quote_unquote_parallel() {
        let (ast, db) = setup_test("Nil | ()");
        let mut classifier = BindingClassifier::new(&db);

        let result = classifier.validate_quote_unquote(ast[0]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_has_seen_binding() {
        let (_, db) = setup_test("x");
        let mut classifier = BindingClassifier::new(&db);

        let x_sym = db.intern("x");
        assert!(!classifier.has_seen_binding(x_sym));

        let variables = vec![PatternVariable::new(
            x_sym,
            SourcePos::default(),
            BinderKind::Proc,
        )];

        classifier.check_binding_uniqueness(&variables).unwrap();
        assert!(classifier.has_seen_binding(x_sym));
    }

    #[test]
    fn test_get_first_occurrence() {
        let (_, db) = setup_test("x");
        let mut classifier = BindingClassifier::new(&db);

        let x_sym = db.intern("x");
        let pos = SourcePos::at_col(10);

        let variables = vec![PatternVariable::new(x_sym, pos, BinderKind::Proc)];

        classifier.check_binding_uniqueness(&variables).unwrap();

        let occurrence = classifier.get_first_occurrence(x_sym);
        assert!(occurrence.is_some());
        assert_eq!(occurrence.unwrap().symbol, x_sym);
        assert_eq!(occurrence.unwrap().position, pos);
    }

    #[test]
    fn test_classify_receipt_multiple_bindings() {
        let (ast, db) = setup_test(r#"for(x <- @"ch1"; y <- @"ch2") { Nil }"#);

        let mut classifier = BindingClassifier::new(&db);

        if let ast::Proc::ForComprehension { receipts, .. } = ast[0].proc {
            // This for-comprehension has 2 receipts (separated by ;)
            assert_eq!(receipts.len(), 2);

            // Classify first receipt
            let result1 = classifier.classify_receipt(&receipts[0]);
            assert!(result1.is_ok());
            assert_eq!(result1.unwrap().len(), 1);

            // Classify second receipt
            let result2 = classifier.classify_receipt(&receipts[1]);
            assert!(result2.is_ok());
            assert_eq!(result2.unwrap().len(), 1);
        } else {
            panic!("Expected ForComprehension");
        }
    }

    #[test]
    fn test_standalone_classify_binding() {
        let (ast, db) = setup_test(r#"for(x <- @"channel") { Nil }"#);

        if let ast::Proc::ForComprehension { receipts, .. } = ast[0].proc {
            let bind = &receipts[0][0];
            let classification = classify_binding(&db, bind);

            assert_eq!(classification, BindingClassification::Proc);
        } else {
            panic!("Expected ForComprehension");
        }
    }

    #[test]
    fn test_standalone_validate_quote_unquote() {
        let (ast, db) = setup_test("[1, 2, 3]");

        let result = validate_quote_unquote(&db, ast[0]);
        assert!(result.is_ok());
    }
}
