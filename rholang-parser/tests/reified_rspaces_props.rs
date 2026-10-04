//! Property tests for the Reified RSpaces syntax (invariants G1–G6).
//!
//! The parser has no AST printer, so every property generates a model,
//! renders source text from it, parses the text, and compares the AST with
//! the model.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::test_runner::RngSeed;
use rholang_parser::RholangParser;
use rholang_parser::ast::{AnnProc, Bind, Collection, Hyperparam, Name, Proc, Source, Var};
use rholang_parser::parser::errors::ParsingError;
use rholang_parser::{SourcePos, SourceSpan};
use validated::Validated;

fn config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_seed: RngSeed::Fixed(0x5EED_2026_1004),
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

/// The keyword strings of the grammar plus `use` and `free`; generated
/// identifiers avoid all of them.
const KEYWORDS: &[&str] = &[
    "Bool",
    "ByteArray",
    "Int",
    "Nil",
    "Set",
    "String",
    "Uri",
    "agent",
    "and",
    "bundle",
    "bundle0",
    "catch",
    "constructor",
    "contract",
    "default",
    "else",
    "false",
    "for",
    "free",
    "if",
    "in",
    "let",
    "match",
    "matches",
    "method",
    "new",
    "not",
    "or",
    "private",
    "select",
    "true",
    "try",
    "use",
    "where",
];

fn identifier() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,5}".prop_filter("a keyword is not an identifier", |name| {
        !KEYWORDS.contains(&name.as_str())
    })
}

/// Named parameters with distinct keys, in generation order.
fn named_params(max: usize) -> impl Strategy<Value = Vec<(String, i64)>> {
    prop::collection::vec((identifier(), 0i64..1000), 0..=max).prop_map(|pairs| {
        let mut seen = BTreeSet::new();
        pairs
            .into_iter()
            .filter(|(key, _)| seen.insert(key.clone()))
            .collect()
    })
}

fn render_values(values: &[i64]) -> String {
    values
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_params(positional: &[i64], named: &[(String, i64)]) -> String {
    positional
        .iter()
        .map(i64::to_string)
        .chain(named.iter().map(|(key, value)| format!("{key} = {value}")))
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_one<'a>(parser: &'a RholangParser<'a>, source: &'a str) -> AnnProc<'a> {
    match parser.parse(source) {
        Validated::Good(procs) => {
            assert_eq!(procs.len(), 1, "{source} must be one process: {procs:?}");
            procs[0]
        }
        Validated::Fail(failure) => panic!("{source} must parse: {failure:?}"),
    }
}

fn long_value(proc: &AnnProc) -> i64 {
    match proc.proc {
        Proc::LongLiteral(value) => *value,
        other => panic!("expected an integer literal, got {other:?}"),
    }
}

fn first_bind<'a>(proc: &'a AnnProc<'a>) -> &'a Bind<'a> {
    match proc.proc {
        Proc::ForComprehension { receipts, .. } => &receipts[0].binds[0],
        other => panic!("expected a receive, got {other:?}"),
    }
}

/// Checks a hyperparameter list against the model: the positional values
/// in order, then the named ones in order.
fn assert_params(params: &[Hyperparam], positional: &[i64], named: &[(String, i64)]) {
    assert_eq!(params.len(), positional.len() + named.len());
    for (param, expected) in params.iter().zip(positional) {
        assert!(
            matches!(param, Hyperparam::Positional(value) if long_value(value) == *expected),
            "{param:?} must be the positional value {expected}"
        );
    }
    for (param, (key, expected)) in params[positional.len()..].iter().zip(named) {
        assert!(
            matches!(param, Hyperparam::Named { key: actual, value } if actual.name == key && long_value(value) == *expected),
            "{param:?} must be the named value {key} = {expected}"
        );
    }
}

#[derive(Debug, Clone, Copy)]
enum SendForm {
    Single,
    Multiple,
    Sync,
    Method,
    SendReceiveSource,
    MethodSource,
}

