use rholang_parser::RholangParser;
use rholang_parser::ast::{
    AnnProc, Bind, Collection, Hyperparam, Name, Proc, Source, SyncSendCont, Var,
};
use rholang_parser::parser::errors::{
    ParsingError, QUERY_EMPTY_GROUP, QUERY_NON_SIMPLE_SOURCE, QUERY_POSITIONAL_PARAM,
};
use rholang_parser::{SourcePos, SourceSpan};
use validated::Validated;

/// A one-line span `line:start - line:end` (1-based, end exclusive).
fn span(line: usize, start: usize, end: usize) -> SourceSpan {
    SourceSpan {
        start: SourcePos { line, col: start },
        end: SourcePos { line, col: end },
    }
}

/// Parses `source`, which must fail, and returns every error with its span.
fn errors_of(source: &str) -> Vec<(ParsingError, SourceSpan)> {
    let parser = RholangParser::new();
    match parser.parse(source) {
        Validated::Good(procs) => panic!("expected a parse failure for {source}: {procs:?}"),
        Validated::Fail(failures) => failures
            .iter()
            .flat_map(|failure| failure.errors.iter())
            .map(|error| (error.error.clone(), error.span))
            .collect(),
    }
}

/// Every process of a parse, depth first, quoted processes included.
fn all_procs<'a>(procs: &'a [AnnProc<'a>]) -> Vec<&'a Proc<'a>> {
    procs
        .iter()
        .flat_map(|proc| proc.iter_preorder_dfs())
        .map(|ann| ann.proc)
        .collect()
}

#[test]
fn use_blocks_keep_nested_space_names_and_body() {
    let parser = RholangParser::new();
    let parsed = parser.parse("use @\"outer\" { use inner { Nil } }");
    let procs = match parsed {
        Validated::Good(procs) => procs,
        Validated::Fail(error) => panic!("parse failed: {error:?}"),
    };

    assert_eq!(procs.len(), 1);
    let Proc::UseBlock { space, proc } = procs[0].proc else {
        panic!("expected outer use block");
    };
    assert!(matches!(space, Name::Quote(_)));
    let Proc::UseBlock { space, proc } = proc.proc else {
        panic!("expected inner use block");
    };
    assert!(matches!(space, Name::NameVar(Var::Id(id)) if id.name == "inner"));
    assert!(matches!(proc.proc, Proc::Nil));
}

#[test]
fn use_is_still_an_identifier_in_context() {
    let parser = RholangParser::new();
    let parsed = parser.parse("contract f(use) = { out!(use) }");
    let procs = match parsed {
        Validated::Good(procs) => procs,
        Validated::Fail(error) => panic!("parse failed: {error:?}"),
    };

    let Proc::Contract { body, .. } = procs[0].proc else {
        panic!("expected contract");
    };
    assert!(
        matches!(body.proc, Proc::Send { inputs, .. } if matches!(inputs[0].proc, Proc::ProcVar(Var::Id(id)) if id.name == "use"))
    );
}

#[test]
fn use_can_be_a_channel_name() {
    let parser = RholangParser::new();
    let parsed = parser.parse("use!(1)");
    let procs = match parsed {
        Validated::Good(procs) => procs,
        Validated::Fail(error) => panic!("parse failed: {error:?}"),
    };

    assert!(
        matches!(procs[0].proc, Proc::Send { channel: Name::NameVar(Var::Id(id)), .. } if id.name == "use")
    );
}

#[test]
fn other_identifiers_cannot_select_a_space() {
    let parser = RholangParser::new();
    assert!(matches!(
        parser.parse("other @\"space\" { Nil }"),
        Validated::Fail(_)
    ));
}

#[test]
fn send_parameters_preserve_order_and_explicit_empty_section() {
    let parser = RholangParser::new();
    let parsed = parser.parse("out!(1; 2, priority=3) | out!(4;)");
    let procs = match parsed {
        Validated::Good(procs) => procs,
        Validated::Fail(error) => panic!("parse failed: {error:?}"),
    };
    let Proc::Par { left, right } = procs[0].proc else {
        panic!("expected parallel sends");
    };
    let Proc::Send {
        inputs,
        params: Some(params),
        ..
    } = left.proc
    else {
        panic!("expected parameter send");
    };
    assert_eq!(inputs.len(), 1);
    assert!(
        matches!(params[0], Hyperparam::Positional(value) if matches!(value.proc, Proc::LongLiteral(2)))
    );
    assert!(
        matches!(params[1], Hyperparam::Named { key, value } if key.name == "priority" && matches!(value.proc, Proc::LongLiteral(3)))
    );
    assert!(matches!(right.proc, Proc::Send { params: Some(params), .. } if params.is_empty()));
}

