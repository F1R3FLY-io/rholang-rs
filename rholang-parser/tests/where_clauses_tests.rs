use rholang_parser::{
    RholangParser,
    ast::{BinaryExpOp, Proc},
};
use validated::Validated;

#[test]
fn receive_with_where_guard_populates_receipt_guard() {
    let parser = RholangParser::new();
    let parsed = parser.parse("for (@x <- a where x) { Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };
    assert_eq!(procs.len(), 1);

    match procs[0].proc {
        Proc::ForComprehension { receipts, .. } => {
            assert_eq!(receipts.len(), 1);
            let r = &receipts[0];
            assert_eq!(r.binds.len(), 1, "one bind expected");
            assert!(r.guard.is_some(), "guard should be populated by parser");
        }
        ref other => panic!("expected ForComprehension, got {other:?}"),
    }
}

#[test]
fn receive_without_where_has_no_guard() {
    let parser = RholangParser::new();
    let parsed = parser.parse("for (@x <- a) { Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::ForComprehension { receipts, .. } => {
            assert!(receipts[0].guard.is_none(), "no guard expected");
        }
        ref other => panic!("expected ForComprehension, got {other:?}"),
    }
}

#[test]
fn match_case_with_where_guard_populates_case_guard() {
    let parser = RholangParser::new();
    let parsed = parser.parse("match x { y where y => Nil _ => Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::Match { cases, .. } => {
            assert_eq!(cases.len(), 2);
            assert!(cases[0].guard.is_some(), "first case should have guard");
            assert!(
                cases[1].guard.is_none(),
                "second case (wildcard) should have no guard"
            );
        }
        ref other => panic!("expected Match, got {other:?}"),
    }
}

#[test]
fn match_without_where_has_no_guards() {
    let parser = RholangParser::new();
    let parsed = parser.parse("match x { 1 => Nil _ => Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::Match { cases, .. } => {
            for case in cases {
                assert!(case.guard.is_none());
            }
        }
        ref other => panic!("expected Match, got {other:?}"),
    }
}

#[test]
fn nested_for_with_per_receipt_guards() {
    let parser = RholangParser::new();
    let parsed = parser.parse("for (@x <- a where x; @y <- b where y) { Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::ForComprehension { receipts, .. } => {
            assert_eq!(receipts.len(), 2);
            assert!(receipts[0].guard.is_some(), "first receipt guard");
            assert!(receipts[1].guard.is_some(), "second receipt guard");
        }
        ref other => panic!("expected ForComprehension, got {other:?}"),
    }
}

#[test]
fn receive_join_with_guard() {
    // Join across &-separated binds with a single shared guard.
    let parser = RholangParser::new();
    let parsed = parser.parse("for (@x <- a & @y <- b where x) { Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::ForComprehension { receipts, .. } => {
            assert_eq!(receipts.len(), 1);
            assert_eq!(receipts[0].binds.len(), 2);
            assert!(
                receipts[0].guard.is_some(),
                "guard should attach to the &-joined receipt"
            );
        }
        ref other => panic!("expected ForComprehension, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// R5.1 edge cases flagged in PR #91 review (and a few extras)
// ---------------------------------------------------------------------------

#[test]
fn remainder_bind_with_guard() {
    // `for (@x, @y ...@rest <- chan where x > 0) { Nil }` — names list with
    // a remainder, attached to a single receipt that carries a `where` guard.
    let parser = RholangParser::new();
    let parsed = parser.parse("for (@x, @y ...@rest <- chan where x > 0) { Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::ForComprehension { receipts, .. } => {
            assert_eq!(receipts.len(), 1);
            assert_eq!(
                receipts[0].binds.len(),
                1,
                "one bind with multiple names + remainder"
            );
            assert!(
                receipts[0].guard.is_some(),
                "guard populated despite remainder"
            );
        }
        ref other => panic!("expected ForComprehension, got {other:?}"),
    }
}

#[test]
fn three_bind_atomic_join_with_guard() {
    // Three-or-more &-joined binds sharing a single guard.
    let parser = RholangParser::new();
    let parsed = parser.parse("for (@x <- a & @y <- b & @z <- c where x + y + z > 0) { Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::ForComprehension { receipts, .. } => {
            assert_eq!(receipts.len(), 1);
            assert_eq!(receipts[0].binds.len(), 3);
            assert!(
                receipts[0].guard.is_some(),
                "guard should attach to the 3-bind join"
            );
        }
        ref other => panic!("expected ForComprehension, got {other:?}"),
    }
}

#[test]
fn nested_for_comp_inner_guard_refs_outer_var() {
    // Genuinely nested for-comprehensions, inner guard mentions outer-bound
    // variable. This is the case the review flagged as missing — the existing
    // `nested_for_with_per_receipt_guards` test only exercises ;-separated
    // receipts, not actually-nested fors.
    let parser = RholangParser::new();
    let parsed = parser.parse("for (@x <- a) { for (@y <- b where y > x) { Nil } }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::ForComprehension {
            receipts: outer_receipts,
            proc: outer_body,
        } => {
            assert_eq!(outer_receipts.len(), 1);
            assert!(
                outer_receipts[0].guard.is_none(),
                "outer receipt has no guard"
            );
            match outer_body.proc {
                Proc::ForComprehension {
                    receipts: inner_receipts,
                    ..
                } => {
                    assert_eq!(inner_receipts.len(), 1);
                    assert!(
                        inner_receipts[0].guard.is_some(),
                        "inner for-comp receipt should carry the guard"
                    );
                }
                ref other => panic!("expected inner ForComprehension, got {other:?}"),
            }
        }
        ref other => panic!("expected outer ForComprehension, got {other:?}"),
    }
}

