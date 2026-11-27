//! `foreach`, `try` and `agent` are contextual keywords: they start their
//! block forms, but stay usable as ordinary identifiers elsewhere. Malformed
//! `foreach` binds are reported as parse failures, never as panics.

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

#[rstest]
#[case::foreach_applied("foreach(1)")]
#[case::foreach_channel("new foreach in { foreach!(1) }")]
#[case::foreach_payload("new x in { x!(foreach) }")]
#[case::foreach_receive_binder("new c in { for (foreach <- c) { foreach!(1) } }")]
#[case::try_applied("try(1)")]
#[case::try_channel("new try in { try!(1) }")]
#[case::agent_applied("agent(1)")]
#[case::agent_channel("new agent in { agent!(1) }")]
#[case::all_as_payload("new x in { x!(foreach, try, agent) }")]
fn contextual_keywords_parse_as_identifiers(#[case] source: &str) {
    assert!(parses(source), "expected {source:?} to parse");
}

#[test]
fn foreach_with_binds_is_still_a_comprehension() {
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
#[case::try_block("new s in { try @v <- s { Nil } catch @e { Nil } }")]
#[case::agent_block(
    "new foo in { agent foo { constructor() { Nil } | default(...@args) { Nil } } }"
)]
fn keyword_block_forms_still_parse(#[case] source: &str) {
    assert!(parses(source), "expected {source:?} to parse");
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
