//! Equivalence tests for the parse-time desugaring landed in this PR.
//!
//! The golden snapshots show the shape of `x!y(args)` after parsing
//! (a `SendSync` with the method-name `StringLiteral` prepended), but
//! they don't prove that this shape MATCHES the AST produced by the
//! hand-written `x!?("y", args)` form. These tests close that gap:
//! each case parses both forms and asserts the resulting ASTs agree
//! after spans are elided.
//!
//! Spans are elided because the synthesized `StringLiteral` inherits
//! the method `var`'s position (a 1-character span at the bare
//! identifier), whereas the hand-written form has the 3-character
//! span of `"y"`. The semantic content -- channel, inputs, method
//! name, continuation shape -- is what matters for the equivalence
//! the desugaring promises.

use rholang_parser::RholangParser;
use rstest::rstest;
use std::fs;
use std::path::PathBuf;

/// Strip `span: SourceSpan { ... }` and `pos: SourcePos { ... }`
/// sub-trees from a Debug-formatted AST so two ASTs that differ only
/// in source positions compare equal. A line-based filter is enough:
/// the Debug formatter puts each span/pos field on its own line, and
/// `SourcePos`/`SourceSpan` payloads occupy a contiguous block of
/// lines indented further than the `span:` / `pos:` opener.
fn strip_positions(debug_str: &str) -> String {
    let mut out = String::with_capacity(debug_str.len());
    let mut skip_until_indent: Option<usize> = None;

    for line in debug_str.lines() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();

        if let Some(open_indent) = skip_until_indent {
            // Keep skipping lines that are MORE indented than the
            // span:/pos: opener, plus the closing `}` line at the
            // exact same indent.
            if indent > open_indent {
                continue;
            }
            // Same-indent line: this is the closing `},` of the
            // SourceSpan / SourcePos block. Skip it too, then turn
            // off the skip filter.
            if trimmed.starts_with('}') {
                skip_until_indent = None;
                continue;
            }
            // Fell out of the block another way (shouldn't happen
            // with rustfmt-formatted Debug output, but be safe).
            skip_until_indent = None;
        }

        if trimmed.starts_with("span: SourceSpan {") || trimmed.starts_with("pos: SourcePos {") {
            skip_until_indent = Some(indent);
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    out
}

/// Parse `source` and return the Validated outcome formatted with `{:#?}`
/// and stripped of source positions.
fn parse_stripped(source: &str) -> String {
    let parser = RholangParser::new();
    let result = parser.parse(source);
    strip_positions(&format!("{result:#?}"))
}

/// Read a corpus file from `tests/corpus/<name>.rho` and return its
/// parsed-and-stripped form. Used by the agent-block equivalence
/// tests below to pair sugared and hand-written desugared files.
fn parse_corpus_stripped(name: &str) -> String {
    let mut path: PathBuf = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR set by cargo")
        .into();
    path.push("tests/corpus");
    path.push(format!("{name}.rho"));
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    parse_stripped(&source)
}

#[rstest]
#[case::terminator(
    r#"new x in { x!y(1, 2). }"#,
    r#"new x in { x!?("y", 1, 2). }"#,
)]
#[case::terminator_no_args(
    r#"new x in { x!y(). }"#,
    r#"new x in { x!?("y"). }"#,
)]
#[case::sequential(
    r#"new x in { x!set(42); Nil }"#,
    r#"new x in { x!?("set", 42); Nil }"#,
)]
#[case::sequential_uses_outer_scope(
    r#"new x, z in { x!set(42); z!(1) }"#,
    r#"new x, z in { x!?("set", 42); z!(1) }"#,
)]
#[case::comparison_args(
    r#"new x, a, b, c, d in { x!y(a < b, c > d). }"#,
    r#"new x, a, b, c, d in { x!?("y", a < b, c > d). }"#,
)]
#[case::bundle_arg(
    r#"new x, t in { x!y(bundle+{*t}). }"#,
    r#"new x, t in { x!?("y", bundle+{*t}). }"#,
)]
#[case::nested_send_arg(
    r#"new x, a, b in { x!y(a!(b)). }"#,
    r#"new x, a, b in { x!?("y", a!(b)). }"#,
)]
#[case::nested_send_method_arg(
    r#"new x, a, b in { x!y(a!z(b).). }"#,
    r#"new x, a, b in { x!?("y", a!?("z", b).). }"#,
)]
fn proc_position_desugars_to_send_sync(#[case] sugared: &str, #[case] hand_written: &str) {
    let s = parse_stripped(sugared);
    let h = parse_stripped(hand_written);
    pretty_assertions::assert_eq!(s, h, "x!y(args) AST should match x!?(\"y\", args) AST");
}

