use test_macros::test_rholang_code;

use super::{
    Binder, BinderId, BinderKind, FactPass, ProcRef, ResolverPass, SemanticDb, Symbol, VarBinding,
};

use pretty_assertions::{assert_eq, assert_matches};
use rholang_parser::ast;

#[test_rholang_code(
    r#"
    new anyone, rtn in {
      for (@map <- rtn) {
        @map.get("auction_end")!([*anyone])
      }
    }"#
)]
fn test_simple_scope<'test>(tree: ProcRef<'test>, db: &mut SemanticDb<'test>) {
    let root = db[tree];
    let resolver = ResolverPass::new(root);
    resolver.run(db);

    let root_scope = db.get_scope(root).expect("expected root scope");
    assert_eq!(
        root_scope.num_binders(),
        2,
        "expected 'new' to introduce two names"
    );

    let root_binders: Vec<BinderId> = root_scope.binder_range().collect();
    if let ast::Proc::New {
        proc: new_body,
        decls,
    } = tree.proc
    {
        ensure_name_decls_introduce_binders(db, decls, &root_binders);

        let inner_for = db[new_body];
        let inner_scope = db.get_scope(inner_for).expect("expected inner 'for' scope");
        assert_eq!(
            inner_scope.num_binders(),
            1,
            "expected 'for (@map <- rtn) {{ P }}' to introduce one var"
        );
        assert_eq!(
            inner_scope.num_free(),
            1,
            "expected 'for (@map <- rtn) {{ P }}' to introduce one free var"
        );

        let (map_bid, map) = db
            .free_binders_of(inner_scope)
            .next()
            .expect("expected a free var");
        assert_matches!(
            *map,
            Binder {
                name,
                kind: BinderKind::Proc,
                scope: _,
                index: 0,
                source_position: _
            } if symbol_matches_string(db, name, "map"),
            "expected 'for (@map <- rtn) {{ P }}' to introduce name: @map"
        );

        assert_eq!(
            inner_scope.num_captures(),
            2,
            "expected inner 'for' to capture two names from enclosing 'new'"
        );

        let for_captured: Vec<BinderId> = inner_scope.captures().collect();
        assert_eq!(
            for_captured, root_binders,
            "expected inner 'for' to capture names from the enclosing 'new'"
        );

        let expected_bindings = vec![
            VarBinding::Free { index: 0 },      // @map in for
            VarBinding::Bound(root_binders[1]), // rtn in for
            VarBinding::Bound(map_bid),         // map in map.get("auction_end")
            VarBinding::Bound(root_binders[0]), // anyone in [*anyone]
        ];
        let actual_bindings: Vec<VarBinding> = db.bound_positions().map(|(_, b)| b).collect();
        assert_eq!(actual_bindings, expected_bindings);

        assert!(
            db.diagnostics().is_empty(),
            "expected no warnings or errors"
        );
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

    let root_scope = db.get_scope(root).expect("expected root scope");
    assert_eq!(
        root_scope.num_binders(),
        3,
        "expected 'new' to introduce three names"
    );

    let root_binders: Vec<BinderId> = root_scope.binder_range().collect();
    if let ast::Proc::New {
        proc: new_body,
        decls,
    } = tree.proc
    {
        ensure_name_decls_introduce_binders(db, decls, &root_binders);

        if let ast::Proc::Par { left: _, right } = new_body.proc {
            let inner_for = db[right];
            let inner_scope = db.get_scope(inner_for).expect("expected inner 'for' scope");
            assert_eq!(
                inner_scope.num_binders(),
                3,
                "expected 'for(@blockNumber, @timestamp, @sender <- retCh) {{ P }}' to introduce three vars"
            );
            assert_eq!(
                inner_scope.num_free(),
                3,
                "expected 'for(@blockNumber, @timestamp, @sender <- retCh) {{ P }}' to introduce three free vars"
            );

            let inner_binders: Vec<BinderId> = inner_scope.free().collect();
            let expected_binders = vec![
                ("blockNumber", BinderKind::Proc),
                ("timestamp", BinderKind::Proc),
                ("sender", BinderKind::Proc),
            ];
            ensure_binders_in_order(db, &inner_binders, &expected_binders);

            assert_eq!(
                inner_scope.num_captures(),
                2,
                "expected inner 'for' to capture two names from the enclosing 'new'"
            );

            let expected_bindings = vec![
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
            ];
            let actual_bindings: Vec<VarBinding> = db.bound_positions().map(|(_, b)| b).collect();
            assert_eq!(actual_bindings, expected_bindings);

            assert!(
                db.diagnostics().is_empty(),
                "expected no warnings or errors"
            );
        } else {
            panic!("unexpected AST structure: {new_body:#?}");
        }
    } else {
        panic!("unexpected AST structure: {tree:#?}");
    }
}

fn ensure_name_decls_introduce_binders(
    db: &SemanticDb,
    decls: &[ast::NameDecl],
    bids: &[BinderId],
) {
    let num_decls = decls.len();
    let num_binders = bids.len();
    assert_eq!(
        num_decls, num_binders,
        "expected 'new' with {num_decls} name declaration(s) to introduce equal number of binders"
    );
    for (i, decl) in decls.iter().enumerate() {
        let binder = db[bids[i]];
        assert_matches!(
            binder,
            Binder {
                name,
                kind: BinderKind::Name(uri),
                scope: _,
                index,
                source_position: _
            } if index == i && symbol_matches_string(db, name, decl.id.name) && opt_symbol_matches_string(db, uri, decl.uri.as_deref()),
            "expected 'new' to bind only names in declaration order"
        );
    }
}

fn ensure_binders_in_order(db: &SemanticDb, bids: &[BinderId], names_kinds: &[(&str, BinderKind)]) {
    let num_binders = bids.len();
    assert_eq!(
        num_binders,
        names_kinds.len(),
        "expected {num_binders} binder(s)"
    );
    for (i, bid) in bids.iter().enumerate() {
        let binder = db[*bid];
        let (expected_name, expected_kind) = names_kinds[i];
        assert_matches!(
            binder,
            Binder {
                name,
                kind,
                scope: _,
                index,
                source_position: _
            } if index == i && symbol_matches_string(db, name, expected_name) && kind == expected_kind,
            "unexpected binder"
        );
    }
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
