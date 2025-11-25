//! Arrow type homogeneity validation for for-comprehensions

use crate::sem::{Diagnostic, ErrorKind, PID, SemanticDb};
use rholang_parser::ast::{Bind, Receipts};

/// Validates arrow type homogeneity in for-comprehensions
///
/// This validator ensures that within each concurrent join group (bindings
/// separated by `&`), all bindings use the same arrow type
pub struct ArrowTypeValidator<'a, 'ast> {
    #[allow(dead_code)] // Reserved for future semantic checks
    db: &'a SemanticDb<'ast>,
}

impl<'a, 'ast> ArrowTypeValidator<'a, 'ast> {
    pub fn new(db: &'a SemanticDb<'ast>) -> Self {
        Self { db }
    }

    /// Validate that all concurrent join groups have homogeneous arrow types
    ///
    /// This is the main entry point for validation. It iterates over each
    /// receipt (sequential group) and validates that within each receipt,
    /// all bindings use the same arrow type
    ///
    /// # Arguments
    ///
    /// * `pid` - The PID of the for-comprehension being validated
    /// * `receipts` - The receipts from a for-comprehension AST node
    /// * `diagnostics` - Mutable vector to collect any errors found
    pub(super) fn validate(
        &self,
        pid: PID,
        receipts: &'ast Receipts<'ast>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for (receipt_idx, receipt) in receipts.iter().enumerate() {
            self.validate_receipt(pid, receipt_idx, receipt, diagnostics);
        }
    }

