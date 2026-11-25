//! Malformed `foreach` binds are reported as parse failures, never as
//! panics, and well-formed comprehensions still parse.

use rholang_parser::RholangParser;
use rholang_parser::ast::Proc;
use rstest::rstest;
use validated::Validated;

/// Parses `source`, turning a panic into a test failure with the input named.
fn parses(source: &str) -> bool {
    let result = std::panic::catch_unwind(|| {
        let parser = RholangParser::new();
        matches!(parser.parse(source), Validated::Good(_))
    });
    result.unwrap_or_else(|_| panic!("parser panicked on {source:?}"))
}

#[test]
fn foreach_with_binds_is_a_comprehension() {
    let parser = RholangParser::new();
    let source = "new a, b in { foreach (x <- a & y <- b where x) yield (r) { r!(x) } }";
    let Validated::Good(procs) = parser.parse(source) else {
        panic!("expected {source:?} to parse");
    };
    let Proc::New { proc, .. } = procs[0].proc else {
        panic!("expected a `new` at the top level");
    };
    assert!(matches!(proc.proc, Proc::Foreach { .. }));
}

#[rstest]
#[case::doubled_separator("foreach (x <- a & & y <- b) yield (r) { Nil }")]
#[case::logical_and("foreach (x <- a && y <- b) yield (r) { Nil }")]
#[case::stray_literal("foreach (x <- a & 1 & y <- b) yield (r) { Nil }")]
#[case::trailing_separator("foreach (x <- a & y <- b &) yield (r) { Nil }")]
#[case::missing_arrow("foreach (x <- a & y b) yield (r) { Nil }")]
#[case::nested("new a, b in { foreach (x <- a & & y <- b) yield (r) { Nil } }")]
fn malformed_foreach_binds_fail_without_panicking(#[case] source: &str) {
    assert!(!parses(source), "expected {source:?} to be rejected");
}