#[derive(Debug, Clone, Copy)]
enum QueryForm {
    /// `ch ~ e`
    Bare,
    /// `ch ~ (e1, …; k = v…)`
    Group,
    /// `ch ~ (e1, …)`, no `;`
    Parenthesized,
}

#[derive(Debug, Clone)]
enum ParamEntry {
    Positional(i64),
    Named(&'static str, i64),
    NonIdentifierKey(i64, i64),
}

fn one_line_span(start: usize, end: usize) -> SourceSpan {
    SourceSpan {
        start: SourcePos {
            line: 1,
            col: start,
        },
        end: SourcePos { line: 1, col: end },
    }
}

/// Identifiers that look like the desugaring's fresh names.
const FRESH_LOOKALIKES: &[&str] = &["__ok", "__rest", "__ok0", "__rest0", "__ok1", "__rest1"];

/// Reified RSpaces positions inside a try body; `{n}` is the identifier.
const REIFIED_POSITIONS: &[&str] = &[
    "x!(1; k = {n})",
    "x!(1; {n} = 1)",
    "for (y <- ch!?(1; k = {n})) { Nil }",
    "for (y <- ch ~ {n}) { Nil }",
    "for (y <- ch ~ (1; k = {n})) { Nil }",
    "new z : {n} in { Nil }",
    "new z : @{n} in { Nil }",
    "use {n} { Nil }",
    "x!(free Th({n}))",
    "x!(free {n}(1))",
];

/// Contexts in which `use` and `free` stay identifiers; `@@` is the word.
const IDENTIFIER_CONTEXTS: &[&str] = &[
    "x!(@@{sep}+ {n})",
    "match e { 1 => @@{sep}{n} => Nil }",
    "match e { 1 => @@{sep}_ => Nil }",
    "match e { 1 => @@{sep}{n}!(1) => Nil }",
    "@@!({n})",
    "out!(@@, {n})",
    "new @@, {n} in { @@!({n}) }",
    "{ @@{sep}| {n} }",
    "if ({n}) @@{sep}else Nil",
    "(@@{sep}and {n})",
    "let {n} = @@{sep}in { {n} }",
    "contract @@({n}) = { Nil }",
    "for (@@ <- {n}) { Nil }",
];

const SEPARATORS: &[&str] = &[" ", "  ", "\n", " /* c */ ", " // c\n "];

/// Same-length stand-ins that are ordinary identifiers.
fn stand_in(word: &str) -> &'static str {
    match word {
        "use" => "uze",
        "free" => "frex",
        other => panic!("no stand-in for {other}"),
    }
}