#[test]
fn free_theory_keeps_name_and_arguments() {
    let parser = RholangParser::new();
    let parsed = parser.parse("free Nat(1, 2) | free!(3)");
    let procs = match parsed {
        Validated::Good(procs) => procs,
        Validated::Fail(error) => panic!("parse failed: {error:?}"),
    };
    let Proc::Par { left, right } = procs[0].proc else {
        panic!("expected parallel terms");
    };
    assert!(
        matches!(left.proc, Proc::FreeTheory { name, args } if name.name == "Nat" && args.len() == 2)
    );
    assert!(
        matches!(right.proc, Proc::Send { channel: Name::NameVar(Var::Id(id)), .. } if id.name == "free")
    );
}

#[test]
fn new_space_annotation_preserves_binder_order() {
    let parser = RholangParser::new();
    let parsed = parser.parse("new room : space, reply in { Nil }");
    let procs = match parsed {
        Validated::Good(procs) => procs,
        Validated::Fail(error) => panic!("parse failed: {error:?}"),
    };
    let Proc::New { decls, .. } = procs[0].proc else {
        panic!("expected new declaration");
    };
    assert_eq!(decls.len(), 2);
    assert_eq!(decls[0].id.name, "room");
    assert!(matches!(decls[0].space, Some(Name::NameVar(Var::Id(id))) if id.name == "space"));
    assert_eq!(decls[1].id.name, "reply");
    assert!(decls[1].space.is_none());
}

#[test]
fn receive_queries_keep_payload_and_named_parameter_order() {
    let parser = RholangParser::new();
    let parsed = parser.parse("for(x <- ch ~ (1, 2; metric=3, limit=4)) { Nil }");
    let procs = match parsed {
        Validated::Good(procs) => procs,
        Validated::Fail(error) => panic!("parse failed: {error:?}"),
    };
    let Proc::ForComprehension { receipts, .. } = procs[0].proc else {
        panic!("expected receive");
    };
    let Some(query) = receipts[0].binds[0].query_spec() else {
        panic!("expected query");
    };
    assert_eq!(query.payload.len(), 2);
    assert!(matches!(query.payload[0].proc, Proc::LongLiteral(1)));
    assert!(matches!(query.payload[1].proc, Proc::LongLiteral(2)));
    let Some(params) = &query.params else {
        panic!("expected query parameters");
    };
    assert!(matches!(&params[0], Hyperparam::Named { key, .. } if key.name == "metric"));
    assert!(matches!(&params[1], Hyperparam::Named { key, .. } if key.name == "limit"));
}

#[test]
fn receive_queries_support_simple_and_empty_grouped_payloads() {
    let parser = RholangParser::new();
    let parsed = parser.parse("for(x <- ch ~ 1) { Nil } | for(y <- ch ~ (; metric=3)) { Nil }");
    let procs = match parsed {
        Validated::Good(procs) => procs,
        Validated::Fail(error) => panic!("parse failed: {error:?}"),
    };
    let Proc::Par { left, right } = procs[0].proc else {
        panic!("expected parallel receives");
    };
    let Proc::ForComprehension {
        receipts: simple, ..
    } = left.proc
    else {
        panic!("expected simple query");
    };
    let Proc::ForComprehension {
        receipts: grouped, ..
    } = right.proc
    else {
        panic!("expected grouped query");
    };
    let simple_query = simple[0].binds[0]
        .query_spec()
        .expect("the simple receive has a query");
    assert_eq!(simple_query.payload.len(), 1);
    assert!(simple_query.params.is_none());
    let query = grouped[0].binds[0]
        .query_spec()
        .expect("the grouped receive has a query");
    assert!(query.payload.is_empty());
    assert!(
        matches!(query.params.as_deref(), Some([Hyperparam::Named { key, .. }]) if key.name == "metric")
    );
}

#[test]
fn malformed_parameter_and_query_groups_fail_to_parse() {
    assert!(matches!(
        errors_of("out!(1; priority=)").as_slice(),
        [(ParsingError::SyntaxError { .. }, error_span)] if *error_span == span(1, 17, 18)
    ));
    assert_eq!(
        errors_of("for(x <- ch ~ (; 1)) { Nil }"),
        vec![(
            ParsingError::MalformedQuerySpec {
                reason: QUERY_POSITIONAL_PARAM
            },
            span(1, 18, 19)
        )]
    );
    assert!(matches!(
        errors_of("for(x <- ch ~ (1; metric=)) { Nil }").as_slice(),
        [(ParsingError::SyntaxError { .. }, error_span)] if *error_span == span(1, 25, 26)
    ));
}

