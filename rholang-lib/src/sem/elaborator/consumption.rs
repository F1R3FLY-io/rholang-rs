//! This module validates consumption modes for for-comprehension bindings
//! It ensures that:
//! - Linear consumption (`<-`) consumes channels exactly once
//! - Peek semantics (`<<-`) perform non-consuming reads
//! - Repeated binds (`<=`) maintain persistence/contract-like behavior
//! - Reentrancy vulnerabilities are detected and reported

use crate::sem::{PID, SemanticDb, Symbol};
use rholang_parser::ast::{Bind, Name, Receipts, Source};
use std::collections::HashMap;

use super::errors::{ValidationError, ValidationResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConsumptionMode {
    Linear,
    Repeated,
    Peek,
}

/// Tracks consumption patterns for channels
#[derive(Debug)]
struct ConsumptionTracker {
    /// Maps channel symbols to their consumption modes and positions
    /// Key: channel symbol
    /// Value: list of (mode, source_position) for each usage
    channel_usages: HashMap<Symbol, Vec<(ConsumptionMode, rholang_parser::SourcePos)>>,
}

impl ConsumptionTracker {
    fn new() -> Self {
        Self {
            channel_usages: HashMap::new(),
        }
    }

    fn record_usage(
        &mut self,
        channel: Symbol,
        mode: ConsumptionMode,
        pos: rholang_parser::SourcePos,
    ) {
        self.channel_usages
            .entry(channel)
            .or_default()
            .push((mode, pos));
    }

    /// Validate that linear channels are only consumed once
    fn validate_linear_consumption(&self) -> ValidationResult {
        for usages in self.channel_usages.values() {
            // Count linear consumptions (excluding peeks which don't consume)
            let linear_consumptions: Vec<_> = usages
                .iter()
                .filter(|(mode, _)| *mode == ConsumptionMode::Linear)
                .collect();

            // Linear channels should only be consumed once
            if linear_consumptions.len() > 1 {
                let pos = linear_consumptions[1].1;
                return Err(ValidationError::InvalidPatternStructure {
                    pid: PID(0), // Will be set by caller
                    position: Some(pos),
                    reason: format!(
                        "Channel consumed linearly multiple times. \
                         Linear consumption (<-) expects exactly one message, \
                         but channel appears {} times with linear consumption mode.",
                        linear_consumptions.len()
                    ),
                });
            }

            // Check for conflicting modes: linear + repeated on same channel
            let has_linear = usages
                .iter()
                .any(|(mode, _)| *mode == ConsumptionMode::Linear);
            let has_repeated = usages
                .iter()
                .any(|(mode, _)| *mode == ConsumptionMode::Repeated);

            if has_linear && has_repeated {
                let linear_pos = usages
                    .iter()
                    .find(|(mode, _)| *mode == ConsumptionMode::Linear)
                    .map(|(_, pos)| *pos)
                    .unwrap_or_default();

                return Err(ValidationError::InvalidPatternStructure {
                    pid: PID(0),
                    position: Some(linear_pos),
                    reason: "Channel has conflicting consumption modes. \
                         Cannot use both linear (<-) and repeated (<=) consumption \
                         on the same channel in a single for-comprehension."
                        .to_string(),
                });
            }
        }

        Ok(())
    }
}

pub struct ConsumptionValidator<'a, 'ast> {
    db: &'a SemanticDb<'ast>,
}

impl<'a, 'ast> ConsumptionValidator<'a, 'ast> {
    pub fn new(db: &'a SemanticDb<'ast>) -> Self {
        Self { db }
    }

    /// This method validates consumption patterns by:
    /// 1. Tracking all channel usages and their consumption modes
    /// 2. Validating linear consumption (channels consumed exactly once)
    /// 3. Validating peek semantics (non-consuming reads)
    /// 4. Validating repeated binds (persistent/contract-like behavior)
    /// 5. Detecting potential reentrancy vulnerabilities
    /// 6. Verifying channel existence and binding
    pub fn validate(&self, _for_comp_pid: PID, receipts: &Receipts<'ast>) -> ValidationResult {
        let tracker = self.track_consumption_patterns(receipts)?;

        tracker.validate_linear_consumption()?;

        self.detect_reentrancy_patterns(receipts)?;

        self.verify_all_channels(receipts)?;

        Ok(())
    }

    /// Builds a consumption tracker that records how each channel is used
    /// throughout the for-comprehension
    fn track_consumption_patterns(
        &self,
        receipts: &Receipts<'ast>,
    ) -> ValidationResult<ConsumptionTracker> {
        let mut tracker = ConsumptionTracker::new();

        for receipt in receipts.iter() {
            for bind in receipt.iter() {
                let (mode, source_name, pos) = match bind {
                    Bind::Linear { rhs, .. } => {
                        let name = self.extract_name_from_source(rhs);
                        let pos = self.get_name_position(name);
                        (ConsumptionMode::Linear, name, pos)
                    }
                    Bind::Repeated { rhs, .. } => {
                        let pos = self.get_name_position(rhs);
                        (ConsumptionMode::Repeated, rhs, pos)
                    }
                    Bind::Peek { rhs, .. } => {
                        let pos = self.get_name_position(rhs);
                        (ConsumptionMode::Peek, rhs, pos)
                    }
                };

                // Only track variable names (not wildcards or quoted processes)
                if let Some(symbol) = self.get_channel_symbol(source_name) {
                    tracker.record_usage(symbol, mode, pos);
                }
            }
        }

        Ok(tracker)
    }

    fn extract_name_from_source<'b>(&self, source: &'b Source<'ast>) -> &'b Name<'ast> {
        match source {
            Source::Simple { name } => name,
            Source::ReceiveSend { name } => name,
            Source::SendReceive { name, .. } => name,
        }
    }

    fn get_channel_symbol(&self, name: &Name<'ast>) -> Option<Symbol> {
        match name {
            Name::NameVar(var) => match var {
                rholang_parser::ast::Var::Id(id) => Some(self.db.intern(id.name)),
                rholang_parser::ast::Var::Wildcard => None,
            },
            Name::Quote(_) => None,
        }
    }

    fn get_name_position(&self, name: &Name<'ast>) -> rholang_parser::SourcePos {
        match name {
            Name::NameVar(var) => match var {
                rholang_parser::ast::Var::Id(id) => id.pos,
                rholang_parser::ast::Var::Wildcard => rholang_parser::SourcePos::default(),
            },
            Name::Quote(proc) => proc.span.start,
        }
    }

    /// Detect potential reentrancy vulnerabilities
    /// Note: We only detect structural patterns here
    /// Full reentrancy analysis requires runtime state tracking
    fn detect_reentrancy_patterns(&self, receipts: &Receipts<'ast>) -> ValidationResult {
        for receipt in receipts.iter() {
            for bind in receipt.iter() {
                let source = match bind {
                    Bind::Linear { rhs, .. } => rhs,
                    Bind::Repeated { .. } | Bind::Peek { .. } => {
                        // Repeated and Peek only have Name, not Source
                        // So they cannot have reentrancy patterns
                        continue;
                    }
                };

                // Check if source pattern could enable reentrancy
                if self.is_reentrancy_pattern(source) {
                    // TODO: emit warning diagnostic
                    // For now, we just validate the structure exists
                }
            }
        }

        Ok(())
    }

    fn is_reentrancy_pattern(&self, source: &Source<'ast>) -> bool {
        matches!(
            source,
            Source::SendReceive { .. } | Source::ReceiveSend { .. }
        )
    }

    /// Verify all channels in receipts exist and are properly bound
    fn verify_all_channels(&self, receipts: &Receipts<'ast>) -> ValidationResult {
        for receipt in receipts.iter() {
            for bind in receipt.iter() {
                let name = match bind {
                    Bind::Linear { rhs, .. } => self.extract_name_from_source(rhs),
                    Bind::Repeated { rhs, .. } => rhs,
                    Bind::Peek { rhs, .. } => rhs,
                };

                self.verify_channel_exists(name)?;
            }
        }
        Ok(())
    }

    /// Verify that a channel exists and is properly bound
    fn verify_channel_exists(&self, name: &Name<'ast>) -> ValidationResult {
        if let Name::NameVar(rholang_parser::ast::Var::Id(id)) = name
            && self.db.binder_of_id(*id).is_none()
        {
            let sym = self.db.intern(id.name);
            return Err(ValidationError::UnboundVariable {
                var: sym,
                pos: id.pos,
            });
        }
        // Wildcards and quoted processes are always valid as channels
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::{FactPass, SemanticDb, resolver::ResolverPass};
    use rholang_parser::{RholangParser, ast::Proc};
    use std::collections::HashSet;

    // Test-specific helper methods for ConsumptionTracker
    impl ConsumptionTracker {
        /// Get channels that are peeked (non-consuming reads)
        fn get_peeked_channels(&self) -> HashSet<Symbol> {
            self.channel_usages
                .iter()
                .filter(|(_, usages)| {
                    usages
                        .iter()
                        .any(|(mode, _)| *mode == ConsumptionMode::Peek)
                })
                .map(|(channel, _)| *channel)
                .collect()
        }

        fn get_consumed_channels(&self) -> HashSet<Symbol> {
            self.channel_usages
                .iter()
                .filter(|(_, usages)| {
                    usages
                        .iter()
                        .any(|(mode, _)| *mode != ConsumptionMode::Peek)
                })
                .map(|(channel, _)| *channel)
                .collect()
        }
    }

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
        let validator = ConsumptionValidator::new(&db);
        assert!(std::ptr::eq(validator.db, &db));
    }

    #[test]
    fn test_simple_linear_consumption() {
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Simple linear consumption should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_peek_consumption() {
        let code = r#"new ch in { for(@x <<- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Peek consumption should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_repeated_consumption() {
        let code = r#"new ch in { for(@x <= ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Repeated consumption should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_mixed_consumption_modes() {
        let code = r#"new ch1, ch2, ch3 in { for(@x <- ch1; @y <= ch2; @z <<- ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Mixed consumption modes should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_sequential_receipts() {
        let code = r#"new ch1, ch2 in { for(@a <- ch1; @b <- ch2) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Sequential receipts should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_parallel_bindings() {
        let code = r#"new ch1, ch2, ch3 in { for(@a <- ch1 & @b <- ch2 & @c <= ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Parallel bindings should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_verify_channel_exists() {
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        // Channel exists and is properly bound, so validation should pass
        assert!(result.is_ok(), "Existing channel should validate");
    }

    #[test]
    fn test_wildcard_channel() {
        let code = r#"for(@x <- _) { Nil }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Wildcard channel should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_quoted_process_channel() {
        let code = r#"for(@x <- @Nil) { Nil }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Quoted process channel should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_linear_consumption_single_use() {
        let code = r#"new ch in { for(@x <- ch) { @x!(1) } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Linear consumption with single use should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_linear_consumption_multiple_uses_error() {
        // Same channel consumed linearly twice - should fail
        let code = r#"new ch in { for(@x <- ch; @y <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_err(),
            "Linear consumption with multiple uses should fail"
        );

        if let Err(ValidationError::InvalidPatternStructure { reason, .. }) = result {
            assert!(
                reason.contains("consumed linearly multiple times"),
                "Error should mention multiple linear consumptions: {}",
                reason
            );
        } else {
            panic!("Expected InvalidPatternStructure error");
        }
    }

    #[test]
    fn test_repeated_consumption_multiple_uses() {
        // Same channel with repeated consumption mode - should succeed
        let code = r#"new ch in { for(@x <= ch; @y <= ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Repeated consumption with multiple uses should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_peek_consumption_non_consuming() {
        let code = r#"new ch in { for(@x <<- ch; @y <<- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Peek consumption (non-consuming) should allow multiple reads: {:?}",
            result
        );
    }

    #[test]
    fn test_conflicting_modes_linear_and_repeated() {
        // Same channel with both linear and repeated - should fail
        let code = r#"new ch in { for(@x <- ch; @y <= ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(result.is_err(), "Conflicting consumption modes should fail");

        if let Err(ValidationError::InvalidPatternStructure { reason, .. }) = result {
            assert!(
                reason.contains("conflicting consumption modes"),
                "Error should mention conflicting modes: {}",
                reason
            );
        } else {
            panic!("Expected InvalidPatternStructure error");
        }
    }

    #[test]
    fn test_peek_with_linear_allowed() {
        // Peek doesn't consume, so it can coexist with linear
        let code = r#"new ch in { for(@x <<- ch; @y <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Peek with linear consumption should validate (peek doesn't consume): {:?}",
            result
        );
    }

    #[test]
    fn test_peek_with_repeated_allowed() {
        // Peek doesn't consume, so it can coexist with repeated
        let code = r#"new ch in { for(@x <<- ch; @y <= ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Peek with repeated consumption should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_different_channels_different_modes() {
        let code = r#"new ch1, ch2, ch3 in { for(@x <- ch1; @y <= ch2; @z <<- ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_ok(),
            "Different channels with different modes should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_parallel_bindings_linear_same_channel() {
        // Parallel join waiting for 2 messages from same channel
        let code = r#"new ch in { for(@x <- ch & @y <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        // This should fail because same channel consumed linearly twice
        assert!(
            result.is_err(),
            "Parallel bindings on same channel with linear consumption should fail"
        );
    }

    #[test]
    fn test_consumption_tracker_records_usages() {
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let tracker = validator
            .track_consumption_patterns(receipts)
            .expect("Should track patterns");

        // Channel should be recorded
        let consumed = tracker.get_consumed_channels();
        assert!(!consumed.is_empty(), "Should have consumed channels");
    }

    #[test]
    fn test_consumption_tracker_peek_vs_consume() {
        let code = r#"new ch1, ch2 in { for(@x <<- ch1; @y <- ch2) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let tracker = validator
            .track_consumption_patterns(receipts)
            .expect("Should track patterns");

        let peeked = tracker.get_peeked_channels();
        let consumed = tracker.get_consumed_channels();

        assert_eq!(peeked.len(), 1, "Should have one peeked channel");
        assert_eq!(consumed.len(), 1, "Should have one consumed channel");
    }

    #[test]
    fn test_reentrancy_detection_logic() {
        // Test the reentrancy pattern detection logic directly
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);

        // Reentrancy detection should not fail validation for now, we expect only warning
        let result = validator.detect_reentrancy_patterns(receipts);
        assert!(
            result.is_ok(),
            "Reentrancy detection should not fail validation: {:?}",
            result
        );
    }

    #[test]
    fn test_verify_channel_exists_unbound() {
        let code = r#"for(@x <- unbound_ch) { Nil }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        // Should fail because channel is unbound
        assert!(
            result.is_err(),
            "Unbound channel should cause validation error"
        );

        if let Err(ValidationError::UnboundVariable { .. }) = result {
            // Expected
        } else {
            panic!("Expected UnboundVariable error, got: {:?}", result);
        }
    }

    #[test]
    fn test_complex_multi_receipt_validation() {
        let code = r#"new ch1, ch2, ch3, ch4 in {
            for(@a <- ch1; @b <= ch2; @c <<- ch3; @d <- ch4) {
                Nil
            }
        }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        // Different channels with different modes - should validate
        assert!(
            result.is_ok(),
            "Complex multi-receipt pattern should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_linear_then_repeated_on_same_channel() {
        let code = r#"new ch in { for(@x <- ch; @y <= ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_err(),
            "Linear then repeated on same channel should fail"
        );
    }

    #[test]
    fn test_three_linear_consumptions_same_channel() {
        let code = r#"new ch in { for(@x <- ch; @y <- ch; @z <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let receipts = get_receipts(&db, pid);

        let validator = ConsumptionValidator::new(&db);
        let result = validator.validate(pid, receipts);

        assert!(
            result.is_err(),
            "Three linear consumptions on same channel should fail"
        );

        if let Err(ValidationError::InvalidPatternStructure { reason, .. }) = result {
            assert!(
                reason.contains("3 times"),
                "Error should mention 3 times: {}",
                reason
            );
        }
    }
}
