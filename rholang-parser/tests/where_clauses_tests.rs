use rholang_parser::{RholangParser, ast::Proc};
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
            assert!(receipts[0].guard.is_some(), "guard should attach to the &-joined receipt");
        }
        ref other => panic!("expected ForComprehension, got {other:?}"),
    }
}