#[test]
fn for_receipt_guard_is_match_expression() {
    // Guard payload is itself a `match` expression — the AST shape this
    // produces drives downstream classification (e.g., f1r3node-rust's
    // `EMatchExpr` recognizer).
    let parser = RholangParser::new();
    let parsed = parser.parse("for (@x <- a where match x { 1 => true _ => false }) { Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::ForComprehension { receipts, .. } => {
            assert_eq!(receipts.len(), 1);
            let guard = receipts[0].guard.as_ref().expect("guard should be present");
            assert!(
                matches!(guard.proc, Proc::Match { .. }),
                "guard should be Proc::Match, got {:?}",
                guard.proc
            );
        }
        ref other => panic!("expected ForComprehension, got {other:?}"),
    }
}

#[test]
fn match_case_guard_is_match_expression() {
    // Match case whose guard is itself a `match` — same shape, different
    // host node.
    let parser = RholangParser::new();
    let parsed = parser.parse("match x { y where match y { _ => true } => big _ => small }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::Match { cases, .. } => {
            assert_eq!(cases.len(), 2);
            let guard = cases[0]
                .guard
                .as_ref()
                .expect("first case should have a guard");
            assert!(
                matches!(guard.proc, Proc::Match { .. }),
                "guard should be Proc::Match, got {:?}",
                guard.proc
            );
            assert!(
                cases[1].guard.is_none(),
                "second (wildcard) case should not have a guard"
            );
        }
        ref other => panic!("expected Match, got {other:?}"),
    }
}

#[test]
fn match_case_guard_is_arithmetic_expression() {
    // Guards are arbitrary procs — common shape is a comparison/arithmetic.
    let parser = RholangParser::new();
    let parsed = parser.parse("match v { n where n + 1 > 0 => big _ => small }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::Match { cases, .. } => {
            let guard = cases[0]
                .guard
                .as_ref()
                .expect("first case should have a guard");
            // Top-level should be `> 0`, with `n + 1` on the left.
            match guard.proc {
                Proc::BinaryExp {
                    op: BinaryExpOp::Gt,
                    ..
                } => {}
                ref other => panic!("expected BinaryExp(Gt), got {other:?}"),
            }
        }
        ref other => panic!("expected Match, got {other:?}"),
    }
}

#[test]
fn for_receipt_guard_with_method_call() {
    // Guard payload is a method call; this exercises a dot-notation path
    // through the parser inside a guard.
    let parser = RholangParser::new();
    let parsed = parser.parse("for (@xs <- a where xs.length() > 0) { Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::ForComprehension { receipts, .. } => {
            let guard = receipts[0].guard.as_ref().expect("guard should be present");
            // Top-level is a comparison; the lhs is the method call.
            match guard.proc {
                Proc::BinaryExp {
                    op: BinaryExpOp::Gt,
                    left,
                    ..
                } => {
                    assert!(
                        matches!(left.proc, Proc::Method { .. }),
                        "lhs should be a Method, got {:?}",
                        left.proc
                    );
                }
                ref other => panic!("expected BinaryExp(Gt), got {other:?}"),
            }
        }
        ref other => panic!("expected ForComprehension, got {other:?}"),
    }
}

#[test]
fn match_with_some_cases_guarded_others_not() {
    // Mixed: per-case guard population varies across the cases vector.
    let parser = RholangParser::new();
    let parsed = parser.parse("match v { 0 => zero n where n > 0 => pos _ => neg_or_other }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::Match { cases, .. } => {
            assert_eq!(cases.len(), 3);
            assert!(cases[0].guard.is_none(), "literal case has no guard");
            assert!(cases[1].guard.is_some(), "named case has guard");
            assert!(cases[2].guard.is_none(), "wildcard case has no guard");
        }
        ref other => panic!("expected Match, got {other:?}"),
    }
}

#[test]
fn for_receipt_guard_is_boolean_combinator() {
    // Guard with `and`/`or` — checks that connectives parse inside guards.
    let parser = RholangParser::new();
    let parsed = parser.parse("for (@x <- a where x > 0 and x < 100) { Nil }");

    let procs = match parsed {
        Validated::Good(p) => p,
        Validated::Fail(e) => panic!("parse failed: {e:?}"),
    };

    match procs[0].proc {
        Proc::ForComprehension { receipts, .. } => {
            let guard = receipts[0].guard.as_ref().expect("guard");
            assert!(
                matches!(
                    guard.proc,
                    Proc::BinaryExp {
                        op: BinaryExpOp::And,
                        ..
                    }
                ),
                "expected BinaryExp(And) at top level, got {:?}",
                guard.proc
            );
        }
        ref other => panic!("expected ForComprehension, got {other:?}"),
    }
}