#[rstest]
#[case::for_source(
    r#"new x in { for (@z <- x!get()) { Nil } }"#,
    r#"new x in { for (@z <- x!?("get")) { Nil } }"#,
)]
#[case::for_source_with_args(
    r#"new x in { for (@z <- x!compute(1, 2, 3)) { Nil } }"#,
    r#"new x in { for (@z <- x!?("compute", 1, 2, 3)) { Nil } }"#,
)]
#[case::for_source_with_body(
    r#"new x, ret in { for (@val <- x!get()) { ret!(val) } }"#,
    r#"new x, ret in { for (@val <- x!?("get")) { ret!(val) } }"#,
)]
fn for_source_desugars_to_send_receive(#[case] sugared: &str, #[case] hand_written: &str) {
    let s = parse_stripped(sugared);
    let h = parse_stripped(hand_written);
    pretty_assertions::assert_eq!(
        s,
        h,
        "for (z <- x!y(args)) AST should match for (z <- x!?(\"y\", args)) AST"
    );
}

/// Reserved keywords (e.g. `new`) cannot be used as method names.
/// The `var` token in the grammar excludes globally-reserved words,
/// so the parser produces ERROR nodes around the keyword. This test
/// locks the rejection in.
#[test]
fn reserved_keyword_as_method_name_is_rejected() {
    let parser = RholangParser::new();
    let result = parser.parse("new x in { x!new(1). }");
    // Use the Debug output as the assertion vehicle -- the Validated
    // variant tag distinguishes Good/Fail cleanly.
    let dbg = format!("{result:#?}");
    assert!(
        dbg.starts_with("Fail("),
        "expected Fail outcome, got: {dbg}"
    );
    assert!(
        dbg.contains("UnexpectedVar") || dbg.contains("SyntaxError"),
        "expected an UnexpectedVar/SyntaxError on `new`, got: {dbg}"
    );
}

/// `!=` (neq) must NOT be parsed as a send_method whose method name
/// starts with `=`. The grammar distinguishes by what follows the
/// `!`: a var (method name) means send_method; otherwise it's an
/// operator. This guard catches any future grammar churn that
/// would conflate them.
#[test]
fn neq_operator_does_not_collide_with_send_method() {
    let parser = RholangParser::new();
    let dbg = format!(
        "{:#?}",
        parser.parse("new x, y in { if (x != y) { @0!(1) } }")
    );
    assert!(dbg.starts_with("Good("), "expected Good outcome: {dbg}");
    assert!(
        dbg.contains("neq:") || dbg.contains("Neq"),
        "expected a `neq` node in the AST: {dbg}"
    );
    assert!(
        !dbg.contains("SendMethod"),
        "x != y should not parse as send_method"
    );
}

/// Agent block desugaring equivalence: each pair (`<name>.rho`,
/// `<name>_desugared.rho`) in `tests/corpus/` is parsed and compared
/// modulo source spans. The desugared form spells out the FIP
/// expansion (`for + new this, private + match dispatch + bundle+`)
/// using only constructs that exist before this PR; if the visitor's
/// desugaring drifts from the FIP intent, these tests catch it.
///
/// Snapshot files for both halves already exist (`golden.rs` picks
/// them up automatically); this assertion is the programmatic check
/// that a reviewer doesn't have to verify by eye.
#[rstest]
#[case::minimal("agent_minimal", "agent_minimal_desugared")]
#[case::with_methods("agent_with_methods", "agent_with_methods_desugared")]
#[case::with_private("agent_with_private", "agent_with_private_desugared")]
#[case::private_state("agent_private_state", "agent_private_state_desugared")]
fn agent_block_desugars_to_handwritten_form(
    #[case] sugared_basename: &str,
    #[case] desugared_basename: &str,
) {
    let s = parse_corpus_stripped(sugared_basename);
    let d = parse_corpus_stripped(desugared_basename);
    pretty_assertions::assert_eq!(
        s,
        d,
        "agent block AST should match its hand-written desugared form ({sugared_basename} vs {desugared_basename})"
    );
}

/// Sanity guard for the position-stripping helper itself: two parses
/// of the SAME source should be string-equal both before and after
/// stripping. If this fails, the strip routine is buggy and the other
/// tests in this file would silently lose discriminating power.
#[test]
fn strip_positions_is_idempotent_for_same_source() {
    let src = r#"new x in { x!?("y", 1, 2). }"#;
    let a = parse_stripped(src);
    let b = parse_stripped(src);
    assert_eq!(a, b);
    // And the stripped output should NOT contain span: / pos: opener lines.
    assert!(
        !a.contains("span: SourceSpan {"),
        "strip_positions left a span: opener"
    );
    assert!(
        !a.contains("pos: SourcePos {"),
        "strip_positions left a pos: opener"
    );
}
