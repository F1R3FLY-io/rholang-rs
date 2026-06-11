//! AST-level tests for cost-accounted Rholang surface syntax, aligned with Greg's
//! concrete-syntax appendix (cost-accounted-rho.tex §app:concrete): signed terms
//! `{% P %}[s]`, bare token stacks `s :: … :: ()` (no `purse(...)` wrapper),
//! signatures (ground / `# P` hash / `(*)` compound / `-o` lollipop), per-clause
//! signed binds `{% y <- x %}[s]`, and signed `for(...) {% P %}` continuations.

use rholang_parser::{
    ast::{Bind, Name, Proc, Signature, Var},
    RholangParser,
};
use validated::Validated;

fn parse_one<'a>(parser: &'a RholangParser<'a>, src: &'a str) -> rholang_parser::ast::AnnProc<'a> {
    match parser.parse(src) {
        Validated::Good(procs) => {
            assert_eq!(procs.len(), 1, "expected exactly one top-level process");
            procs.into_iter().next().unwrap()
        }
        Validated::Fail(e) => panic!("parse failed for {src:?}: {e:?}"),
    }
}

fn assert_ground(sig: &Signature, expected: &str) {
    match sig {
        Signature::Ground(Name::NameVar(Var::Id(id))) => {
            assert_eq!(id.name, expected, "ground sig name")
        }
        other => panic!("expected ground sig {expected:?}, got {other:?}"),
    }
}

#[test]
fn signed_term_atomic_ground() {
    let parser = RholangParser::new();
    let proc = parse_one(&parser, "{% Nil %}[ s ]");
    match proc.proc {
        Proc::SignedTerm { proc, sig } => {
            assert!(matches!(proc.proc, Proc::Nil), "body should be Nil");
            assert_ground(sig, "s");
        }
        other => panic!("expected SignedTerm, got {other:?}"),
    }
}

#[test]
fn signed_term_compound_left_assoc() {
    let parser = RholangParser::new();
    // `a (*) b (*) c` is left-associative: Compound(Compound(a, b), c).
    let proc = parse_one(&parser, "{% Nil %}[ a (*) b (*) c ]");
    match proc.proc {
        Proc::SignedTerm {
            sig: Signature::Compound(ab, c),
            ..
        } => {
            assert_ground(c, "c");
            match ab.as_ref() {
                Signature::Compound(a, b) => {
                    assert_ground(a, "a");
                    assert_ground(b, "b");
                }
                other => panic!("expected left-assoc Compound(a,b), got {other:?}"),
            }
        }
        other => panic!("expected compound SignedTerm, got {other:?}"),
    }
}

#[test]
fn signed_term_hash_sig_complex_via_block() {
    let parser = RholangParser::new();
    // `# P` takes a high-precedence principal; a complex process is wrapped `# { P }`.
    let proc = parse_one(&parser, "{% Nil %}[ # { x!(1) } ]");
    match proc.proc {
        Proc::SignedTerm {
            sig: Signature::Hash(body),
            ..
        } => assert!(
            matches!(body.proc, Proc::Send { .. }),
            "hash body is the (block-unwrapped) process"
        ),
        other => panic!("expected hash SignedTerm, got {other:?}"),
    }
}

#[test]
fn signed_term_hash_sig_simple() {
    let parser = RholangParser::new();
    let proc = parse_one(&parser, "{% Nil %}[ # principal ]");
    match proc.proc {
        Proc::SignedTerm {
            sig: Signature::Hash(body),
            ..
        } => assert!(
            matches!(body.proc, Proc::ProcVar(_)),
            "hash body is the principal var"
        ),
        other => panic!("expected hash SignedTerm, got {other:?}"),
    }
}

#[test]
fn signed_term_lollipop_right_assoc() {
    let parser = RholangParser::new();
    // `a -o b -o c` is right-associative: Transfer(a, Transfer(b, c)).
    let proc = parse_one(&parser, "{% Nil %}[ a -o b -o c ]");
    match proc.proc {
        Proc::SignedTerm {
            sig: Signature::Transfer(a, bc),
            ..
        } => {
            assert_ground(a, "a");
            match bc.as_ref() {
                Signature::Transfer(b, c) => {
                    assert_ground(b, "b");
                    assert_ground(c, "c");
                }
                other => panic!("expected right-assoc Transfer(b,c), got {other:?}"),
            }
        }
        other => panic!("expected transfer SignedTerm, got {other:?}"),
    }
}

#[test]
fn signature_mixed_precedence() {
    let parser = RholangParser::new();
    // ground/# tightest > (*) > -o  =>  `# Nil (*) b -o c` parses as
    // Transfer(Compound(Hash, Ground(b)), Ground(c)).
    let proc = parse_one(&parser, "{% Nil %}[ # Nil (*) b -o c ]");
    match proc.proc {
        Proc::SignedTerm {
            sig: Signature::Transfer(comp, c),
            ..
        } => {
            assert_ground(c, "c");
            match comp.as_ref() {
                Signature::Compound(hash, b) => {
                    assert!(matches!(hash.as_ref(), Signature::Hash(_)), "left is # P");
                    assert_ground(b, "b");
                }
                other => panic!("expected Compound(Hash, Ground), got {other:?}"),
            }
        }
        other => panic!("expected Transfer at the top, got {other:?}"),
    }
}