#[test]
fn contextual_keywords_remain_identifiers_outside_their_forms() {
    let parser = RholangParser::new();
    for source in ["use!(1)", "free!(2)"] {
        let parsed = parser.parse(source);
        assert!(matches!(parsed, Validated::Good(_)), "{source}: {parsed:?}");
    }
    let parsed = parser.parse("contract f(use, free) = { @\"out\"!(use, free) }");
    assert!(matches!(parsed, Validated::Good(_)));
}

#[test]
fn parameter_only_sends_keep_empty_data_and_parameter_values() {
    let parser = RholangParser::new();
    let parsed = parser.parse("out!(; priority=1) | out!?(; priority=2).");
    let Validated::Good(procs) = parsed else {
        panic!("parse failed")
    };
    let Proc::Par { left, right } = procs[0].proc else {
        panic!("expected parallel sends")
    };
    assert!(
        matches!(left.proc, Proc::Send { inputs, params: Some(params), .. } if inputs.is_empty() && params.len() == 1)
    );
    assert!(
        matches!(right.proc, Proc::SendSync { inputs, params: Some(params), .. } if inputs.is_empty() && params.len() == 1)
    );
}

#[test]
fn method_and_receive_source_parameters_keep_values() {
    let parser = RholangParser::new();
    let Validated::Good(procs) = parser.parse("@\"svc\"!call(1; priority=2).") else {
        panic!("method send parse failed")
    };
    assert!(
        matches!(procs[0].proc, Proc::SendSync { inputs, params: Some(params), .. } if inputs.len() == 2 && params.len() == 1)
    );

    let Validated::Good(procs) = parser.parse("for(x <- @\"svc\"!?(; priority=1)) { Nil }") else {
        panic!("source parse failed")
    };
    let Proc::ForComprehension { receipts, .. } = procs[0].proc else {
        panic!("expected receive")
    };
    assert!(
        matches!(&receipts[0].binds[0], Bind::Linear { rhs: Source::SendReceive { inputs, params: Some(params), .. }, .. } if inputs.is_empty() && params.len() == 1)
    );

    let Validated::Good(procs) = parser.parse("for(x <- @\"svc\"!call(1; priority=2)) { Nil }")
    else {
        panic!("method source parse failed")
    };
    let Proc::ForComprehension { receipts, .. } = procs[0].proc else {
        panic!("expected receive")
    };
    assert!(
        matches!(&receipts[0].binds[0], Bind::Linear { rhs: Source::SendReceive { inputs, params: Some(params), .. }, .. } if inputs.len() == 2 && params.len() == 1)
    );
}

#[test]
fn malformed_method_and_source_parameter_values_fail_to_parse() {
    let parser = RholangParser::new();
    for source in [
        "@\"svc\"!call(1; priority=).",
        "for(x <- @\"svc\"!?(; priority=)) { Nil }",
        "for(x <- @\"svc\"!call(; priority=)) { Nil }",
    ] {
        assert!(
            matches!(parser.parse(source), Validated::Fail(_)),
            "{source}"
        );
    }
}

#[test]
fn try_source_parameters_survive_desugaring() {
    let parser = RholangParser::new();
    let parsed = parser
        .parse("try @[file] <- @\"svc\"!openFile(\"path\"; priority=2) { Nil } catch @_ { Nil }");
    let Validated::Good(procs) = parsed else {
        panic!("parse failed")
    };
    let Proc::ForComprehension { receipts, .. } = procs[0].proc else {
        panic!("expected receive")
    };
    assert!(
        matches!(&receipts[0].binds[0], Bind::Linear { rhs: Source::SendReceive { params: Some(params), .. }, .. } if params.len() == 1)
    );
}

#[test]
fn send_receive_source_params_populated() {
    let parser = RholangParser::new();
    let Validated::Good(procs) = parser.parse("for (r <- ch!?(d; ttl=60)) { Nil }") else {
        panic!("parse failed")
    };
    let Proc::ForComprehension { receipts, .. } = procs[0].proc else {
        panic!("expected receive")
    };
    let Bind::Linear {
        rhs:
            Source::SendReceive {
                name,
                inputs,
                params: Some(params),
            },
        query_spec: None,
        ..
    } = &receipts[0].binds[0]
    else {
        panic!("expected a send-receive source with parameters")
    };
    assert!(name.is_ident("ch"));
    assert_eq!(inputs.len(), 1);
    assert!(inputs[0].is_ident("d"));
    assert!(
        matches!(params.as_slice(), [Hyperparam::Named { key, value }] if key.name == "ttl" && matches!(value.proc, Proc::LongLiteral(60)))
    );
}

