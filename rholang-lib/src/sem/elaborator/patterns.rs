//! Pattern analysis and validation for for-comprehensions
//!
//! This module implements the pattern visitor and analyzer for Phase 2.1 of the
//! For-Comprehension Elaborator. It provides:
//!
//! - Pattern traversal with visitor pattern
//! - Variable extraction from patterns
//! - Connective validation (AND/OR only in patterns)
//! - Pattern satisfiability checking
//! - Remainder pattern support

use crate::sem::{BinderKind, SemanticDb, Symbol};
use rholang_parser::ast::{self, AnnProc, Var};
use rholang_parser::SourcePos;
use std::collections::HashSet;

use super::errors::{ElaborationResult, ValidationError, ValidationResult};

/// Information extracted from analyzing a pattern
#[derive(Debug, Clone)]
pub struct PatternInfo {
    /// Variables bound by this pattern
    pub variables: Vec<PatternVariable>,
    /// Whether the pattern contains wildcards
    pub has_wildcards: bool,
    /// Whether the pattern contains remainder patterns (...@rest)
    pub has_remainder: bool,
    /// Whether the pattern contains connectives (AND/OR)
    pub has_connectives: bool,
    /// Maximum nesting depth of the pattern
    pub depth: usize,
}

/// Information about a variable found in a pattern
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternVariable {
    /// The symbol (interned name)
    pub symbol: Symbol,
    /// Source position
    pub position: SourcePos,
    /// Kind of binder (name or proc)
    pub kind: BinderKind,
    /// Whether this is a remainder variable
    pub is_remainder: bool,
}

impl PatternVariable {
    pub fn new(symbol: Symbol, position: SourcePos, kind: BinderKind) -> Self {
        Self {
            symbol,
            position,
            kind,
            is_remainder: false,
        }
    }

    pub fn with_remainder(mut self, is_remainder: bool) -> Self {
        self.is_remainder = is_remainder;
        self
    }
}

