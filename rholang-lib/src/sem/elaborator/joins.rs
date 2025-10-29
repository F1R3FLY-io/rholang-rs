//! This module validates join semantics in for-comprehensions
//! It ensures that:
//! - Join operations maintain atomicity (all-or-nothing semantics)
//! - Parallel bindings have valid structure
//! - Channel availability is validated before blocking operations

use crate::sem::{PID, SemanticDb, Symbol};
use rholang_parser::ast::{Bind, Name, Receipts};
use std::collections::{HashMap, HashSet};

use super::errors::{ValidationError, ValidationResult};

// Removed unused ChannelDependency struct - we only need the channel Symbol

/// Dependency graph for deadlock detection
#[derive(Debug, Clone)]
struct DependencyGraph {
    /// Adjacency list: channel -> channels it depends on
    edges: HashMap<Symbol, HashSet<Symbol>>,
    /// Track all channels in the graph
    channels: HashSet<Symbol>,
}

impl DependencyGraph {
    fn new() -> Self {
        Self {
            edges: HashMap::new(),
            channels: HashSet::new(),
        }
    }
    fn add_channel(&mut self, channel: Symbol) {
        self.channels.insert(channel);
        self.edges.entry(channel).or_default();
    }

    /// Add a dependency edge: `from` depends on `to`
    fn add_dependency(&mut self, from: Symbol, to: Symbol) {
        self.add_channel(from);
        self.add_channel(to);
        self.edges.entry(from).or_default().insert(to);
    }

    /// Detect cycles in the dependency graph using DFS
    fn find_cycle(&self) -> Option<Vec<Symbol>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for &channel in &self.channels {
            if !visited.contains(&channel)
                && let Some(cycle) =
                    self.dfs_cycle(channel, &mut visited, &mut rec_stack, &mut path)
            {
                return Some(cycle);
            }
        }

        None
    }

    fn dfs_cycle(
        &self,
        node: Symbol,
        visited: &mut HashSet<Symbol>,
        rec_stack: &mut HashSet<Symbol>,
        path: &mut Vec<Symbol>,
    ) -> Option<Vec<Symbol>> {
        visited.insert(node);
        rec_stack.insert(node);
        path.push(node);

        if let Some(neighbors) = self.edges.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    if let Some(cycle) = self.dfs_cycle(neighbor, visited, rec_stack, path) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&neighbor) {
                    let cycle_start = path.iter().position(|&ch| ch == neighbor).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        path.pop();
        rec_stack.remove(&node);
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
                let mut channels_in_parallel = HashSet::new();
                for bind in receipt.iter() {
                    let channel = self.get_channel_symbol(bind.source_name());
                    if channel != Symbol::DUMMY {
                        channels_in_parallel.insert(channel);
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
    fn test_no_circular_dependency_simple() {
        let code = r#"new ch1, ch2 in { for(@x <- ch1; @y <- ch2) { @x!(y) } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = JoinValidator::new(&db);
        let result = validator.detect_deadlocks(receipts);

        assert!(
            result.is_ok(),
            "Simple sequential pattern should not have circular dependencies: {:?}",
            result
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
    fn test_dependency_graph_cycle_detection() {
        let mut graph = DependencyGraph::new();

        let sym1 = Symbol(1);
        let sym2 = Symbol(2);
        let sym3 = Symbol(3);

        // Create a cycle: 1 -> 2 -> 3 -> 1
        graph.add_dependency(sym1, sym2);
        graph.add_dependency(sym2, sym3);
        graph.add_dependency(sym3, sym1);

        let cycle = graph.find_cycle();
        assert!(cycle.is_some(), "Cycle should be detected");

        let cycle_vec = cycle.unwrap();
        assert!(!cycle_vec.is_empty(), "Cycle should not be empty");
    }

    #[test]
    fn test_dependency_graph_no_cycle() {
        let mut graph = DependencyGraph::new();

        let sym1 = Symbol(1);
        let sym2 = Symbol(2);
        let sym3 = Symbol(3);

        // Create a DAG: 1 -> 2 -> 3
        graph.add_dependency(sym1, sym2);
        graph.add_dependency(sym2, sym3);

        let cycle = graph.find_cycle();
        assert!(cycle.is_none(), "No cycle should be detected in DAG");
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
}