#[test]
fn duplicate_param_key_reports_first_span() {
    // Every repetition points back at the first occurrence.
    assert_eq!(
        errors_of("x!(1; k = 1, j = 2, k = 3, k = 4)"),
        vec![
            (
                ParsingError::DuplicateParamKey {
                    key: "k".to_owned(),
                    first: span(1, 7, 8)
                },
                span(1, 21, 22)
            ),
            (
                ParsingError::DuplicateParamKey {
                    key: "k".to_owned(),
                    first: span(1, 7, 8)
                },
                span(1, 28, 29)
            ),
        ]
    );
}

#[test]
fn positional_param_after_named_is_rejected() {
    assert_eq!(
        errors_of("x!(1; 2, k = 1, 3)"),
        vec![(ParsingError::PositionalAfterNamed, span(1, 17, 18))]
    );
    assert_eq!(
        errors_of("for (y <- ch!?(1; k = 1, 2)) { Nil }"),
        vec![(ParsingError::PositionalAfterNamed, span(1, 26, 27))]
    );
}

#[test]
fn non_identifier_param_key_is_rejected() {
    assert_eq!(
        errors_of("x!(1; 2 = 3)"),
        vec![(ParsingError::NonIdentifierParamKey, span(1, 7, 8))]
    );
    assert_eq!(
        errors_of("x!(1; \"k\" = 3, Nil = 4)"),
        vec![
            (ParsingError::NonIdentifierParamKey, span(1, 7, 10)),
            (ParsingError::NonIdentifierParamKey, span(1, 16, 19)),
        ]
    );
}

#[test]
fn malformed_query_spec_is_rejected() {
    let query_error =
        |reason, error_span| (ParsingError::MalformedQuerySpec { reason }, error_span);
    // (i) a positional query parameter
    assert_eq!(
        errors_of("for (y <- ch ~ (1; k = 2, 3)) { Nil }"),
        vec![query_error(QUERY_POSITIONAL_PARAM, span(1, 27, 28))]
    );
    // (ii) the empty group
    assert_eq!(
        errors_of("for (y <- ch ~ (;)) { Nil }"),
        vec![query_error(QUERY_EMPTY_GROUP, span(1, 16, 19))]
    );
    // (iii) a query on a `!?`, `?!` or `!m` source
    assert_eq!(
        errors_of("for (y <- ch!?(1) ~ 2) { Nil }"),
        vec![query_error(QUERY_NON_SIMPLE_SOURCE, span(1, 19, 22))]
    );
    assert_eq!(
        errors_of("for (y <- ch?! ~ 2) { Nil }"),
        vec![query_error(QUERY_NON_SIMPLE_SOURCE, span(1, 16, 19))]
    );
    assert_eq!(
        errors_of("for (y <- ch!get(1) ~ 2) { Nil }"),
        vec![query_error(QUERY_NON_SIMPLE_SOURCE, span(1, 21, 24))]
    );
}

#[test]
fn tuple_query_without_semicolon_is_a_single_payload() {
    let parser = RholangParser::new();
    let Validated::Good(procs) =
        parser.parse("for (y <- ch ~ (1, 2)) { Nil } | for (z <- ch ~ (1, 2;)) { Nil }")
    else {
        panic!("parse failed")
    };
    let Proc::Par { left, right } = procs[0].proc else {
        panic!("expected parallel receives")
    };
    let Proc::ForComprehension { receipts, .. } = left.proc else {
        panic!("expected receive")
    };
    let tuple_query = receipts[0].binds[0]
        .query_spec()
        .expect("the receive has a query");
    assert!(
        matches!(tuple_query.payload.as_slice(), [element] if matches!(element.proc, Proc::Collection(Collection::Tuple(items)) if items.len() == 2))
    );
    assert!(tuple_query.params.is_none());
    let Proc::ForComprehension { receipts, .. } = right.proc else {
        panic!("expected receive")
    };
    let list_query = receipts[0].binds[0]
        .query_spec()
        .expect("the receive has a query");
    assert_eq!(list_query.payload.len(), 2);
    assert!(list_query.params.is_none());
}