#[test]
fn token_stack_multi_layer_bare() {
    let parser = RholangParser::new();
    // Bare token stack — no `purse(...)` wrapper (Greg app:concrete `Stk`).
    let proc = parse_one(&parser, "a :: b :: ()");
    match proc.proc {
        Proc::TokenStack { stack } => {
            assert_eq!(stack.layers.len(), 2, "two atomic layers");
            assert_ground(&stack.layers[0], "a");
            assert_ground(&stack.layers[1], "b");
        }
        other => panic!("expected TokenStack, got {other:?}"),
    }
}

#[test]
fn empty_stack_is_unit() {
    let parser = RholangParser::new();
    // A bare empty `()` is plain unit/Nil, not a token stack.
    let proc = parse_one(&parser, "()");
    assert!(
        matches!(proc.proc, Proc::Unit),
        "empty () is Unit, got {:?}",
        proc.proc
    );
}

#[test]
fn parallel_stacks_parse() {
    // Located/parallel stacks are ordinary parallel composition (Greg StkPar).
    let parser = RholangParser::new();
    let _ = parse_one(&parser, "s :: () | t :: ()");
}

#[test]
fn ring_fence_via_new_bound_signature() {
    let parser = RholangParser::new();
    // Ring-fencing is via a `new`-bound signature (binding-sensitive Σ⟦s⟧).
    let proc = parse_one(&parser, "new s in { s :: () }");
    match proc.proc {
        Proc::New { proc: body, .. } => {
            assert!(
                matches!(body.proc, Proc::TokenStack { .. }),
                "new body is a token stack"
            );
        }
        other => panic!("expected New, got {other:?}"),
    }
}

#[test]
fn nested_signed_term() {
    let parser = RholangParser::new();
    let proc = parse_one(&parser, "{% {% Nil %}[ a ] %}[ b ]");
    match proc.proc {
        Proc::SignedTerm { proc: inner, sig } => {
            assert_ground(sig, "b");
            match inner.proc {
                Proc::SignedTerm { sig: inner_sig, .. } => assert_ground(inner_sig, "a"),
                other => panic!("expected nested SignedTerm, got {other:?}"),
            }
        }
        other => panic!("expected outer SignedTerm, got {other:?}"),
    }
}

#[test]
fn signed_term_as_send_payload() {
    let parser = RholangParser::new();
    let proc = parse_one(&parser, "ch!( {% Nil %}[ s ] )");
    match proc.proc {
        Proc::Send { inputs, .. } => {
            assert_eq!(inputs.len(), 1);
            assert!(
                matches!(inputs[0].proc, Proc::SignedTerm { .. }),
                "send payload is a signed term"
            );
        }
        other => panic!("expected Send, got {other:?}"),
    }
}

#[test]
fn token_stack_as_send_payload() {
    let parser = RholangParser::new();
    let proc = parse_one(&parser, "ch!( s :: () )");
    match proc.proc {
        Proc::Send { inputs, .. } => {
            assert!(
                matches!(inputs[0].proc, Proc::TokenStack { .. }),
                "send payload is a token stack"
            );
        }
        other => panic!("expected Send, got {other:?}"),
    }
}

#[test]
fn per_clause_signed_bind() {
    let parser = RholangParser::new();
    // Axis-C join: a signed bind `{% y <- x %}[s]` alongside a plain bind.
    let proc = parse_one(&parser, "for( {% y <- x %}[ s ] & @z <- w ){ Nil }");
    match proc.proc {
        Proc::ForComprehension { receipts, .. } => {
            assert_eq!(receipts.len(), 1, "one receipt");
            let binds = &receipts[0].binds;
            assert_eq!(binds.len(), 2, "two binds");
            match &binds[0] {
                Bind::Signed { sig, .. } => assert_ground(sig, "s"),
                other => panic!("expected first bind Signed, got {other:?}"),
            }
            assert!(
                matches!(&binds[1], Bind::Linear { .. }),
                "second bind is plain Linear"
            );
        }
        other => panic!("expected ForComprehension, got {other:?}"),
    }
}

#[test]
fn signed_for_continuation() {
    let parser = RholangParser::new();
    // `for(...) {% P %}` — the signed continuation unwraps to its inner process.
    let proc = parse_one(&parser, "for( x <- c ) {% @\"r\"!(1) %}");
    match proc.proc {
        Proc::ForComprehension { proc: body, .. } => {
            assert!(
                matches!(body.proc, Proc::Send { .. }),
                "continuation is the inner send"
            );
        }
        other => panic!("expected ForComprehension, got {other:?}"),
    }
}

#[test]
fn lollipop_does_not_break_subtraction() {
    // The `-o` token is confined to signature context; ordinary subtraction
    // with `o`-prefixed identifiers must still parse (Phase-0 spike result).
    let parser = RholangParser::new();
    for src in ["x - owed", "x-owed", "balance-owed"] {
        match parser.parse(src) {
            Validated::Good(_) => {}
            Validated::Fail(e) => panic!("`{src}` should parse as subtraction: {e:?}"),
        }
    }
}

#[test]
fn malformed_cost_syntax_is_rejected() {
    let parser = RholangParser::new();
    // Empty signature, incomplete stack, missing signature brackets.
    for src in ["{% Nil %}[ ]", "a :: ", "{% Nil %} s"] {
        assert!(
            matches!(parser.parse(src), Validated::Fail(_)),
            "`{src}` should fail to parse"
        );
    }
}

#[test]
fn ordinary_rholang_with_joins_still_parses() {
    // Regression: N-ary joins (already in the base grammar) are unaffected.
    let parser = RholangParser::new();
    let proc = parse_one(&parser, "for(x <- a & y <- b){ *x | *y }");
    assert!(matches!(proc.proc, Proc::ForComprehension { .. }));
}