    /// Validate a single receipt (concurrent join group)
    ///
    /// Checks that all bindings in this concurrent group use the same arrow type
    /// If mixed arrow types are found, a diagnostic is added to the diagnostics vector
    ///
    /// # Arguments
    ///
    /// * `pid` - The PID of the for-comprehension
    /// * `receipt_idx` - Index of this receipt (for error reporting)
    /// * `receipt` - The bindings in this concurrent join group
    /// * `diagnostics` - Mutable vector to collect any errors found
    fn validate_receipt(
        &self,
        pid: PID,
        receipt_idx: usize,
        receipt: &[Bind<'ast>],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Single bindings are always valid
        if receipt.len() <= 1 {
            return;
        }

        // Check if all bindings use the same arrow type as the first one
        let first_pos = receipt[0].position();
        let expected_arrow = Self::arrow_type_name(&receipt[0]);

        // Find the first binding with a different arrow type
        for bind in &receipt[1..] {
            let current_arrow = Self::arrow_type_name(bind);
            if current_arrow != expected_arrow {
                diagnostics.push(Diagnostic::error(
                    pid,
                    ErrorKind::MixedArrowTypes {
                        receipt_index: receipt_idx,
                        expected: expected_arrow,
                        found: current_arrow,
                    },
                    Some(first_pos),
                ));
                return;
            }
        }
    }

    /// Get the arrow type name for a binding
    fn arrow_type_name(bind: &Bind) -> &'static str {
        match bind {
            Bind::Linear { .. } => "linear (<-)",
            Bind::Repeated { .. } => "repeated (<=)",
            Bind::Peek { .. } => "peek (<<-)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::{DiagnosticKind, ErrorKind, SemanticDb};
    use rholang_parser::ast::{AnnProc, Proc};
    use test_macros::test_rholang_code;

    /// Helper to validate that a for-comprehension has homogeneous arrow types
    fn assert_valid_arrows(db: &SemanticDb) {
        for (pid, proc_ref) in db.filter_procs(|p| matches!(p.proc, Proc::ForComprehension { .. }))
        {
            if let Proc::ForComprehension { receipts, .. } = proc_ref.proc {
                let validator = ArrowTypeValidator::new(db);
                let mut diagnostics = Vec::new();
                validator.validate(pid, receipts, &mut diagnostics);
                assert!(
                    diagnostics.is_empty(),
                    "Expected no errors, but got: {diagnostics:?}"
                );
                return;
            }
        }
        panic!("No for-comprehension found");
    }

    /// Helper to validate that a for-comprehension has mixed arrow types (error case)
    fn assert_mixed_arrows(db: &SemanticDb) {
        for (pid, proc_ref) in db.filter_procs(|p| matches!(p.proc, Proc::ForComprehension { .. }))
        {
            if let Proc::ForComprehension { receipts, .. } = proc_ref.proc {
                let validator = ArrowTypeValidator::new(db);
                let mut diagnostics = Vec::new();
                validator.validate(pid, receipts, &mut diagnostics);
                assert!(!diagnostics.is_empty(), "Expected MixedArrowTypes error");
                assert!(matches!(
                    diagnostics[0].kind,
                    DiagnosticKind::Error(ErrorKind::MixedArrowTypes { .. })
                ));
                return;
            }
        }
        panic!("No for-comprehension found");
    }

    // Homogeneous Concurrent Groups (Valid)

    #[test_rholang_code("for(@x <- ch1 & @y <- ch2 & @z <- ch3) { Nil }")]
    fn test_all_linear_concurrent(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    #[test_rholang_code("for(@x <= ch1 & @y <= ch2) { Nil }")]
    fn test_all_repeated_concurrent(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    #[test_rholang_code("for(@x <<- ch1 & @y <<- ch2) { Nil }")]
    fn test_all_peek_concurrent(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    #[test_rholang_code("for(@x <- ch) { Nil }")]
    fn test_single_linear(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    #[test_rholang_code("for(@x <= ch) { Nil }")]
    fn test_single_repeated(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    #[test_rholang_code("for(@x <<- ch) { Nil }")]
    fn test_single_peek(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    // Mixed Concurrent Groups (Invalid)

    #[test_rholang_code("for(@x <- ch1 & @y <= ch2) { Nil }")]
    fn test_linear_and_repeated_error(_procs: &[AnnProc], db: &SemanticDb) {
        assert_mixed_arrows(db);
    }

    #[test_rholang_code("for(@x <- ch1 & @y <<- ch2) { Nil }")]
    fn test_linear_and_peek_error(_procs: &[AnnProc], db: &SemanticDb) {
        assert_mixed_arrows(db);
    }

    #[test_rholang_code("for(@x <= ch1 & @y <<- ch2) { Nil }")]
    fn test_repeated_and_peek_error(_procs: &[AnnProc], db: &SemanticDb) {
        assert_mixed_arrows(db);
    }

    #[test_rholang_code("for(@x <- ch1 & @y <= ch2 & @z <<- ch3) { Nil }")]
    fn test_all_three_types_error(_procs: &[AnnProc], db: &SemanticDb) {
        assert_mixed_arrows(db);
    }

    // Sequential Groups with Different Types (Valid)

    #[test_rholang_code("for(@x <- ch1; @y <= ch2; @z <<- ch3) { Nil }")]
    fn test_sequential_different_types(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    #[test_rholang_code("for(@a <- ch1 & @b <- ch2; @c <= ch3 & @d <= ch4) { Nil }")]
    fn test_mixed_sequential_and_concurrent(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    #[test_rholang_code(
        "for(@w <- ch1 & @x <- ch2; @y <= ch3 & @z <= ch4; @p <<- ch5 & @q <<- ch6) { Nil }"
    )]
    fn test_sequential_all_combinations(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    // Edge Cases

    #[test_rholang_code("for(@x, @y <- ch) { Nil }")]
    fn test_same_channel_different_patterns(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    #[test_rholang_code("for(@[x, y] <- ch1 & @{a /\\ b} <- ch2) { Nil }")]
    fn test_complex_patterns(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    // Real-World Examples

    #[test_rholang_code("for(@true <- ack & @false <- nack) { Nil }")]
    fn test_join_pattern(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    #[test_rholang_code("for(@request <= server) { Nil }")]
    fn test_replicated_server(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }

    #[test_rholang_code("for(@value <<- state) { Nil }")]
    fn test_peek_pattern(_procs: &[AnnProc], db: &SemanticDb) {
        assert_valid_arrows(db);
    }
}
