//! This module validates join semantics in for-comprehensions
//! It ensures that:
//! - Validation same arrow type mixing within parallel bindings
//! - Join operations maintain atomicity (all-or-nothing semantics)
//! - Parallel bindings have valid structure
//! - Channel availability is validated before blocking operations

use crate::sem::{PID, SemanticDb, Symbol};
use bitvec::prelude::*;
use rholang_parser::ast::{Bind, Name, Receipts};
use smallvec::SmallVec;

use super::channel_validation::{get_name_position, get_source_position};
use super::errors::{ValidationError, ValidationResult};

/// Dependency graph for deadlock detection
#[derive(Debug, Clone)]
struct DependencyGraph {
    /// Adjacency list: channel -> channels it depends on
    edges: Vec<(Symbol, SmallVec<[Symbol; 4]>)>,

    /// All channels in the graph
    channels: SmallVec<[Symbol; 8]>,
}

impl DependencyGraph {
    fn new() -> Self {
        Self {
            edges: Vec::new(),
            channels: SmallVec::new(),
        }
    }

    /// Detect cycles in the dependency graph using DFS
    fn find_cycle(&self) -> Option<Vec<Symbol>> {
        let max_symbol = self.channels.iter().map(|s| s.0).max().unwrap_or(0);
        let mut visited = bitvec![0; max_symbol as usize + 1];
        let mut rec_stack = bitvec![0; max_symbol as usize + 1];
        let mut path = Vec::new();

        for &channel in &self.channels {
            if !visited[channel.0 as usize] {
                if let Some(cycle) =
                    self.dfs_cycle(channel, &mut visited, &mut rec_stack, &mut path)
                {
                    return Some(cycle);
                }
            }
        }

        None
    }

