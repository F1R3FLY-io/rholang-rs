use test_macros::test_rholang_code;

use crate::match_proc;

use super::{
    BinderId, BinderKind, ErrorKind, FactPass, ProcRef, ResolverPass, SemanticDb, VarBinding,
};

use rholang_parser::ast;

#[test_rholang_code(
    r#"
    new anyone, rtn in {
      for (@map <- rtn) {
        @map.get("auction_end")!([*anyone])
      }
    }"#
)]
fn test_scope_nested<'test>(tree: ProcRef<'test>, db: &'test mut SemanticDb<'test>) {
    let root = db[tree];
    let resolver = ResolverPass::new(root);
    resolver.run(db);

    let root_scope = expect::scope(db, root, 2);
    let (root_binders, inner_scope) = match_proc!(tree.proc, ast::Proc::New { proc: inner_for, decls } => {
        let root_binders: Vec<BinderId> = expect::name_decls(db, decls, root_scope).collect();
        let inner_scope = expect::scope(db, inner_for, 1);
        (root_binders, inner_scope)
    });

    let free: Vec<BinderId> =
        expect::free(db, vec![("map", BinderKind::Proc)], inner_scope).collect();

    expect::captures(&root_binders, inner_scope);
    expect::bound(
        db,
        &vec![
            VarBinding::Free { index: 0 },      // @map in for
            VarBinding::Bound(root_binders[1]), // rtn in for
            VarBinding::Bound(free[0]),         // map in map.get("auction_end")
            VarBinding::Bound(root_binders[0]), // anyone in [*anyone]
        ],
    );

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"
new blockData(`rho:block:data`), retCh, stdout(`rho:io:stdout`) in {
  blockData!(*retCh) |
  for(@blockNumber, @timestamp, @sender <- retCh) {
      stdout!({"block number": blockNumber}) |
      stdout!({"block time": timestamp})|
      stdout!({"block sender": sender})
  }
}"#
)]
fn test_pattern_many_names<'test>(tree: ProcRef<'test>, db: &'test mut SemanticDb<'test>) {
    let root = db[tree];
    let resolver = ResolverPass::new(root);
    resolver.run(db);

    let root_scope = expect::scope(db, root, 3);
    let (root_binders, inner_scope) = match_proc!(tree.proc, ast::Proc::New {
        proc:
            ast::AnnProc {
                proc:
                    ast::Proc::Par {
                        left: _,
                        right: inner_for,
                    },
                ..
            },
        decls,
    } => {
        let root_binders: Vec<BinderId> = expect::name_decls(db, decls, root_scope).collect();
        let inner_scope = expect::scope(db, inner_for, 3);
        (root_binders, inner_scope)
    });

    let inner_binders: Vec<BinderId> = expect::free(
        db,
        vec![
            ("blockNumber", BinderKind::Proc),
            ("timestamp", BinderKind::Proc),
            ("sender", BinderKind::Proc),
        ],
        inner_scope,
    )
    .collect();

    expect::captures(&root_binders[1..], inner_scope);
    expect::bound(
        db,
        &vec![
            VarBinding::Bound(root_binders[0]), // blockData in blockData!(*retCh)
            VarBinding::Bound(root_binders[1]), // retCh in blockData!(*retCh)
            VarBinding::Free { index: 0 },      // @blockNumber in for
            VarBinding::Free { index: 1 },      // @timestamp in for
            VarBinding::Free { index: 2 },      // @sender in for
            VarBinding::Bound(root_binders[1]), // retCh in for
            VarBinding::Bound(root_binders[2]), // stdout in for body
            VarBinding::Bound(inner_binders[0]), // blockNumber in for body
            VarBinding::Bound(root_binders[2]), // stdout in for body
            VarBinding::Bound(inner_binders[1]), // timestamp in for body
            VarBinding::Bound(root_binders[2]), // stdout in for body
            VarBinding::Bound(inner_binders[2]), // sender in for body
        ],
    );

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"new retCh, PoSCh, rl(`rho:registry:lookup`), stdout(`rho:io:stdout`) in {
  stdout!("About to lookup pos contract...") |
  rl!(`rho:rchain:pos`, *PoSCh) |
  for(@(_, PoS) <- PoSCh) {
    stdout!("About to bond...") |
    new deployerId(`rho:rchain:deployerId`) in {
      @PoS!("bond", *deployerId, 100, *retCh) |
      for ( @(true, message) <- retCh) {
        stdout!("Successfully bonded!")
      }
    }
  }
}"#
)]
fn test_scope_deeply_nested<'test>(tree: ProcRef<'test>, db: &'test mut SemanticDb<'test>) {
    let root = db[tree];
    let resolver = ResolverPass::new(root);
    resolver.run(db);

    let root_scope = expect::scope(db, root, 4);

    let topmost_retch = expect::binder(db, "retCh", root_scope);
    let topmost_stdout = expect::binder(db, "stdout", root_scope);

    // find first inner for comprehension
    let first_inner_for_node = expect::node(db, expect::for_with_channel_match("PoSCh"));
    let first_for_scope = expect::scope(db, db[first_inner_for_node], 1);
    let pos_in_for = expect::free(db, vec![("PoS", BinderKind::Proc)], first_for_scope)
        .next()
        .unwrap();
    // and then find the innermost new
    let (deployer_id, innermost_new_body) = match_proc!(first_inner_for_node.proc, ast::Proc::ForComprehension {
        receipts: _,
        proc:
            ast::AnnProc {
                proc:
                    ast::Proc::Par {
                        left: _,
                        right:
                            innermost_node @ ast::AnnProc {
                                proc:
                                    ast::Proc::New {
                                        decls: innermost_decls,
                                        proc: innermost_new_body,
                                    },
                                ..
                            },
                    },
                ..
            },
    } => {
        let inner_new_scope = expect::scope(db, innermost_node, 1);
        expect::captures(&vec![topmost_retch, pos_in_for], inner_new_scope);
        let deployer_id = expect::name_decls(db, innermost_decls, inner_new_scope)
            .next()
            .unwrap();
        (deployer_id, innermost_new_body)
    });

    // and now we can query the body of innermost new for bindings
    expect::bound_in_range(
        db,
        &vec![
            // in @PoS!("bond", *deployerId, 100, *retCh)
            VarBinding::Bound(pos_in_for),
            VarBinding::Bound(deployer_id),
            VarBinding::Bound(topmost_retch),
            // in for(@(true, message) <- retCh) { P }
            VarBinding::Free { index: 0 },
            VarBinding::Bound(topmost_retch),
            // in for body
            VarBinding::Bound(topmost_stdout),
        ],
        innermost_new_body,
    );

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(r#"
new dupe in {
  contract dupe(@depth) = {
    if (depth <= 0) {
      Nil
    } else {
      dupe!(depth - 1) | dupe!(depth - 1) | dupe!(depth - 1) | dupe!(depth - 1) | dupe!(depth - 1) | dupe!(depth - 1) | dupe!(depth - 1) | dupe!(depth - 1) | dupe!(depth - 1) | dupe!(depth - 1)
    }
  } | dupe!(2)
}"#)]
fn test_contract<'test>(tree: ProcRef<'test>, db: &'test mut SemanticDb<'test>) {
    let root = db[tree];
    let resolver = ResolverPass::new(root);
    resolver.run(db);

    let root_scope = expect::scope(db, root, 1);
    let dupe = expect::binder(db, "dupe", root_scope);

    let contract_scope = match_proc!(tree.proc, ast::Proc::New {
        proc:
            ast::AnnProc {
                proc:
                    ast::Proc::Par {
                        left: contract_node,
                        ..
                    },
                ..
            },
        ..
    } => expect::scope(db, contract_node, 1));

    let depth = expect::free(db, vec![("depth", BinderKind::Proc)], contract_scope)
        .next()
        .unwrap();

    let mut expected_bindings = vec![
        VarBinding::Bound(dupe),       // contract dupe
        VarBinding::Free { index: 0 }, // @depth
        VarBinding::Bound(depth),      // if (depth <= 0)
    ];
    expected_bindings.extend(
        std::iter::once(VarBinding::Bound(dupe))
            .chain(std::iter::once(VarBinding::Bound(depth)))
            .cycle()
            .take(20),
    );

    expect::bound_in_scope(db, &expected_bindings, contract_scope);

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"
contract Cell( get, set, state ) = {
  for( rtn <- get; @v <- state ) {
      rtn!( v ) | state!( v ) | Cell!( *get, *set, *state )
  } |
  for( @newValue <- set; _ <- state ) {
      state!( newValue ) | Cell!( *get, *set, *state )
  }
}"#
)]
fn test_pattern_sequence<'test>(tree: ProcRef<'test>, db: &'test mut SemanticDb<'test>) {
    let root = db[tree];
    let resolver = ResolverPass::new(root);
    resolver.run(db);

    let root_scope = expect::scope(db, root, 3);
    let contract_binders: Vec<BinderId> = expect::free(
        db,
        vec![
            ("get", BinderKind::Name(None)),
            ("set", BinderKind::Name(None)),
            ("state", BinderKind::Name(None)),
        ],
        root_scope,
    )
    .collect();

    let ((left_for_scope, left_for_body), (right_for_scope, right_for_body)) = match_proc!(tree.proc, ast::Proc::Contract {
        body:
            ast::AnnProc {
                proc:
                    ast::Proc::Par {
                        left:
                            left @ ast::AnnProc {
                                proc:
                                    ast::Proc::ForComprehension {
                                        proc: left_for_body,
                                        ..
                                    },
                                ..
                            },
                        right:
                            right @ ast::AnnProc {
                                proc:
                                    ast::Proc::ForComprehension {
                                        proc: right_for_body,
                                        ..
                                    },
                                ..
                            },
                    },
                ..
            },
        ..
    } => {
        let left_for_scope = expect::scope(db, left, 2);
        let right_for_scope = expect::scope(db, right, 1);
        ((left_for_scope, left_for_body), (right_for_scope, right_for_body))
    });

    let left_free: Vec<BinderId> = expect::free(
        db,
        vec![("rtn", BinderKind::Name(None)), ("v", BinderKind::Proc)],
        left_for_scope,
    )
    .collect();
    let right_free = expect::free(db, vec![("newValue", BinderKind::Proc)], right_for_scope)
        .next()
        .unwrap();

    expect::bound_in_range(
        db,
        &vec![
            VarBinding::Bound(left_free[0]),        // rtn
            VarBinding::Bound(left_free[1]),        // v
            VarBinding::Bound(contract_binders[2]), // state
            VarBinding::Bound(left_free[1]),        // v
            // Cell is unbound (see below)
            VarBinding::Bound(contract_binders[0]), // get
            VarBinding::Bound(contract_binders[1]), // set
            VarBinding::Bound(contract_binders[2]), // state
        ],
        left_for_body,
    );

    expect::bound_in_range(
        db,
        &vec![
            VarBinding::Bound(contract_binders[2]), // state
            VarBinding::Bound(right_free),          // newValue
            // Cell is unbound (see below)
            VarBinding::Bound(contract_binders[0]), // get
            VarBinding::Bound(contract_binders[1]), // set
            VarBinding::Bound(contract_binders[2]), // state
        ],
        right_for_body,
    );

    // for simplicity in this test we omitted declaration of 'Cell', so we expect it to be unbounded
    expect::error(db, ErrorKind::UnboundVariable, root);
}