#[test]
fn query_terminates_before_signed_bind() {
    let parser = RholangParser::new();
    let Validated::Good(procs) = parser.parse("for (y <- ch ~ q & {% z <- c %}[s]) { Nil }") else {
        panic!("parse failed")
    };
    let Proc::ForComprehension { receipts, .. } = procs[0].proc else {
        panic!("expected receive")
    };
    let binds = &receipts[0].binds;
    assert_eq!(binds.len(), 2);
    let query = binds[0].query_spec().expect("the first bind has a query");
    assert!(matches!(query.payload.as_slice(), [payload] if payload.is_ident("q")));
    assert!(
        matches!(&binds[1], Bind::Signed { rhs: Source::Simple { name }, .. } if name.is_ident("c"))
    );
}

#[test]
fn new_space_annotation_accepts_quoted_name() {
    let parser = RholangParser::new();
    let Validated::Good(procs) = parser.parse("new x : @\"s\", y : @{*z}, w in { Nil }") else {
        panic!("parse failed")
    };
    let Proc::New { decls, .. } = procs[0].proc else {
        panic!("expected new declaration")
    };
    assert!(
        matches!(decls[0].space, Some(Name::Quote(quoted)) if matches!(quoted.proc, Proc::StringLiteral("s")))
    );
    assert!(
        matches!(decls[1].space, Some(Name::Quote(quoted)) if matches!(quoted.proc, Proc::Eval { name } if name.is_ident("z")))
    );
    assert!(decls[2].space.is_none());

    // A URI binder cannot carry an annotation, in either order.
    for source in [
        "new x : s(`rho:io:stdout`) in { Nil }",
        "new x(`rho:io:stdout`) : s in { Nil }",
    ] {
        assert!(
            matches!(parser.parse(source), Validated::Fail(_)),
            "{source}"
        );
    }
}

#[test]
fn sync_send_dot_then_digit_is_unchanged() {
    // As at 02cef80: after `x!?(a)` only the continuations `.` and `;` are
    // valid, so `.5` is the empty continuation and then the integer `5`.
    let parser = RholangParser::new();
    let Validated::Good(procs) = parser.parse("x!?(a).5") else {
        panic!("parse failed")
    };
    assert_eq!(procs.len(), 2);
    assert!(matches!(
        procs[0].proc,
        Proc::SendSync {
            inputs,
            params: None,
            cont: SyncSendCont::Empty,
            ..
        } if inputs.len() == 1
    ));
    assert!(matches!(procs[1].proc, Proc::LongLiteral(5)));
}

#[test]
fn use_and_free_stay_identifiers_where_02cef80_accepts_them() {
    // Every source parses at 02cef80, so it keeps its identifier reading: a
    // keyword needs a space or theory name and then `{` or `(`. The golden
    // corpus file `use_free_keywords_as_identifiers.rho` pins the exact trees.
    for source in [
        "x!(use + 1)",
        "match free { _ => Nil }",
        "if (c) free else Nil",
        "use and true",
        "use!(1)",
        "out!(use)",
        "free | Nil",
        "new use, free in { use!(free) }",
        "x!(use /* comment */ + 1)",
        "x!(free // comment\n + 1)",
        "let x = use in { x }",
        "match e { 1 => use _ => Nil }",
        "match e { 1 => use x => Nil }",
        "match e { 1 => use @x!(1) => Nil }",
        "match e { 1 => free x => Nil }",
        "match e { 1 => free Set(1) => Nil }",
        "match e { 1 => use bundle0 { Nil } => Nil }",
        "match e { 1 => use not { Nil } => Nil }",
        "contract f(use, free) = { use!(free) }",
        // The 02cef80 witness of every excluded word (grammar.js). The
        // `select` witnesses are tree-sitter corpus cases only: the AST
        // builder does not implement `select`, at 02cef80 or now.
        "use and { Nil }",
        "use or { Nil }",
        "use matches { Nil }",
        "if (c) use else { Nil }",
        "match e { use where { true } => Nil }",
        "match e { 1 => use bundle { Nil } => Nil }",
        "match e { 1 => use match { x } { _ => Nil } => Nil }",
        "match e { 1 => use not { true } => Nil }",
        "free and (true)",
        "free or (true)",
        "free matches (true)",
        "if (c) free else (Nil)",
        "let x = free in (x)",
        "match e { free where (true) => Nil }",
        "match e { 1 => free if (c) Nil => Nil }",
        "match e { 1 => free for (x <- c) { Nil } => Nil }",
        "match e { 1 => free not (true) => Nil }",
        "match e { 1 => free match (x) { _ => Nil } => Nil }",
        "match e { 1 => use @bundle { Nil }!(1) => Nil }",
        "match e { 1 => use @bundle0 { Nil }!(1) => Nil }",
        "match e { 1 => use @match { x } { _ => Nil }!(1) => Nil }",
        "match e { 1 => use @not { true }!(1) => Nil }",
    ] {
        let parser = RholangParser::new();
        let procs = match parser.parse(source) {
            Validated::Good(procs) => procs,
            Validated::Fail(error) => panic!("{source}: {error:?}"),
        };
        assert!(
            all_procs(&procs)
                .iter()
                .all(|proc| !matches!(proc, Proc::UseBlock { .. } | Proc::FreeTheory { .. })),
            "{source} must not contain a keyword form: {procs:?}"
        );
        let identifiers = procs
            .iter()
            .flat_map(|proc| proc.iter_vars())
            .filter(|var| var.is_ident("use") || var.is_ident("free"))
            .count();
        assert!(
            identifiers > 0,
            "{source} must keep `use` or `free` as an identifier"
        );
    }
}

