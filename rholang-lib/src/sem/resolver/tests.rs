use test_macros::test_rholang_code;

use super::{BinderId, BinderKind, FactPass, ProcRef, ResolverPass, SemanticDb, VarBinding};

use pretty_assertions::assert_eq;
use rholang_parser::ast;

#[test_rholang_code(
    r#"
    new anyone, rtn in {
      for (@map <- rtn) {
        @map.get("auction_end")!([*anyone])
      }
    }"#
)]
fn test_scope_nested<'test>(tree: ProcRef<'test>, db: &mut SemanticDb<'test>) {
    let root = db[tree];
    let resolver = ResolverPass::new(root);
    resolver.run(db);

    let root_scope = expect::scope(db, root, 2);
    if let ast::Proc::New {
        proc: new_body,
        decls,
    } = tree.proc
    {
        let root_binders: Vec<BinderId> = expect::name_decls(db, decls, root_scope).collect();

        let inner_for = db[new_body];
        let inner_scope = expect::scope(db, inner_for, 1);
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
    } else {
        panic!("unexpected AST structure: {tree:#?}");
    }
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
fn test_multi_name_single_pattern<'test>(tree: ProcRef<'test>, db: &mut SemanticDb<'test>) {
    let root = db[tree];
    let resolver = ResolverPass::new(root);
    resolver.run(db);

    let root_scope = expect::scope(db, root, 3);
    if let ast::Proc::New {
        proc: new_body,
        decls,
    } = tree.proc
    {
        let root_binders: Vec<BinderId> = expect::name_decls(db, decls, root_scope).collect();

        if let ast::Proc::Par { left: _, right } = new_body.proc {
            let inner_for = db[right];
            let inner_scope = expect::scope(db, inner_for, 3);
            assert_eq!(
                inner_scope.num_free(),
                3,
                "expected 'for(@blockNumber, @timestamp, @sender <- retCh) {{ P }}' to introduce three free vars"
            );

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
        } else {
            panic!("unexpected AST structure: {new_body:#?}");
        }
    } else {
        panic!("unexpected AST structure: {tree:#?}");
    }
}

mod expect {
    use crate::sem::{
        Binder, BinderId, BinderKind, PID, ScopeInfo, SemanticDb, Symbol, VarBinding,
    };
    use pretty_assertions::{assert_eq, assert_matches};
    use rholang_parser::ast;

    pub(super) fn scope<'test>(
        db: &'test SemanticDb,
        proc: PID,
        expected_binders: usize,
    ) -> &'test ScopeInfo {
        let expected = db.get_scope(proc).expect("expect::scope");
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

        for (i, expected_decl) in name_decls.iter().enumerate() {
            let binder = binders[i];
            assert_matches!(
                binder,
                Binder {
                    name,
                    kind: BinderKind::Name(uri),
                    scope: _,
                    index,
                    source_position: _
                } if index == i && symbol_matches_string(db, name, expected_decl.id.name) && opt_symbol_matches_string(db, uri, expected_decl.uri.as_deref()),
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
            "expect::free with {expected_len} binder(s)"
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

    pub(super) fn bound(db: &SemanticDb, expected: &[VarBinding]) {
        let actual_bindings: Vec<VarBinding> = db.bound_positions().map(|bound| bound.binding).collect();
        assert_eq!(actual_bindings, expected, "expect::bound");
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