/// Trait for visiting pattern nodes during traversal
///
/// This visitor follows the standard visitor pattern, allowing different
/// implementations to extract different information from patterns.
pub trait PatternVisitor<'ast> {
    /// Visit a variable in a pattern
    fn visit_var(&mut self, var: Var<'ast>, pos: SourcePos, kind: BinderKind);

    /// Visit a quoted process (@P) in a pattern
    fn visit_quote(&mut self, proc: AnnProc<'ast>);

    /// Visit a collection (list, set, map, tuple) in a pattern
    fn visit_collection(&mut self, collection: &ast::Collection<'ast>);

    /// Visit a connective (AND/OR) in a pattern
    fn visit_connective(&mut self, op: ast::BinaryExpOp, left: AnnProc<'ast>, right: AnnProc<'ast>);

    /// Visit a wildcard (_) in a pattern
    fn visit_wildcard(&mut self, pos: SourcePos);

    /// Visit a remainder pattern (...@rest)
    fn visit_remainder(&mut self, var: Option<Var<'ast>>, pos: SourcePos);
}

/// Analyzes patterns and extracts information about variables, structure, and validity
pub struct PatternAnalyzer<'a, 'ast> {
    db: &'a SemanticDb<'ast>,
    /// Track variables seen in this pattern to detect duplicates
    seen_variables: HashSet<Symbol>,
    /// Collected pattern information
    info: PatternInfo,
    /// Current depth during traversal
    current_depth: usize,
    /// Quote depth for tracking name vs proc context
    quote_depth: usize,
}

impl<'a, 'ast> PatternAnalyzer<'a, 'ast> {
    /// Create a new pattern analyzer
    pub fn new(db: &'a SemanticDb<'ast>) -> Self {
        Self {
            db,
            seen_variables: HashSet::new(),
            info: PatternInfo {
                variables: Vec::new(),
                has_wildcards: false,
                has_remainder: false,
                has_connectives: false,
                depth: 0,
            },
            current_depth: 0,
            quote_depth: 0,
        }
    }

    /// Analyze a pattern and extract information
    pub fn analyze_pattern(&mut self, pattern: AnnProc<'ast>) -> ElaborationResult<PatternInfo> {
        self.traverse_pattern(pattern)?;
        Ok(self.info.clone())
    }

    /// Traverse a pattern recursively
    fn traverse_pattern(&mut self, pattern: AnnProc<'ast>) -> ElaborationResult<()> {
        self.current_depth += 1;
        if self.current_depth > self.info.depth {
            self.info.depth = self.current_depth;
        }

        match pattern.proc {
            // Literals - valid in patterns
            ast::Proc::Nil
            | ast::Proc::Unit
            | ast::Proc::BoolLiteral(_)
            | ast::Proc::LongLiteral(_)
            | ast::Proc::StringLiteral(_)
            | ast::Proc::UriLiteral(_)
            | ast::Proc::SimpleType(_) => {
                // Literals don't introduce bindings
            }

            // Variables
            ast::Proc::ProcVar(var) => {
                self.visit_var(*var, pattern.span.start, self.current_binder_kind());
            }

            // Collections
            ast::Proc::Collection(collection) => {
                self.visit_collection(collection);
                self.traverse_collection(collection)?;
            }

            // Connectives (AND/OR) - only allowed in patterns
            ast::Proc::BinaryExp { op, left, right } => {
                if matches!(
                    op,
                    ast::BinaryExpOp::Conjunction | ast::BinaryExpOp::Disjunction
                ) {
                    self.visit_connective(*op, *left, *right);
                    self.traverse_pattern(*left)?;
                    self.traverse_pattern(*right)?;
                } else {
                    // Other binary operations in patterns
                    self.traverse_pattern(*left)?;
                    self.traverse_pattern(*right)?;
                }
            }

            // Parallel composition in patterns
            ast::Proc::Par { left, right } => {
                self.traverse_pattern(*left)?;
                self.traverse_pattern(*right)?;
            }

            // Send patterns (for matching sends)
            ast::Proc::Send { inputs, .. } => {
                for input in inputs.iter() {
                    self.traverse_pattern(*input)?;
                }
            }

            // Other constructs - may be valid in patterns depending on context
            _ => {
                // For now, allow but don't extract bindings
            }
        }

        self.current_depth -= 1;
        Ok(())
    }

    /// Traverse a collection and extract pattern variables
    fn traverse_collection(&mut self, collection: &ast::Collection<'ast>) -> ElaborationResult<()> {
        match collection {
            ast::Collection::List {
                elements,
                remainder,
            }
            | ast::Collection::Set {
                elements,
                remainder,
            } => {
                for element in elements {
                    self.traverse_pattern(*element)?;
                }
                if let Some(var) = remainder {
                    self.visit_remainder(Some(*var), var.get_position().unwrap_or_default());
                }
            }
            ast::Collection::Tuple(elements) => {
                for element in elements {
                    self.traverse_pattern(*element)?;
                }
            }
            ast::Collection::Map {
                elements,
                remainder,
            } => {
                for (key, value) in elements {
                    self.traverse_pattern(*key)?;
                    self.traverse_pattern(*value)?;
                }
                if let Some(var) = remainder {
                    self.visit_remainder(Some(*var), var.get_position().unwrap_or_default());
                }
            }
        }
        Ok(())
    }

    /// Determine current binder kind based on quote depth
    fn current_binder_kind(&self) -> BinderKind {
        if self.quote_depth % 2 == 0 {
            BinderKind::Proc
        } else {
            BinderKind::Name(None)
        }
    }

    /// Validate that connectives only appear in pattern context
    pub fn validate_connectives(&self, pattern: AnnProc<'ast>) -> ValidationResult<()> {
        // This is called during pattern analysis, so connectives are allowed
        self.check_connectives_recursive(pattern, true)
    }

    /// Recursively check connectives
    fn check_connectives_recursive(
        &self,
        proc: AnnProc<'ast>,
        in_pattern: bool,
    ) -> ValidationResult<()> {
        match proc.proc {
            ast::Proc::BinaryExp { op, left, right } => {
                if matches!(
                    op,
                    ast::BinaryExpOp::Conjunction | ast::BinaryExpOp::Disjunction
                ) && !in_pattern
                {
                    return Err(ValidationError::ConnectiveOutsidePattern {
                        pos: proc.span.start,
                    });
                }
                self.check_connectives_recursive(*left, in_pattern)?;
                self.check_connectives_recursive(*right, in_pattern)?;
            }
            ast::Proc::Par { left, right } => {
                self.check_connectives_recursive(*left, in_pattern)?;
                self.check_connectives_recursive(*right, in_pattern)?;
            }
            ast::Proc::Collection(collection) => {
                self.check_collection_connectives(collection, in_pattern)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn check_collection_connectives(
        &self,
        collection: &ast::Collection<'ast>,
        in_pattern: bool,
    ) -> ValidationResult<()> {
        match collection {
            ast::Collection::List { elements, .. } | ast::Collection::Set { elements, .. } => {
                for element in elements {
                    self.check_connectives_recursive(*element, in_pattern)?;
                }
            }
            ast::Collection::Tuple(elements) => {
                for element in elements {
                    self.check_connectives_recursive(*element, in_pattern)?;
                }
            }
            ast::Collection::Map { elements, .. } => {
                for (key, value) in elements {
                    self.check_connectives_recursive(*key, in_pattern)?;
                    self.check_connectives_recursive(*value, in_pattern)?;
                }
            }
        }
        Ok(())
    }

    /// Check if a pattern can be satisfied (basic structural validation)
    pub fn check_pattern_satisfiability(&self, pattern: AnnProc<'ast>) -> ValidationResult<()> {
        match pattern.proc {
            // These are always satisfiable
            ast::Proc::Nil
            | ast::Proc::Unit
            | ast::Proc::BoolLiteral(_)
            | ast::Proc::LongLiteral(_)
            | ast::Proc::StringLiteral(_)
            | ast::Proc::UriLiteral(_)
            | ast::Proc::SimpleType(_)
            | ast::Proc::ProcVar(_) => Ok(()),

            // Connectives need special handling
            ast::Proc::BinaryExp {
                op: ast::BinaryExpOp::Conjunction,
                left,
                right,
            } => {
                // AND pattern - both sides must be satisfiable and compatible
                self.check_pattern_satisfiability(*left)?;
                self.check_pattern_satisfiability(*right)?;
                // TODO: Check for contradictory constraints
                Ok(())
            }

            ast::Proc::BinaryExp {
                op: ast::BinaryExpOp::Disjunction,
                left,
                right,
            } => {
                // OR pattern - at least one side must be satisfiable
                let left_result = self.check_pattern_satisfiability(*left);
                let right_result = self.check_pattern_satisfiability(*right);

                if left_result.is_err() && right_result.is_err() {
                    return Err(ValidationError::UnsatisfiablePattern {
                        pattern: "All branches of OR pattern are unsatisfiable".to_string(),
                    });
                }
                Ok(())
            }

            // Collections are satisfiable if their elements are
            ast::Proc::Collection(collection) => {
                self.check_collection_satisfiability(collection)
            }

            // Parallel composition
            ast::Proc::Par { left, right } => {
                self.check_pattern_satisfiability(*left)?;
                self.check_pattern_satisfiability(*right)?;
                Ok(())
            }

            // Other constructs - assume satisfiable for now
            _ => Ok(()),
        }
    }

    fn check_collection_satisfiability(
        &self,
        collection: &ast::Collection<'ast>,
    ) -> ValidationResult<()> {
        match collection {
            ast::Collection::List { elements, .. }
            | ast::Collection::Set { elements, .. }
            | ast::Collection::Tuple(elements) => {
                for element in elements {
                    self.check_pattern_satisfiability(*element)?;
                }
                Ok(())
            }
            ast::Collection::Map { elements, .. } => {
                for (key, value) in elements {
                    self.check_pattern_satisfiability(*key)?;
                    self.check_pattern_satisfiability(*value)?;
                }
                Ok(())
            }
        }
    }
}

impl<'a, 'ast> PatternVisitor<'ast> for PatternAnalyzer<'a, 'ast> {
    fn visit_var(&mut self, var: Var<'ast>, pos: SourcePos, kind: BinderKind) {
        match var {
            Var::Wildcard => {
                self.info.has_wildcards = true;
            }
            Var::Id(id) => {
                let symbol = self.db.intern(id.name);

                // Track variable for duplicate detection
                self.seen_variables.insert(symbol);

                self.info
                    .variables
                    .push(PatternVariable::new(symbol, pos, kind));
            }
        }
    }

    fn visit_quote(&mut self, proc: AnnProc<'ast>) {
        self.quote_depth += 1;
        let _ = self.traverse_pattern(proc);
        self.quote_depth -= 1;
    }

    fn visit_collection(&mut self, _collection: &ast::Collection<'ast>) {
        // Collection handling is done in traverse_collection
    }

    fn visit_connective(
        &mut self,
        _op: ast::BinaryExpOp,
        _left: AnnProc<'ast>,
        _right: AnnProc<'ast>,
    ) {
        self.info.has_connectives = true;
    }

    fn visit_wildcard(&mut self, _pos: SourcePos) {
        self.info.has_wildcards = true;
    }

    fn visit_remainder(&mut self, var: Option<Var<'ast>>, pos: SourcePos) {
        self.info.has_remainder = true;
        if let Some(Var::Id(id)) = var {
            let symbol = self.db.intern(id.name);
            self.seen_variables.insert(symbol);
            self.info.variables.push(
                PatternVariable::new(symbol, pos, self.current_binder_kind()).with_remainder(true),
            );
        }
    }
}

/// Extract all variables from a pattern in for-comprehension bindings
pub fn extract_pattern_variables<'a, 'ast>(
    db: &'a SemanticDb<'ast>,
    pattern: AnnProc<'ast>,
) -> ElaborationResult<Vec<PatternVariable>> {
    let mut analyzer = PatternAnalyzer::new(db);
    let info = analyzer.analyze_pattern(pattern)?;
    Ok(info.variables)
}

/// Validate that a pattern only contains connectives in appropriate contexts
pub fn validate_pattern_connectives<'a, 'ast>(
    db: &'a SemanticDb<'ast>,
    pattern: AnnProc<'ast>,
) -> ValidationResult<()> {
    let analyzer = PatternAnalyzer::new(db);
    analyzer.validate_connectives(pattern)
}

/// Check if a pattern is satisfiable
pub fn check_pattern_satisfiability<'a, 'ast>(
    db: &'a SemanticDb<'ast>,
    pattern: AnnProc<'ast>,
) -> ValidationResult<()> {
    let analyzer = PatternAnalyzer::new(db);
    analyzer.check_pattern_satisfiability(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::SemanticDb;
    use rholang_parser::{RholangParser, ast::AnnProc};

    // Helper to create test setups with leaked memory (OK for tests)
    fn setup_test(code: &str) -> (AnnProc<'static>, SemanticDb<'static>) {
        let parser = Box::leak(Box::new(RholangParser::new()));
        let code_static: &'static str = Box::leak(code.to_string().into_boxed_str());
        let ast = parser.parse(code_static).expect("Failed to parse test code");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let _pid = db.build_index(proc);

        (*proc, db)
    }

    #[test]
    fn test_analyze_simple_variable_pattern() {
        let (proc, db) = setup_test("x");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        assert_eq!(info.variables.len(), 1);
        assert_eq!(&db[info.variables[0].symbol], "x");
        assert!(!info.has_wildcards);
        assert!(!info.has_remainder);
        assert!(!info.has_connectives);
    }

    #[test]
    fn test_analyze_wildcard_pattern() {
        let (proc, db) = setup_test("_");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        assert_eq!(info.variables.len(), 0);
        assert!(info.has_wildcards);
    }

    #[test]
    fn test_analyze_list_pattern_with_variables() {
        let (proc, db) = setup_test("[x, y, z]");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        assert_eq!(info.variables.len(), 3);
        assert!(!info.has_wildcards);
        assert!(!info.has_remainder);
    }

    #[test]
    fn test_analyze_list_pattern_with_remainder() {
        let (proc, db) = setup_test("[x, y ...rest]");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        assert_eq!(info.variables.len(), 3);
        assert!(info.has_remainder);

        // Check that 'rest' is marked as remainder
        let rest_var = info
            .variables
            .iter()
            .find(|v| &db[v.symbol] == "rest")
            .unwrap();
        assert!(rest_var.is_remainder);
    }

    #[test]
    fn test_analyze_nested_pattern() {
        let (proc, db) = setup_test("[[x, y], [z, w]]");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        assert_eq!(info.variables.len(), 4);
        assert!(info.depth > 1);
    }

    #[test]
    fn test_connective_detection() {
        let (proc, db) = setup_test("[x, y]");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        // Simple list shouldn't have connectives
        assert!(!info.has_connectives);
    }

    #[test]
    fn test_pattern_satisfiability_simple() {
        let (proc, db) = setup_test("42");

        let analyzer = PatternAnalyzer::new(&db);
        let result = analyzer.check_pattern_satisfiability(proc);

        assert!(result.is_ok());
    }

    #[test]
    fn test_pattern_satisfiability_collection() {
        let (proc, db) = setup_test("[1, 2, 3]");

        let analyzer = PatternAnalyzer::new(&db);
        let result = analyzer.check_pattern_satisfiability(proc);

        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_pattern_variables() {
        let (proc, db) = setup_test("[x, y, z]");

        let vars = extract_pattern_variables(&db, proc).unwrap();

        assert_eq!(vars.len(), 3);
        let var_names: Vec<_> = vars.iter().map(|v| db[v.symbol].to_string()).collect();
        assert!(var_names.contains(&"x".to_string()));
        assert!(var_names.contains(&"y".to_string()));
        assert!(var_names.contains(&"z".to_string()));
    }

    #[test]
    fn test_parallel_patterns() {
        let (proc, db) = setup_test("x | y");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        assert_eq!(info.variables.len(), 2);
    }

    #[test]
    fn test_mixed_pattern_with_wildcards() {
        let (proc, db) = setup_test("[x, _, z]");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        assert_eq!(info.variables.len(), 2);
        assert!(info.has_wildcards);
    }

    #[test]
    fn test_tuple_pattern() {
        let (proc, db) = setup_test("(x, y, z)");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        assert_eq!(info.variables.len(), 3);
    }

    #[test]
    fn test_map_pattern() {
        let (proc, db) = setup_test("{\"key\": value}");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        // Should extract 'value' variable
        assert_eq!(info.variables.len(), 1);
        assert_eq!(&db[info.variables[0].symbol], "value");
    }

    #[test]
    fn test_pattern_depth_tracking() {
        let (proc, db) = setup_test("[[[x]]]");

        let mut analyzer = PatternAnalyzer::new(&db);
        let info = analyzer.analyze_pattern(proc).unwrap();

        assert!(info.depth >= 3);
    }
}