#[test_rholang_code(
    r#"
new anyone, unused_rtn in {
    for (auction_contract <- rtn) {
        @auction_contract.get("auction_end")!([anyone, unused_rtn])
    }
}"#
)]
fn test_error_proc_name<'test>(tree: ProcRef<'test>, db: &'test mut SemanticDb<'test>) {
    let root = db[tree];
    let resolver = ResolverPass::new(root);
    resolver.run(db);

    let root_scope = expect::scope(db, root, 2);
    let anyone = expect::binder(db, "anyone", root_scope);
    let anyone_info = db[anyone];
    let unused_rtn = expect::binder(db, "unused_rtn", root_scope);
    let unused_rtn_info = db[unused_rtn];

    let inner_for_scope = expect::scope(
        db,
        |node: ProcRef<'test>| matches!(node.proc, ast::Proc::ForComprehension { .. }),
        1,
    );
    let auction_contract = expect::binder(db, "auction_contract", inner_for_scope);
    let auction_contract_info = db[auction_contract];
    // another way of finding a process
    let inner_for = auction_contract_info.scope;

    expect::error(db, ErrorKind::UnboundVariable, inner_for);
    expect::error(
        db,
        ErrorKind::NameInProcPosition(auction_contract, auction_contract_info.name),
        expect::proc_var_match("auction_contract"),
    );
    expect::error(
        db,
        ErrorKind::NameInProcPosition(anyone, anyone_info.name),
        expect::proc_var_match("anyone"),
    );
    expect::error(
        db,
        ErrorKind::NameInProcPosition(unused_rtn, unused_rtn_info.name),
        expect::proc_var_match("unused_rtn"),
    )
}

