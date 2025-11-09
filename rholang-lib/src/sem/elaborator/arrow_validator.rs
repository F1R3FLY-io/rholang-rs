//! Arrow type homogeneity validation for for-comprehensions

use crate::sem::{Diagnostic, ErrorKind, PID, SemanticDb};
use rholang_parser::SourcePos;
use rholang_parser::ast::{Bind, Receipts, Source};
use smallvec::SmallVec;

/// Arrow type for a binding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArrowType {
    /// Linear receive: `x <- channel`
    Linear,
    /// Repeated/persistent receive: `x <= channel`
    Repeated,
    /// Peek/non-consuming receive: `x <<- channel`
    Peek,
}

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
    /// all bindings use the same arrow type.
    ///
    /// # Arguments
    ///
    /// * `pid` - The PID of the for-comprehension being validated
    /// * `receipts` - The receipts from a for-comprehension AST node
    ///
    /// # Returns
    ///
    /// * `None` - All concurrent join groups have homogeneous arrow types
    /// * `Some(Diagnostic)` - Found mixed arrow types error
    pub(super) fn validate(&self, pid: PID, receipts: &'ast Receipts<'ast>) -> Option<Diagnostic> {
        for (receipt_idx, receipt) in receipts.iter().enumerate() {
            if let Some(diagnostic) = self.validate_receipt(pid, receipt_idx, receipt) {
                return Some(diagnostic);
            }
        }
        None
    }

    /// Validate a single receipt (concurrent join group)
    ///
    /// Checks that all bindings in this concurrent group use the same arrow type.
    ///
    /// # Arguments
    ///
    /// * `pid` - The PID of the for-comprehension
    /// * `receipt_idx` - Index of this receipt (for error reporting)
    /// * `receipt` - The bindings in this concurrent join group
    ///
    /// # Returns
    ///
    /// * `None` - All bindings use the same arrow type
    /// * `Some(Diagnostic)` - Found different arrow types
    fn validate_receipt(
        &self,
        pid: PID,
        receipt_idx: usize,
        receipt: &[Bind<'ast>],
    ) -> Option<Diagnostic> {
        // Single bindings are always valid
        if receipt.len() <= 1 {
            return None;
        }

        // Collect all arrow types in this concurrent group
        let mut arrow_types: SmallVec<[ArrowType; 3]> = SmallVec::new();
        let mut first_pos = None;

        for bind in receipt.iter() {
            let (arrow_type, pos) = self.extract_arrow_type(bind);

            if first_pos.is_none() {
                first_pos = Some(pos);
            }

            if !arrow_types.contains(&arrow_type) {
                arrow_types.push(arrow_type);
            }

            if arrow_types.len() > 1 {
                return Some(Diagnostic::error(
                    pid,
                    ErrorKind::MixedArrowTypes {
                        receipt_index: receipt_idx,
                    },
                    first_pos,
                ));
            }
        }

        None
    }

    /// Extract arrow type and position from a binding
    ///
    /// Determines which arrow type this binding uses and extracts a source
    /// position for error reporting.
    ///
    /// # Arguments
    ///
    /// * `bind` - The binding to analyze
    ///
    /// # Returns
    ///
    /// A tuple of `(ArrowType, SourcePos)` for this binding
    fn extract_arrow_type(&self, bind: &Bind<'ast>) -> (ArrowType, SourcePos) {
        match bind {
            Bind::Linear { rhs, .. } => {
                let pos = Self::get_source_position(rhs);
                (ArrowType::Linear, pos)
            }
            Bind::Repeated { rhs, .. } => {
                let pos = Self::get_name_position(rhs);
                (ArrowType::Repeated, pos)
            }
            Bind::Peek { rhs, .. } => {
                let pos = Self::get_name_position(rhs);
                (ArrowType::Peek, pos)
            }
        }
    }

    /// Get source position from Source
    ///
    /// Extracts position information from a `Source` AST node
    /// Source can be Simple, ReceiveSend, or SendReceive
    fn get_source_position(source: &Source) -> SourcePos {
        match source {
            Source::Simple { name } | Source::ReceiveSend { name } => Self::get_name_position(name),
            Source::SendReceive { name, .. } => Self::get_name_position(name),
        }
    }

    /// Get source position from Name
    ///
    /// Extracts position information from a `Name` AST node
    /// Name can be either a NameVar or a Quote
    fn get_name_position(name: &rholang_parser::ast::Name) -> SourcePos {
        use rholang_parser::ast::{Name, Var};
        match name {
            Name::NameVar(var) => match var {
                Var::Id(id) => id.pos,
                Var::Wildcard => SourcePos::default(),
            },
            Name::Quote(proc) => proc.span.start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::SemanticDb;
    use rholang_parser::RholangParser;

    /// Helper to assert validation succeeds
    fn assert_validates(code: &str) {
        let parser = RholangParser::new();
        let ast = parser.parse(code).expect("parse error");
        let mut db = SemanticDb::new();
        let pid = db.build_index(&ast[0]);

        for proc in ast.iter() {
            if let rholang_parser::ast::Proc::ForComprehension { receipts, .. } = proc.proc {
                let validator = ArrowTypeValidator::new(&db);
                assert!(
                    validator.validate(pid, receipts).is_none(),
                    "Expected validation to succeed for: {}",
                    code
                );
                return;
            }
        }

        panic!("No for-comprehension found in: {}", code);
    }

    /// Helper to assert validation fails with MixedArrowTypes error
    fn assert_mixed_arrows_error(code: &str) {
        let parser = RholangParser::new();
        let ast = parser.parse(code).expect("parse error");
        let mut db = SemanticDb::new();
        let pid = db.build_index(&ast[0]);

        for proc in ast.iter() {
            if let rholang_parser::ast::Proc::ForComprehension { receipts, .. } = proc.proc {
                let validator = ArrowTypeValidator::new(&db);
                match validator.validate(pid, receipts) {
                    Some(diagnostic) => {
                        // Verify it's a MixedArrowTypes error
                        use crate::sem::{DiagnosticKind, ErrorKind};
                        assert!(
                            matches!(
                                diagnostic.kind,
                                DiagnosticKind::Error(ErrorKind::MixedArrowTypes { .. })
                            ),
                            "Expected MixedArrowTypes error, got: {:?}",
                            diagnostic
                        );
                        return;
                    }
                    None => panic!("Expected MixedArrowTypes error for: {}", code),
                }
            }
        }

        panic!("No for-comprehension found in: {}", code);
    }

    // Homogeneous Concurrent Groups (Valid)

    #[test]
    fn test_all_linear_concurrent() {
        assert_validates("for(@x <- ch1 & @y <- ch2 & @z <- ch3) { Nil }");
    }

    #[test]
    fn test_all_repeated_concurrent() {
        assert_validates("for(@x <= ch1 & @y <= ch2) { Nil }");
    }

    #[test]
    fn test_all_peek_concurrent() {
        assert_validates("for(@x <<- ch1 & @y <<- ch2) { Nil }");
    }

    #[test]
    fn test_single_linear() {
        assert_validates("for(@x <- ch) { Nil }");
    }

    #[test]
    fn test_single_repeated() {
        assert_validates("for(@x <= ch) { Nil }");
    }

    #[test]
    fn test_single_peek() {
        assert_validates("for(@x <<- ch) { Nil }");
    }

    // Mixed Concurrent Groups (Invalid)

    #[test]
    fn test_linear_and_repeated_error() {
        assert_mixed_arrows_error("for(@x <- ch1 & @y <= ch2) { Nil }");
    }

    #[test]
    fn test_linear_and_peek_error() {
        assert_mixed_arrows_error("for(@x <- ch1 & @y <<- ch2) { Nil }");
    }

    #[test]
    fn test_repeated_and_peek_error() {
        assert_mixed_arrows_error("for(@x <= ch1 & @y <<- ch2) { Nil }");
    }

    #[test]
    fn test_all_three_types_error() {
        assert_mixed_arrows_error("for(@x <- ch1 & @y <= ch2 & @z <<- ch3) { Nil }");
    }

    // Sequential Groups with Different Types (Valid)

    #[test]
    fn test_sequential_different_types() {
        assert_validates("for(@x <- ch1; @y <= ch2; @z <<- ch3) { Nil }");
    }

    #[test]
    fn test_mixed_sequential_and_concurrent() {
        assert_validates("for(@a <- ch1 & @b <- ch2; @c <= ch3 & @d <= ch4) { Nil }");
    }

    #[test]
    fn test_sequential_all_combinations() {
        assert_validates(
            "for(@w <- ch1 & @x <- ch2; @y <= ch3 & @z <= ch4; @p <<- ch5 & @q <<- ch6) { Nil }",
        );
    }

    // Edge Cases

    #[test]
    fn test_same_channel_different_patterns() {
        // Multiple patterns from the same channel - valid with same arrow type
        assert_validates("for(@x, @y <- ch) { Nil }");
    }

    #[test]
    fn test_complex_patterns() {
        assert_validates("for(@[x, y] <- ch1 & @{a /\\ b} <- ch2) { Nil }");
    }

    // Real-World Examples

    #[test]
    fn test_join_pattern() {
        assert_validates("for(@true <- ack & @false <- nack) { Nil }");
    }

    #[test]
    fn test_replicated_server() {
        assert_validates("for(@request <= server) { Nil }");
    }

    #[test]
    fn test_peek_pattern() {
        assert_validates("for(@value <<- state) { Nil }");
    }
}