proptest! {
    #![proptest_config(config())]

    /// G2: a send, sync send, method send or source with a `;` section keeps
    /// its data and its hyperparameters, positional before named, in source
    /// order.
    #[test]
    fn prop_hyper_inputs_roundtrip(
        form in prop_oneof![
            Just(SendForm::Single),
            Just(SendForm::Multiple),
            Just(SendForm::Sync),
            Just(SendForm::Method),
            Just(SendForm::SendReceiveSource),
            Just(SendForm::MethodSource),
        ],
        data in prop::collection::vec(0i64..1000, 0..4),
        positional in prop::collection::vec(0i64..1000, 0..3),
        named in named_params(3),
    ) {
        let args = format!("({}; {})", render_values(&data), render_params(&positional, &named));
        let source = match form {
            SendForm::Single => format!("ch!{args}"),
            SendForm::Multiple => format!("ch!!{args}"),
            SendForm::Sync => format!("ch!?{args}."),
            SendForm::Method => format!("ch!m{args}."),
            SendForm::SendReceiveSource => format!("for (y <- ch!?{args}) {{ Nil }}"),
            SendForm::MethodSource => format!("for (y <- ch!m{args}) {{ Nil }}"),
        };
        let parser = RholangParser::new();
        let proc = parse_one(&parser, &source);
        let (inputs, params) = match (form, proc.proc) {
            (SendForm::Single | SendForm::Multiple, Proc::Send { inputs, params, .. }) => {
                (inputs.as_slice(), params.as_ref())
            }
            (SendForm::Sync | SendForm::Method, Proc::SendSync { inputs, params, .. }) => {
                (inputs.as_slice(), params.as_ref())
            }
            (SendForm::SendReceiveSource | SendForm::MethodSource, _) => match first_bind(&proc) {
                Bind::Linear { rhs: Source::SendReceive { inputs, params, .. }, .. } => {
                    (inputs.as_slice(), params.as_deref())
                }
                other => panic!("{source}: expected a send-receive source, got {other:?}"),
            },
            (_, other) => panic!("{source}: unexpected process {other:?}"),
        };
        let data_inputs = match form {
            SendForm::Method | SendForm::MethodSource => {
                prop_assert!(matches!(inputs[0].proc, Proc::StringLiteral("m")));
                &inputs[1..]
            }
            SendForm::Single | SendForm::Multiple | SendForm::Sync | SendForm::SendReceiveSource => inputs,
        };
        prop_assert_eq!(data_inputs.iter().map(long_value).collect::<Vec<_>>(), data);
        let params = params.expect("a `;` section gives Some");
        assert_params(params, &positional, &named);
    }

    /// G1: a query payload is a list exactly when the group has a `;`;
    /// otherwise the query is one process (a tuple for `(a, b)`).
    #[test]
    fn prop_query_group_iff_semicolon(
        (form, payload, named) in prop_oneof![
            (0i64..1000).prop_map(|value| (QueryForm::Bare, vec![value], Vec::new())),
            (prop::collection::vec(0i64..1000, 0..4), named_params(3))
                .prop_filter("the empty group is malformed", |(payload, named)| {
                    !payload.is_empty() || !named.is_empty()
                })
                .prop_map(|(payload, named)| (QueryForm::Group, payload, named)),
            prop::collection::vec(0i64..1000, 1..4)
                .prop_map(|payload| (QueryForm::Parenthesized, payload, Vec::new())),
        ],
    ) {
        let query = match form {
            QueryForm::Bare => payload[0].to_string(),
            QueryForm::Group => format!("({}; {})", render_values(&payload), render_params(&[], &named)),
            QueryForm::Parenthesized => format!("({})", render_values(&payload)),
        };
        let source = format!("for (y <- ch ~ {query}) {{ Nil }}");
        let parser = RholangParser::new();
        let proc = parse_one(&parser, &source);
        let spec = first_bind(&proc).query_spec().expect("the bind has a query");
        match form {
            QueryForm::Group => {
                prop_assert_eq!(spec.payload.iter().map(long_value).collect::<Vec<_>>(), payload);
                match &spec.params {
                    Some(params) => {
                        prop_assert!(!named.is_empty());
                        assert_params(params, &[], &named);
                    }
                    None => prop_assert!(named.is_empty()),
                }
            }
            QueryForm::Bare | QueryForm::Parenthesized => {
                prop_assert!(spec.params.is_none());
                prop_assert_eq!(spec.payload.len(), 1);
                match (payload.as_slice(), spec.payload[0].proc) {
                    ([value], _) => prop_assert_eq!(long_value(&spec.payload[0]), *value),
                    (values, Proc::Collection(Collection::Tuple(items))) => {
                        prop_assert_eq!(items.iter().map(long_value).collect::<Vec<_>>(), values.to_vec());
                    }
                    (_, other) => prop_assert!(false, "{source}: expected a tuple, got {other:?}"),
                }
            }
        }
    }

    /// G3: every hyperparameter error is reported with its exact kind and
    /// span, in source order, and a repeated key points at the first
    /// occurrence of the key.
    #[test]
    fn prop_param_errors_exact_and_first(
        entries in prop::collection::vec(
            prop_oneof![
                (0i64..100).prop_map(ParamEntry::Positional),
                (prop::sample::select(&["a", "b", "c"][..]), 0i64..100)
                    .prop_map(|(key, value)| ParamEntry::Named(key, value)),
                (0i64..100, 0i64..100).prop_map(|(key, value)| ParamEntry::NonIdentifierKey(key, value)),
            ],
            1..6,
        ),
    ) {
        // Render `x!(0; …)` and compute the expected errors with their columns.
        let mut source = String::from("x!(0; ");
        let mut expected = Vec::new();
        let mut first_spans: Vec<(&str, SourceSpan)> = Vec::new();
        let mut named_seen = false;
        for (index, entry) in entries.iter().enumerate() {
            if index > 0 {
                source.push_str(", ");
            }
            let start = source.len() + 1;
            match entry {
                ParamEntry::Positional(value) => {
                    source.push_str(&value.to_string());
                    if named_seen {
                        expected.push((ParsingError::PositionalAfterNamed, one_line_span(start, source.len() + 1)));
                    }
                }
                ParamEntry::Named(key, value) => {
                    named_seen = true;
                    source.push_str(key);
                    let key_span = one_line_span(start, source.len() + 1);
                    match first_spans.iter().find(|(seen, _)| seen == key) {
                        Some((_, first)) => expected.push((
                            ParsingError::DuplicateParamKey { key: (*key).to_owned(), first: *first },
                            key_span,
                        )),
                        None => first_spans.push((key, key_span)),
                    }
                    source.push_str(&format!(" = {value}"));
                }
                ParamEntry::NonIdentifierKey(key, value) => {
                    named_seen = true;
                    source.push_str(&key.to_string());
                    expected.push((ParsingError::NonIdentifierParamKey, one_line_span(start, source.len() + 1)));
                    source.push_str(&format!(" = {value}"));
                }
            }
        }
        source.push(')');

        let parser = RholangParser::new();
        match parser.parse(&source) {
            Validated::Good(_) => prop_assert!(expected.is_empty(), "{source}: expected {expected:?}"),
            Validated::Fail(failures) => {
                let actual: Vec<_> = failures
                    .iter()
                    .flat_map(|failure| failure.errors.iter())
                    .map(|error| (error.error.clone(), error.span))
                    .collect();
                prop_assert_eq!(actual, expected, "{}", source);
            }
        }
    }

    /// G4: `use` and `free` are keywords exactly before a space name and
    /// `{`, or a theory name and `(`. Elsewhere they parse as an ordinary
    /// identifier of the same length would.
    #[test]
    fn prop_use_free_keyword_identifier_duality(
        context in prop::sample::select(IDENTIFIER_CONTEXTS),
        word in prop::sample::select(&["use", "free"][..]),
        name in identifier().prop_filter("a stand-in is not a free name", |name| {
            name != "uze" && name != "frex"
        }),
        separator in prop::sample::select(SEPARATORS),
    ) {
        let render = |word: &str| {
            context.replace("{sep}", separator).replace("{n}", &name).replace("@@", word)
        };
        let parser = RholangParser::new();
        let keyword_source = render(word);
        let stand_in_source = render(stand_in(word));
        let keyword_tree = format!("{:#?}", parser.parse(&keyword_source));
        let stand_in_tree = format!("{:#?}", parser.parse(&stand_in_source))
            .replace(&format!("\"{}\"", stand_in(word)), &format!("\"{word}\""));
        prop_assert_eq!(keyword_tree, stand_in_tree, "{} vs {}", keyword_source, stand_in_source);

        // The keyword direction: a space name and `{`, a theory name and `(`.
        let use_source = format!("use{separator}{name}{separator}{{ Nil }}");
        let proc = parse_one(&parser, &use_source);
        prop_assert!(
            matches!(proc.proc, Proc::UseBlock { space: Name::NameVar(Var::Id(id)), .. } if id.name == name),
            "{}: {:?}", use_source, proc
        );
        let free_source = format!("x!(free{separator}{name}{separator}(1))");
        let proc = parse_one(&parser, &free_source);
        prop_assert!(
            matches!(proc.proc, Proc::Send { inputs, .. } if matches!(inputs[0].proc, Proc::FreeTheory { name: theory, .. } if theory.name == name)),
            "{}: {:?}", free_source, proc
        );
        // The stand-ins never form the keyword constructs.
        for source in [
            format!("uze{separator}{name}{separator}{{ Nil }}"),
            format!("x!(frex{separator}{name}{separator}(1))"),
        ] {
            let parser = RholangParser::new();
            let keyword_free = match parser.parse(&source) {
                Validated::Good(procs) => procs.iter().flat_map(|proc| proc.iter_preorder_dfs()).all(|proc| {
                    !matches!(proc.proc, Proc::UseBlock { .. } | Proc::FreeTheory { .. })
                }),
                Validated::Fail(_) => true,
            };
            prop_assert!(keyword_free, "{}", source);
        }
    }

    /// G5: a bare float is exactly the f64 float of the same text.
    #[test]
    fn prop_bare_float_exact_f64(
        sign in prop::sample::select(&["", "-"][..]),
        int_part in "[0-9]{0,6}",
        fraction in proptest::option::of("[0-9]{1,6}"),
        exponent in proptest::option::of((
            prop::sample::select(&["e", "E"][..]),
            prop::sample::select(&["", "+", "-"][..]),
            "[0-9]{1,3}",
        )),
    ) {
        // A bare float has a fraction or an exponent, and an integer part
        // unless it has a fraction (`.5`).
        prop_assume!(fraction.is_some() || (exponent.is_some() && !int_part.is_empty()));
        let mut text = format!("{sign}{int_part}");
        if let Some(fraction) = &fraction {
            text.push('.');
            text.push_str(fraction);
        }
        if let Some((marker, exponent_sign, digits)) = &exponent {
            text.push_str(marker);
            text.push_str(exponent_sign);
            text.push_str(digits);
        }
        for source in [format!("x!({text})"), format!("x!({text}f64)")] {
            let parser = RholangParser::new();
            let proc = parse_one(&parser, &source);
            let Proc::Send { inputs, .. } = proc.proc else {
                panic!("{source}: expected a send");
            };
            prop_assert!(
                matches!(inputs.as_slice(), [AnnProc { proc: Proc::FloatLiteral { value, bits: 64 }, .. }] if *value == text),
                "{}: {:?}", source, inputs
            );
        }
    }

    /// G6: the try/catch desugaring's fresh names avoid identifiers that
    /// appear only in Reified RSpaces positions.
    #[test]
    fn prop_desugar_hygiene_avoids_reified_identifiers(
        placements in prop::collection::vec(
            (prop::sample::select(REIFIED_POSITIONS), prop::sample::select(FRESH_LOOKALIKES)),
            1..5,
        ),
    ) {
        let body = placements
            .iter()
            .map(|(position, name)| position.replace("{n}", name))
            .collect::<Vec<_>>()
            .join(" | ");
        let source = format!("try @r <- ch {{ {body} }} catch @e {{ Nil }}");
        let parser = RholangParser::new();
        let proc = parse_one(&parser, &source);
        let Bind::Linear { lhs, .. } = first_bind(&proc) else {
            panic!("{source}: expected the desugared linear bind");
        };
        let Some(Name::Quote(AnnProc { proc: Proc::Collection(Collection::List { elements, remainder: Some(Var::Id(rest)) }), .. })) = lhs.names.first() else {
            panic!("{source}: expected the `@[ok, ...rest]` pattern, got {lhs:?}");
        };
        let [AnnProc { proc: Proc::ProcVar(Var::Id(ok)), .. }] = elements.as_slice() else {
            panic!("{source}: expected the `ok` binder, got {elements:?}");
        };
        let used: BTreeSet<&str> = placements.iter().map(|(_, name)| *name).collect();
        prop_assert!(!used.contains(ok.name), "{}: `{}` captures a reified identifier", source, ok.name);
        prop_assert!(!used.contains(rest.name), "{}: `{}` captures a reified identifier", source, rest.name);
        prop_assert_ne!(ok.name, rest.name);
    }
}