mod expect {
    use crate::sem::{
        Binder, BinderId, BinderKind, DiagnosticKind, ErrorKind, PID, ProcRef, ScopeInfo,
        SemanticDb, Symbol, VarBinding,
    };
    use pretty_assertions::{assert_eq, assert_matches};
    use rholang_parser::ast;

    pub trait ProcMatch<'a> {
        fn resolve(self, db: &SemanticDb<'a>) -> Option<PID>;
        fn matches(&self, db: &SemanticDb<'a>, pid: PID) -> bool;
    }

    pub fn proc_var_match<'a>(expected: &str) -> impl ProcMatch<'a> {
        move |node: ProcRef<'a>| matches!(node.proc, ast::Proc::ProcVar(ast::Var::Id(ast::Id { name, .. })) if *name == expected)
    }

    pub fn for_with_channel_match<'a>(expected: &str) -> impl ProcMatch<'a> {
        fn has_source_name<'x>(receipts: &[ast::Receipt], expected: &str) -> bool {
            receipts.iter().flatten().any(|bind| {
                matches!(
                    bind.source_name(),
                    ast::Name::NameVar(ast::Var::Id(ast::Id {name, ..})) if *name == expected
                )
            })
        }
        move |node: ProcRef<'a>| matches!(node.proc, ast::Proc::ForComprehension { receipts, .. } if has_source_name(receipts, expected))
    }