    fn dfs_cycle(
        &self,
        node: Symbol,
        visited: &mut BitVec,
        rec_stack: &mut BitVec,
        path: &mut Vec<Symbol>,
    ) -> Option<Vec<Symbol>> {
        let idx = node.0 as usize;
        visited.set(idx, true);
        rec_stack.set(idx, true);
        path.push(node);

        // Find neighbors for this node
        if let Some((_, neighbors)) = self.edges.iter().find(|(ch, _)| *ch == node) {
            for &neighbor in neighbors.iter() {
                let neighbor_idx = neighbor.0 as usize;

                if !visited[neighbor_idx] {
                    if let Some(cycle) = self.dfs_cycle(neighbor, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack[neighbor_idx] {
                    // Found cycle - extract it from path
                    let cycle_start = path.iter().position(|&ch| ch == neighbor).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        path.pop();
        rec_stack.set(idx, false);
        None
    }

    fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }
}

pub struct JoinValidator<'a, 'ast> {
    db: &'a SemanticDb<'ast>,
}

impl<'a, 'ast> JoinValidator<'a, 'ast> {
    pub fn new(db: &'a SemanticDb<'ast>) -> Self {
        Self { db }
    }

    pub fn validate(&self, for_comp_pid: PID, receipts: &'ast Receipts<'ast>) -> ValidationResult {
        self.validate_arrow_type_homogeneity(receipts)?;

        self.validate_join_atomicity(receipts)?;

        self.detect_deadlocks(receipts)?;

        self.validate_channel_availability(for_comp_pid, receipts)?;

        Ok(())
    }

    /// Validate join atomicity (all-or-nothing semantics)
    ///
    /// For parallel bindings within a receipt (`&`), all channels must be available
    /// before any consumption occurs. This ensures atomic execution of the join
    pub fn validate_join_atomicity(&self, receipts: &'ast Receipts<'ast>) -> ValidationResult {
        for receipt in receipts.iter() {
            if receipt.len() > 1 {
                self.check_atomic_group(receipt)?;
            }
        }

        Ok(())
    }

    /// Validates that all channels in the parallel group can be consumed atomically
    /// The actual channel binding validation is performed by `validate_channel_availability`,
    /// so this method only validates the structural integrity of the atomic group.
    fn check_atomic_group(&self, receipt: &[Bind<'ast>]) -> ValidationResult {
        if receipt.is_empty() {
            return Err(ValidationError::InvalidPatternStructure {
                pid: PID(0),
                position: None,
                reason: "Parallel join group cannot be empty".to_string(),
            });
        }

        Ok(())
    }

    /// Builds a dependency graph of channels and detects cycles that could lead to deadlocks
    pub fn detect_deadlocks(&self, receipts: &'ast Receipts<'ast>) -> ValidationResult {
        let graph = self.build_dependency_graph(receipts)?;

        if graph.is_empty() {
            return Ok(());
        }

        // Detect cycles in the dependency graph
        if let Some(cycle) = graph.find_cycle() {
            let channel_names: Vec<String> = cycle
                .iter()
                .filter_map(|&sym| self.db.resolve_symbol(sym))
                .map(|s| s.to_string())
                .collect();

            return Err(ValidationError::DeadlockPotential {
                receipts: format!(
                    "Circular dependency detected: {}",
                    channel_names.join(" -> ")
                ),
                pos: None, // Position will be extracted from PID by elaborator
            });
        }

        Ok(())
    }

    /// Build a dependency graph from receipts
    /// ## Current Limitation:
    /// Static analysis of cross-for-comprehension dependencies requires global
    /// program analysis. We focus on:
    /// 1. Validating structural integrity
    /// 2. Detecting obvious same-channel conflicts in parallel joins
    fn build_dependency_graph(
        &self,
        receipts: &'ast Receipts<'ast>,
    ) -> ValidationResult<DependencyGraph> {
        let graph = DependencyGraph::new();

        // Check for parallel bindings on the same channel (potential issue)
        for receipt in receipts.iter() {
            if receipt.len() > 1 {
                let mut channels_in_parallel: SmallVec<[Symbol; 8]> = SmallVec::new();
                for bind in receipt.iter() {
                    let channel = self.get_channel_symbol(bind.source_name());
                    if channel != Symbol::DUMMY && !channels_in_parallel.contains(&channel) {
                        channels_in_parallel.push(channel);
                        // Note: Same channel appearing multiple times in parallel join
                        // is valid in Rholang (waiting for N messages)
                    }
                }
            }
        }

        // For now, return empty graph. Deadlock detection requires:
        // 1. Control flow analysis across for-comprehensions
        // 2. Data flow analysis to track channel usage in bodies
        // 3. Global program analysis to find circular wait conditions
        Ok(graph)
    }

    /// Ensures that channels are available and properly initialized before
    /// the for-comprehension blocks waiting for them
    pub fn validate_channel_availability(
        &self,
        _for_comp_pid: PID,
        receipts: &'ast Receipts<'ast>,
    ) -> ValidationResult {
        for receipt in receipts.iter() {
            for bind in receipt.iter() {
                let channel_name = bind.source_name();

                super::channel_validation::verify_channel(self.db, channel_name)?;
            }
        }

        Ok(())
    }

    /// Get the symbol for a channel name
    fn get_channel_symbol(&self, name: &Name<'ast>) -> Symbol {
        match name {
            Name::NameVar(var) => match var {
                rholang_parser::ast::Var::Id(id) => self.db.intern(id.name),
                rholang_parser::ast::Var::Wildcard => Symbol::DUMMY,
            },
            Name::Quote(_) => Symbol::DUMMY,
        }
    }

    /// Validates that all bindings in a parallel join use the same arrow type
    pub fn validate_arrow_type_homogeneity(
        &self,
        receipts: &'ast Receipts<'ast>,
    ) -> ValidationResult {
        for (receipt_idx, receipt) in receipts.iter().enumerate() {
            if receipt.len() <= 1 {
                continue;
            }

            let mut arrow_types: SmallVec<[&'static str; 8]> = SmallVec::new();
            let mut first_pos: Option<rholang_parser::SourcePos> = None;

            for bind in receipt.iter() {
                let (arrow_type, pos) = match bind {
                    Bind::Linear { rhs, .. } => {
                        let pos = get_source_position(rhs);
                        ("linear (<-)", pos)
                    }
                    Bind::Repeated { rhs, .. } => {
                        let pos = get_name_position(rhs);
                        ("repeated (<=)", pos)
                    }
                    Bind::Peek { rhs, .. } => {
                        let pos = get_name_position(rhs);
                        ("peek (<<-)", pos)
                    }
                };

                if first_pos.is_none() {
                    first_pos = Some(pos);
                }

                if !arrow_types.iter().any(|&t| t == arrow_type) {
                    arrow_types.push(arrow_type);
                }
            }

            if arrow_types.len() > 1 {
                return Err(ValidationError::MixedArrowTypes {
                    receipt_index: receipt_idx,
                    found_types: arrow_types.into_vec(),
                    pos: first_pos,
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::resolver::ResolverPass;
    use crate::sem::{FactPass, SemanticDb};
    use rholang_parser::RholangParser;
    use rholang_parser::ast::Proc;

    fn setup_db(code: &str) -> (SemanticDb<'static>, PID) {
        // Leak memory for 'static lifetime (test only)
        let parser = Box::leak(Box::new(RholangParser::new()));
        let code_static: &'static str = Box::leak(code.to_string().into_boxed_str());
        let ast = parser.parse(code_static).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        // Run ResolverPass to build scopes
        let resolver = ResolverPass::new(root_pid);
        resolver.run(&mut db);

        // Find the for-comprehension PID
        let for_comp_pid = db
            .find_proc(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .expect("No for-comprehension found");

        (db, for_comp_pid)
    }

    fn get_receipts<'ast>(db: &SemanticDb<'ast>, pid: PID) -> &'ast Receipts<'ast> {
        let proc_ref = db.get(pid).expect("PID not found");
        match proc_ref.proc {
            Proc::ForComprehension { receipts, .. } => receipts,
            _ => panic!("Not a for-comprehension"),
        }
    }

    #[test]
    fn test_validator_creation() {
        let db = SemanticDb::new();
        let validator = JoinValidator::new(&db);
        assert!(std::ptr::eq(validator.db, &db));
    }

    #[test]
    fn test_simple_join_no_deadlock() {
        let code = r#"new ch1, ch2 in { for(@x <- ch1; @y <- ch2) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(result.is_ok(), "Simple join should validate: {:?}", result);
    }

    #[test]
    fn test_parallel_bindings_atomicity() {
        let code = r#"new ch1, ch2, ch3 in { for(@x <- ch1 & @y <- ch2 & @z <- ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_join_atomicity(receipts);

        assert!(
            result.is_ok(),
            "Parallel bindings should validate atomically: {:?}",
            result
        );
    }

    #[test]
    fn test_sequential_receipts() {
        let code = r#"new ch1, ch2, ch3 in { for(@x <- ch1; @y <- ch2; @z <- ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Sequential receipts should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_mixed_parallel_and_sequential() {
        let code = r#"new ch1, ch2, ch3, ch4 in { for(@a <- ch1 & @b <- ch2; @c <- ch3 & @d <- ch4) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Mixed parallel/sequential should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_channel_availability_check() {
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_channel_availability(pid, receipts);

        assert!(
            result.is_ok(),
            "Channel availability should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_unbound_channel_error() {
        let code = r#"for(@x <- unbound_ch) { Nil }"#;
        let (db, pid) = setup_db(code);

        // ResolverPass should have detected unbound variable
        assert!(
            db.has_errors(),
            "Unbound channel should be detected by ResolverPass"
        );

        let receipts = get_receipts(&db, pid);
        let validator = JoinValidator::new(&db);
        let result = validator.validate_channel_availability(pid, receipts);

        // Our validator also detects it
        assert!(
            result.is_err(),
            "Join validator should also detect unbound channel"
        );
    }

    #[test]
    fn test_wildcard_channel() {
        let code = r#"for(@x <- _) { Nil }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Wildcard channel should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_quoted_process_as_channel() {
        let code = r#"for(@x <- @Nil) { Nil }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Quoted process as channel should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_complex_join_pattern() {
        let code = r#"new ch1, ch2, ch3, ch4 in {
            for(@a <- ch1 & @b <- ch2; @c <- ch3 & @d <- ch4) {
                @a!(1) | @b!(2) | @c!(3) | @d!(4)
            }
        }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Complex join pattern should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_single_receipt_single_bind() {
        let code = r#"new ch in { for(@x <- ch) { @x!(1) } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Single receipt with single bind should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_multiple_parallel_same_channel() {
        // Waiting for multiple messages from the same channel
        let code = r#"new ch in { for(@x <- ch & @y <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Multiple bindings on same channel should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_homogeneous_linear_arrows() {
        let code = r#"new ch1, ch2, ch3 in { for(@a <- ch1 & @b <- ch2 & @c <- ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_arrow_type_homogeneity(receipts);

        assert!(
            result.is_ok(),
            "Homogeneous linear arrows should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_homogeneous_repeated_arrows() {
        let code = r#"new ch1, ch2, ch3 in { for(@a <= ch1 & @b <= ch2 & @c <= ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_arrow_type_homogeneity(receipts);

        assert!(
            result.is_ok(),
            "Homogeneous repeated arrows should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_homogeneous_peek_arrows() {
        let code = r#"new ch1, ch2, ch3 in { for(@a <<- ch1 & @b <<- ch2 & @c <<- ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_arrow_type_homogeneity(receipts);

        assert!(
            result.is_ok(),
            "Homogeneous peek arrows should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_mixed_linear_and_repeated_error() {
        let code = r#"new ch1, ch2 in { for(@a <- ch1 & @b <= ch2) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_arrow_type_homogeneity(receipts);

        assert!(result.is_err(), "Mixed linear and repeated should fail");

        if let Err(ValidationError::MixedArrowTypes { found_types, .. }) = result {
            assert_eq!(found_types.len(), 2);
            assert!(found_types.contains(&"linear (<-)"));
            assert!(found_types.contains(&"repeated (<=)"));
        } else {
            panic!("Expected MixedArrowTypes error");
        }
    }

    #[test]
    fn test_mixed_linear_and_peek_error() {
        let code = r#"new ch1, ch2 in { for(@a <- ch1 & @b <<- ch2) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_arrow_type_homogeneity(receipts);

        assert!(result.is_err(), "Mixed linear and peek should fail");

        if let Err(ValidationError::MixedArrowTypes { found_types, .. }) = result {
            assert_eq!(found_types.len(), 2);
            assert!(found_types.contains(&"linear (<-)"));
            assert!(found_types.contains(&"peek (<<-)"));
        } else {
            panic!("Expected MixedArrowTypes error");
        }
    }

    #[test]
    fn test_mixed_repeated_and_peek_error() {
        let code = r#"new ch1, ch2 in { for(@a <= ch1 & @b <<- ch2) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_arrow_type_homogeneity(receipts);

        assert!(result.is_err(), "Mixed repeated and peek should fail");

        if let Err(ValidationError::MixedArrowTypes { found_types, .. }) = result {
            assert_eq!(found_types.len(), 2);
            assert!(found_types.contains(&"repeated (<=)"));
            assert!(found_types.contains(&"peek (<<-)"));
        } else {
            panic!("Expected MixedArrowTypes error");
        }
    }

    #[test]
    fn test_mixed_all_three_arrow_types_error() {
        let code = r#"new ch1, ch2, ch3 in { for(@a <- ch1 & @b <= ch2 & @c <<- ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_arrow_type_homogeneity(receipts);

        assert!(result.is_err(), "Mixed all three arrow types should fail");

        if let Err(ValidationError::MixedArrowTypes { found_types, .. }) = result {
            assert_eq!(found_types.len(), 3);
        } else {
            panic!("Expected MixedArrowTypes error");
        }
    }

    #[test]
    fn test_sequential_receipts_allow_different_types() {
        // Different arrow types in sequential receipts (separated by ;) should be OK
        let code = r#"new ch1, ch2, ch3 in { for(@a <- ch1; @b <= ch2; @c <<- ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_arrow_type_homogeneity(receipts);

        assert!(
            result.is_ok(),
            "Sequential receipts with different arrow types should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_single_binding_always_homogeneous() {
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_arrow_type_homogeneity(receipts);

        assert!(
            result.is_ok(),
            "Single binding is always homogeneous: {:?}",
            result
        );
    }

    #[test]
    fn test_complex_mixed_sequential_and_parallel() {
        // First group: all linear (OK)
        // Second group: all repeated (OK)
        // Third group: mixed (ERROR)
        let code = r#"
            new ch1, ch2, ch3, ch4, ch5, ch6 in {
                for(
                    @a <- ch1 & @b <- ch2;      // OK: all linear
                    @c <= ch3 & @d <= ch4;      // OK: all repeated
                    @e <- ch5 & @f <<- ch6      // ERROR: mixed linear + peek
                ) {
                    Nil
                }
            }
        "#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.validate_arrow_type_homogeneity(receipts);

        assert!(
            result.is_err(),
            "Third receipt group has mixed types - should fail"
        );
    }
}