#[test]
fn use_and_free_are_keywords_before_a_space_or_theory() {
    for (source, space) in [
        ("use s { Nil }", "s"),
        ("use /* c */ s { Nil }", "s"),
        ("use // c\n s\n { Nil }", "s"),
        ("use _s { Nil }", "_s"),
        ("use catch { Nil }", "catch"),
        // Keywords outside the `use W {` table name a space.
        ("use Int { Nil }", "Int"),
        ("use Set { Nil }", "Set"),
        ("use Nil { Nil }", "Nil"),
    ] {
        let parser = RholangParser::new();
        let Validated::Good(procs) = parser.parse(source) else {
            panic!("{source}: parse failed")
        };
        assert!(
            matches!(procs.as_slice(), [AnnProc { proc: Proc::UseBlock { space: Name::NameVar(Var::Id(id)), .. }, .. }] if id.name == space),
            "{source}: {procs:?}"
        );
    }
    for source in [
        "use @x { Nil }",
        "use @\"s\" { Nil }",
        "use @{ x | y } { Nil }",
        "use @(1, 2) { Nil }",
        "use @[1] { Nil }",
        "use @`rho:s` { Nil }",
        "use @1 { Nil }",
        "use _ { Nil }",
        "use @in { Nil }",
        "use @Nil { Nil }",
    ] {
        let parser = RholangParser::new();
        let Validated::Good(procs) = parser.parse(source) else {
            panic!("{source}: parse failed")
        };
        assert!(
            matches!(
                procs.as_slice(),
                [AnnProc {
                    proc: Proc::UseBlock { .. },
                    ..
                }]
            ),
            "{source}: {procs:?}"
        );
    }
    for source in [
        "free Nat()",
        "free Nat (1, 2)",
        "free /* c */ _Th(x)",
        "free bundle0()",
    ] {
        let parser = RholangParser::new();
        let Validated::Good(procs) = parser.parse(source) else {
            panic!("{source}: parse failed")
        };
        assert!(
            matches!(
                procs.as_slice(),
                [AnnProc {
                    proc: Proc::FreeTheory { .. },
                    ..
                }]
            ),
            "{source}: {procs:?}"
        );
    }
}

#[test]
fn free_theory_accepts_builtin_theory_names() {
    // The builtin theories, and the other type words: a type word lexes as
    // `var` in the theory-name position.
    for theory in ["Nat", "Int", "String", "Bool", "Any", "Uri", "ByteArray"] {
        let source = format!("free {theory}()");
        let parser = RholangParser::new();
        let procs = match parser.parse(&source) {
            Validated::Good(procs) => procs,
            Validated::Fail(error) => panic!("{source}: {error:?}"),
        };
        assert!(
            matches!(procs.as_slice(), [AnnProc { proc: Proc::FreeTheory { name, args }, .. }] if name.name == theory && args.is_empty()),
            "{source}: {procs:?}"
        );
    }
    let parser = RholangParser::new();
    let Validated::Good(procs) = parser.parse("free Int()") else {
        panic!("free Int() must parse")
    };
    let Proc::FreeTheory { name, .. } = procs[0].proc else {
        panic!("free Int() must be a theory reference: {procs:?}")
    };
    assert_eq!(name.name, "Int");
}