    impl ProcMatch<'_> for PID {
        fn resolve(self, _db: &SemanticDb) -> Option<PID> {
            Some(self)
        }

        fn matches(&self, _db: &SemanticDb, pid: PID) -> bool {
            *self == pid
        }
    }

    impl<'a, F> ProcMatch<'a> for F
    where
        F: Fn(ProcRef<'a>) -> bool,
    {
        fn resolve(self, db: &SemanticDb<'a>) -> Option<PID> {
            db.find_proc(|node| self(node)).map(|(pid, _)| pid)
        }

        fn matches(&self, db: &SemanticDb<'a>, pid: PID) -> bool {
            db.get(pid).is_some_and(|node| self(node))
        }
    }

    impl<'a> ProcMatch<'a> for ProcRef<'a> {
        fn resolve(self, db: &SemanticDb<'a>) -> Option<PID> {
            db.lookup(self)
        }

        fn matches(&self, db: &SemanticDb<'a>, pid: PID) -> bool {
            db.lookup(self).is_some_and(|from_db| from_db == pid)
        }
    }

    pub(super) fn node<'test, M: ProcMatch<'test>>(
        db: &'test SemanticDb<'test>,
        m: M,
    ) -> ProcRef<'test> {
        m.resolve(db)
            .and_then(|proc| db.get(proc))
            .expect("expect::node")
    }

    pub(super) fn scope<'test, M: ProcMatch<'test>>(
        db: &'test SemanticDb<'test>,
        m: M,
        expected_binders: usize,
    ) -> &'test ScopeInfo {
        let expected = m
            .resolve(db)
            .and_then(|proc| db.get_scope(proc))
            .expect("expect::scope");
        assert_eq!(
            expected.num_binders(),
            expected_binders,
            "expect::scope {expected:#?} with {expected_binders} binder(s)"
        );
        expected
    }

    pub(super) fn name_decls<'test>(
        db: &'test SemanticDb,
        name_decls: &[ast::NameDecl],
        scope: &ScopeInfo,
    ) -> impl DoubleEndedIterator<Item = BinderId> + ExactSizeIterator {
        let binders = db.binders(scope);
        let expected_num_decls = name_decls.len();
        assert_eq!(
            binders.len(),
            expected_num_decls,
            "expect::name_decls {binders:#?} with {expected_num_decls} name declaration(s)"
        );

        for (i, (expected_decl, binder)) in name_decls.iter().zip(binders).enumerate() {
            assert_matches!(
                binder,
                Binder {
                    name,
                    kind: BinderKind::Name(uri),
                    scope: _,
                    index,
                    source_position: _
                } if *index == i && symbol_matches_string(db, *name, expected_decl.id.name) && opt_symbol_matches_string(db, *uri, expected_decl.uri.as_deref()),
                "expect::name_decls {expected_decl} at {i}"
            );
        }

        scope.binder_range()
    }

    pub(super) fn free<'test, E>(
        db: &'test SemanticDb,
        names_kinds: E,
        scope: &ScopeInfo,
    ) -> impl Iterator<Item = BinderId> + ExactSizeIterator
    where
        E: IntoIterator<Item = (&'test str, BinderKind)>,
        E::IntoIter: ExactSizeIterator,
    {
        let expected = names_kinds.into_iter();
        let expected_len = expected.len();

        let free = db.free_binders_of(scope);
        assert_eq!(
            free.len(),
            expected_len,
            "expect::free {scope:#?} with {expected_len} binder(s)"
        );

        free.zip(expected).enumerate().map(
            |(i, ((bid, binder), (expected_name, expected_kind)))| {
                assert_matches!(
                    binder,
                    Binder {
                        name,
                        kind,
                        scope: _,
                        index: _,
                        source_position: _
                    } if symbol_matches_string(db, *name, expected_name) && *kind == expected_kind,
                    "expect::free {expected_name} with {expected_kind:#?} at {i}"
                );

                bid
            },
        )
    }

    pub(super) fn captures(expected: &[BinderId], scope: &ScopeInfo) {
        let captures: Vec<BinderId> = scope.captures().collect();
        assert_eq!(captures, expected, "expect::captures");
    }

    pub(super) fn no_warnings_or_errors(db: &SemanticDb) {
        assert_eq!(db.diagnostics(), &[], "expect::no_warning_or_errors");
    }

    pub(super) fn binder(db: &SemanticDb, name: &str, scope: &ScopeInfo) -> BinderId {
        let sym = db.intern(name);
        db.find_binder_for_symbol(sym, scope)
            .unwrap_or_else(|| panic!("expect::binder {:#?} with {sym}", db.binders(scope)))
    }

    pub(super) fn bound(db: &SemanticDb, expected: &[VarBinding]) {
        let actual_bindings: Vec<VarBinding> =
            db.bound_positions().map(|bound| bound.binding).collect();
        assert_eq!(actual_bindings, expected, "expect::bound");
    }

    pub(super) fn bound_in_range(db: &SemanticDb, expected: &[VarBinding], node: ProcRef) {
        let range = node.span;
        let mut actual_bindings = Vec::with_capacity(expected.len());
        actual_bindings.extend(db.bound_in_range(range).map(|bound| bound.binding));
        assert_eq!(
            actual_bindings, expected,
            "expect::bound_in_range with {node:#?}"
        );
    }

    pub(super) fn bound_in_scope(db: &SemanticDb, expected: &[VarBinding], scope: &ScopeInfo) {
        let mut actual_bindings = Vec::with_capacity(expected.len());
        actual_bindings.extend(db.bound_in_scope(scope).map(|bound| bound.binding));
        assert_eq!(
            actual_bindings, expected,
            "expect::bound_in_scope with {scope:#?}"
        );
    }

    pub(super) fn error<'test, M: ProcMatch<'test>>(
        db: &'test SemanticDb<'test>,
        expected: ErrorKind,
        m: M,
    ) {
        db.errors()
            .find(move |diagnostic| {
                matches!(diagnostic.kind, DiagnosticKind::Error(actual) if actual == expected)
                    && m.matches(db, diagnostic.pid)
            })
            .or_else(|| panic!("expect::error #{expected:#?} in {:#?}", db.diagnostics()));
    }

    fn symbol_matches_string(db: &SemanticDb, sym: Symbol, expected: &str) -> bool {
        db.resolve_symbol(sym) == Some(expected)
    }

    fn opt_symbol_matches_string(
        db: &SemanticDb,
        opt_sym: Option<Symbol>,
        expected: Option<&str>,
    ) -> bool {
        match (opt_sym, expected) {
            (None, None) => true,
            (None, Some(_)) => false,
            (Some(_), None) => false,
            (Some(sym), expected) => db.resolve_symbol(sym) == expected,
        }
    }
}
