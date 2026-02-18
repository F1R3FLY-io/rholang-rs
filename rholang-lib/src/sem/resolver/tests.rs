use test_macros::test_rholang_code;

use crate::{
    match_proc,
    sem::{
        diagnostics::DisjunctionConsistencyCheck,
        pipeline::Pipeline,
        tests::expect::{self, matches},
    },
};

use super::{
    BinderId, BinderKind, ErrorKind, PID, ProcRef, ResolverPass, SemanticDb, VarBinding,
    WarningKind, diagnostics::UnusedVarsPass,
};

use rholang_parser::ast;

fn pipeline<I>(roots: I) -> Pipeline
where
    I: Iterator<Item = PID>,
{
    let pipeline = roots
        .fold(Pipeline::new(), |pipeline, root| {
            pipeline.add_fact(ResolverPass::new(root))
        })
        .add_diagnostic(UnusedVarsPass)
        .add_diagnostic(DisjunctionConsistencyCheck);
    pipeline
}

#[test_rholang_code(
    r#"
    new anyone, rtn in {
      for (@map <- rtn) {
        @map.get("auction_end")!([*anyone])
      }
    }"#, pipeline = pipeline
)]
fn test_scope_nested<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 2);
    let (root_binders, inner_scope) = match_proc!(tree.proc, ast::Proc::New { proc: inner_for, decls } => {
        let root_binders: Vec<BinderId> = expect::name_decls(db, decls, root_scope).collect();
        let inner_scope = expect::scope(db, inner_for, 1);
        (root_binders, inner_scope)
    });

    let [map] = expect::free(db, [("map", BinderKind::Proc)], inner_scope);

    expect::captures(&root_binders, inner_scope);
    expect::bound(
        db,
        &[
            VarBinding::Free { index: 0 },      // @map in for
            VarBinding::Bound(root_binders[1]), // rtn in for
            VarBinding::Bound(map),             // map in map.get("auction_end")
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
}"#, pipeline = pipeline
)]
fn test_pattern_many_names<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
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

    let inner_binders = expect::free(
        db,
        [
            ("blockNumber", BinderKind::Proc),
            ("timestamp", BinderKind::Proc),
            ("sender", BinderKind::Proc),
        ],
        inner_scope,
    );

    expect::captures(&root_binders[1..], inner_scope);
    expect::bound(
        db,
        &[
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
}"#, pipeline = pipeline
)]
fn test_scope_deeply_nested<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 4);

    let topmost_retch = expect::binder(db, "retCh", root_scope);
    let topmost_stdout = expect::binder(db, "stdout", root_scope);

    // find first inner for comprehension
    let first_inner_for_node = expect::node(db, matches::for_with_channel("PoSCh"));
    let first_for_scope = expect::scope(db, db[first_inner_for_node], 1);
    let [pos_in_for] = expect::free(db, [("PoS", BinderKind::Proc)], first_for_scope);

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
        expect::captures(&[topmost_retch, pos_in_for], inner_new_scope);
        let deployer_id = expect::name_decls(db, innermost_decls, inner_new_scope)
            .next()
            .unwrap();
        (deployer_id, innermost_new_body)
    });

    // and now we can query the body of innermost new for bindings
    expect::bound_in_range(
        db,
        &[
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

    expect::unused_variable_warning(db, "message", matches::for_with_channel("retCh"));
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
}"#, pipeline = pipeline)]
fn test_contract<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
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

    let [depth] = expect::free(db, [("depth", BinderKind::Proc)], contract_scope);

    let mut expected_bindings = vec![
        VarBinding::Bound(dupe),       // contract dupe
        VarBinding::Free { index: 0 }, // @depth
        VarBinding::Bound(depth),      // if (depth <= 0)
    ];
    expected_bindings.extend(
        [VarBinding::Bound(dupe), VarBinding::Bound(depth)]
            .iter()
            .cycle()
            .take(20),
    );

    expect::bound_in_scope(db, &expected_bindings, contract_scope);

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"
new loop, primeCheck, stdoutAck(`rho:io:stdoutAck`) in {
  contract loop(@x) = {
    match x {
      [] => Nil
      [head ...tail] => {
        new ret in {
          for (_ <- ret) {
            loop!(tail)
          } | primeCheck!(head, *ret)
        }
      }
    }
  } |
  contract primeCheck(@x, ret) = {
    match x {
      Nil => stdoutAck!("Nil", *ret)
      ~{~Nil | ~Nil} => stdoutAck!("Prime", *ret)
      _ => stdoutAck!("Composite", *ret)
    }
  } |
  loop!([Nil, 7, 7 | 8, 9 | Nil, 9 | 10, Nil, 9])
}"#, pipeline = pipeline
)]
fn test_match<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 3);
    let root_binders: Vec<BinderId> = root_scope.binder_range().collect();
    let var_stdout_ack = root_binders[2];

    let contract_loop = expect::node(db, matches::contract_with_name("loop"));

    let contract_prime_check = expect::node(db, matches::contract_with_name("primeCheck"));
    let prime_check_scope = expect::scope(db, contract_prime_check, 2);
    let [_, var_ret] = expect::free(
        db,
        [("x", BinderKind::Proc), ("ret", BinderKind::Name(None))],
        prime_check_scope,
    );

    let loop_cases = match_proc!(contract_loop.proc, ast::Proc::Contract {
        body: ast::AnnProc {
            proc: ast::Proc::Match { cases, .. }, ..
        }, ..
    } => cases);

    match loop_cases.as_slice() {
        [empty, head_tail] => {
            expect::ground_scope(db, &empty.pattern);

            let head_tail_scope = expect::scope(db, &head_tail.pattern, 2);
            expect::captures(&[], head_tail_scope);
        }
        _ => panic!("Expected 2 cases in {contract_loop:#?}"),
    }

    let prime_check_cases = match_proc!(contract_prime_check.proc, ast::Proc::Contract {
        body: ast::AnnProc {
            proc: ast::Proc::Match { cases, .. }, ..
        }, ..
    } => cases);

    match prime_check_cases.as_slice() {
        [nil, nil_neg, wildcard] => {
            let nil_scope = expect::scope(db, &nil.pattern, 0);
            expect::captures(&[var_stdout_ack, var_ret], nil_scope);

            let nil_neg_scope = expect::scope(db, &nil_neg.pattern, 0);
            expect::captures(&[var_stdout_ack, var_ret], nil_neg_scope);

            let wildcard_scope = expect::scope(db, &wildcard.pattern, 0);
            expect::captures(&[var_stdout_ack, var_ret], wildcard_scope);
        }
        _ => panic!("Expected 3 cases in {contract_prime_check:#?}"),
    }

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
}"#, pipeline = pipeline
)]
fn test_pattern_sequence<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 3);
    let contract_binders = expect::free(
        db,
        [
            ("get", BinderKind::Name(None)),
            ("set", BinderKind::Name(None)),
            ("state", BinderKind::Name(None)),
        ],
        root_scope,
    );

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

    let left_free = expect::free(
        db,
        [("rtn", BinderKind::Name(None)), ("v", BinderKind::Proc)],
        left_for_scope,
    );
    let right_free = expect::free(db, [("newValue", BinderKind::Proc)], right_for_scope);

    expect::bound_in_range(
        db,
        &[
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
        &[
            VarBinding::Bound(contract_binders[2]), // state
            VarBinding::Bound(right_free[0]),       // newValue
            // Cell is unbound (see below)
            VarBinding::Bound(contract_binders[0]), // get
            VarBinding::Bound(contract_binders[1]), // set
            VarBinding::Bound(contract_binders[2]), // state
        ],
        right_for_body,
    );

    // for simplicity in this test we omitted declaration of 'Cell', so we expect it to be unbounded
    expect::error(db, ErrorKind::UnboundVariable, root);
    expect::error(
        db,
        ErrorKind::UnboundVariable,
        |node: ProcRef<'test>| matches!(node.proc, ast::Proc::Send { channel, .. } if channel.is_ident("Cell")),
    );
}

#[test_rholang_code(r#"
new orExample, stdout(`rho:io:stdout`) in {
  contract orExample(@record) = {
    match record {
     {{"name" : {name /\ String},  "age": {age /\ {Int \/ String}}}} => stdout!(["Hello, ", name, " aged ", age])
    }
  } |
  orExample!({"name" : "Joe", "age": 40}) |
  orExample!({"name": "Bob", "age": "41"})
}"#, pipeline = pipeline)]
fn test_connectives<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 2);
    let root_binders: Vec<BinderId> = root_scope.binder_range().collect();
    let var_stdout = root_binders[1];

    let contract = expect::node(db, matches::contract_with_name("orExample"));

    let match_cases = match_proc!(contract.proc, ast::Proc::Contract {
        body: ast::AnnProc {
            proc: ast::Proc::Match { cases, .. }, ..
        }, ..
    } => cases);

    match match_cases.as_slice() {
        [case] => {
            let case_scope = expect::scope(db, &case.pattern, 2);
            let [name, age] = expect::free(
                db,
                [("name", BinderKind::Proc), ("age", BinderKind::Proc)],
                case_scope,
            );

            expect::bound_in_scope(
                db,
                &[
                    VarBinding::Free { index: 0 },
                    VarBinding::Free { index: 1 },
                    VarBinding::Bound(var_stdout),
                    VarBinding::Bound(name),
                    VarBinding::Bound(age),
                ],
                case_scope,
            );
        }
        _ => panic!("Expected 1 case in {contract:#?}"),
    }

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"
new helloNameAge, getOlder, stdout(`rho:io:stdout`) in {
  contract helloNameAge(@{@"name"!(name) | @"age"!(age) | _}) = {
    stdout!(["Hello, ", name, " aged ", age])
  } |
  contract getOlder(@{rest /\ {@"name"!(_) | _} | @"age"!(age) }, ret) = {
    ret!(@"age"!(age + 1) | rest)
  } |
  getOlder!(@"name"!("Joe") | @"age"!(39), *helloNameAge)
}"#, pipeline = pipeline
)]
fn test_pattern_recursive<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 3);
    let root_binders: Vec<BinderId> = root_scope.binder_range().collect();

    let first_contract = expect::node(db, matches::contract_with_name("helloNameAge"));
    let first_contract_scope = expect::scope(db, first_contract, 2);
    let [name1, age1] = expect::free(
        db,
        [("name", BinderKind::Proc), ("age", BinderKind::Proc)],
        first_contract_scope,
    );

    let second_contract = expect::node(db, matches::contract_with_name("getOlder"));
    let second_contract_scope = expect::scope(db, second_contract, 3);
    let [rest, age2, ret] = expect::free(
        db,
        [
            ("rest", BinderKind::Proc),
            ("age", BinderKind::Proc),
            ("ret", BinderKind::Name(None)),
        ],
        second_contract_scope,
    );

    expect::bound_in_scope(
        db,
        &[
            VarBinding::Bound(root_binders[0]), // contract name
            // contract arguments
            VarBinding::Free { index: 0 },
            VarBinding::Free { index: 1 },
            // body
            VarBinding::Bound(root_binders[2]), // stdout
            VarBinding::Bound(name1),
            VarBinding::Bound(age1),
        ],
        first_contract_scope,
    );

    expect::bound_in_scope(
        db,
        &[
            VarBinding::Bound(root_binders[1]), // contract name
            // contract arguments
            VarBinding::Free { index: 0 },
            VarBinding::Free { index: 1 },
            VarBinding::Free { index: 2 },
            // body
            VarBinding::Bound(ret),
            VarBinding::Bound(age2),
            VarBinding::Bound(rest),
        ],
        second_contract_scope,
    );

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"new chan in { 
  for (@{x!(P)}, @{for(y <- z) { y!(Q) }} <- chan) { z!(P) | x!(Q) }
}"#, pipeline = pipeline
)]
fn test_pattern_within_pattern<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 1);
    let var_chan = expect::binder(db, "chan", root_scope);

    let inner_for = match_proc!(tree.proc, ast::Proc::New { proc: inner_for, .. } => {
        expect::scope(db, inner_for, 5)
    });

    let [var_x, var_p, var_z, var_q] = expect::free(
        db,
        [
            ("x", BinderKind::Name(None)),
            ("P", BinderKind::Proc),
            ("z", BinderKind::Name(None)),
            ("Q", BinderKind::Proc),
        ],
        inner_for,
    );
    let var_y = expect::binder(db, "y", inner_for);

    expect::bound_in_scope(
        db,
        &[
            // first pattern
            VarBinding::Free { index: 0 }, // x
            VarBinding::Free { index: 1 }, // P
            // second pattern
            VarBinding::Free { index: 3 }, // y
            VarBinding::Free { index: 2 }, // z
            VarBinding::Bound(var_y),
            VarBinding::Free { index: 4 }, // Q
            // source
            VarBinding::Bound(var_chan),
            // body
            VarBinding::Bound(var_z),
            VarBinding::Bound(var_p),
            VarBinding::Bound(var_x),
            VarBinding::Bound(var_q),
        ],
        inner_for,
    );

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"new chan in { 
  for (@{x!(P)}, @{for(y <- z) { y!(Q) }} <- chan) { x!(P, Q, *y) }
}"#, pipeline = pipeline
)]
fn test_pattern_within_pattern_scoping<'test>(_tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    expect::error(
        db,
        ErrorKind::UnboundVariable,
        |node: ProcRef<'test>| matches!(node.proc, ast::Proc::Eval { name } if name.is_ident("y")),
    );
    expect::unused_variable_warning(db, "z", matches::first_for_comprehension());
}

#[test_rholang_code(
    r#"
new x, y in {
    // This reference to token is a pattern that binds
    for (@token <- x) {
        // This reference should not be binding.
        // It says "if I get the same thing I got from x, do P"
        for (@=token <- y) { token } 
    }
}"#, pipeline = pipeline
)]
fn test_var_ref<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 2);
    match_proc!(tree.proc,
        ast::Proc::New {
            decls: _,
            proc:
                top_for @ ast::AnnProc {
                    proc:
                        ast::Proc::ForComprehension {
                            receipts: _,
                            proc: bottom_for,
                        },
                    ..
                },
        } => {
            let var_y = expect::binder(db, "y", root_scope);
            let top_for_scope = expect::scope(db, top_for, 1);
            let bottom_for_scope = expect::scope(db, bottom_for, 0);

            let [var_token] = expect::free(db, [("token", BinderKind::Proc)], top_for_scope);

            expect::bound_in_scope(
                db,
                &[
                    VarBinding::Bound(var_token),
                    VarBinding::Bound(var_y),
                    VarBinding::Bound(var_token),
                ],
                bottom_for_scope,
            );
            expect::captures(&[var_y, var_token], bottom_for_scope);
        }
    );

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"new port, table in {
        for(@"get", @arg, ack <- port; @value <- @{arg | *table}) {
            @{arg | *table}!(value) |
            ack!(value)
        }
    }"#, pipeline = pipeline
)]
fn test_pattern_sequence_captures<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 2);
    match_proc!(tree.proc,
        ast::Proc::New {
            decls,
            proc:
                inner_for @ ast::AnnProc {
                    proc: ast::Proc::ForComprehension { .. },
                    ..
                },
        } => {
            let root_binders: Vec<BinderId> = expect::name_decls(db, decls, root_scope).collect();

            let for_scope = expect::scope(db, inner_for, 3);
            let [var_arg, var_ack, var_value] = expect::free(
                db,
                [
                    ("arg", BinderKind::Proc),
                    ("ack", BinderKind::Name(None)),
                    ("value", BinderKind::Proc),
                ],
                for_scope,
            );

            expect::bound_in_scope(
                db,
                &[
                    // first pattern
                    VarBinding::Free { index: 0 },      // @arg
                    VarBinding::Free { index: 1 },      // ack
                    VarBinding::Bound(root_binders[0]), // port
                    // second pattern
                    VarBinding::Free { index: 2 },      // @value
                    VarBinding::Bound(var_arg),         // arg in @{ arg | *table }
                    VarBinding::Bound(root_binders[1]), // table in @{ arg | *table }
                    // body
                    VarBinding::Bound(var_arg), // arg in @{ arg | *table }!(value)
                    VarBinding::Bound(root_binders[1]), // table in @{ arg | *table }!(value)
                    VarBinding::Bound(var_value), // value in @{ arg | *table }!(value)
                    VarBinding::Bound(var_ack), // ack in ack!(value)
                    VarBinding::Bound(var_value), // value in ack!(value)
                ],
                for_scope,
            );
            expect::captures(&root_binders, for_scope);
        }
    );

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"new port, table in {
        for(@"get", @arg, ack <- port & @value <- @{arg | *table}) {
            @{arg | *table}!(value) |
            ack!(value)
        }
    }"#, pipeline = pipeline
)]
fn test_pattern_concurrent_captures<'test>(_tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    expect::error(db, ErrorKind::UnboundVariable, matches::proc_var("arg"));
}

#[test_rholang_code(
    r#"new port in {
        for(@"set", @arg1, @arg2, @{ for (@value <- @{arg1 | table}) { @{arg1 | table}!(value) | ack!(_) } } <- port) {
            @{arg1 | table}!(arg2) |
            ack!(true)
        }
    }"#, pipeline = pipeline
)]
fn test_pattern_within_pattern_captures<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 1);
    match_proc!(tree.proc,
        ast::Proc::New {
            decls: _,
            proc:
                inner_for @ ast::AnnProc {
                    proc: ast::Proc::ForComprehension { .. },
                    ..
                },
        } => {
            let var_port = expect::binder(db, "port", root_scope);

            let for_scope = expect::scope(db, inner_for, 5);
            let [var_arg1, var_arg2, var_table, var_ack] = expect::free(
                db,
                [
                    ("arg1", BinderKind::Proc),
                    ("arg2", BinderKind::Proc),
                    ("table", BinderKind::Proc),
                    ("ack", BinderKind::Name(None)),
                ],
                for_scope,
            );
            let var_value = expect::binder(db, "value", for_scope);

            expect::bound_in_scope(
                db,
                &[
                    // first pattern
                    VarBinding::Free { index: 0 }, // @arg1
                    // second pattern
                    VarBinding::Free { index: 1 }, // @arg2
                    // third pattern
                    VarBinding::Free { index: 3 }, // @value in (@value <- @{arg1 | table})
                    VarBinding::Bound(var_arg1), // @arg1 in (@value <- @{arg1 | table})
                    VarBinding::Free { index: 2 }, // table in (@value <- @{arg1 | table})
                    VarBinding::Bound(var_arg1), //  @arg1 in @{arg1 | table}!(@value) | P
                    VarBinding::Bound(var_table), // table in @{arg1 | table}!(@value) | P
                    VarBinding::Bound(var_value), // @value in @{arg1 | table}!(@value) | P
                    VarBinding::Free { index: 4 }, //  ack in ack(_)
                    // source
                    VarBinding::Bound(var_port),
                    // body
                    VarBinding::Bound(var_arg1), // arg1 in @{ arg1 | table }!(arg2)
                    VarBinding::Bound(var_table), // table in @{ arg1 | table }!(arg2)
                    VarBinding::Bound(var_arg2), // arg2 in @{ arg1 | table }!(arg2)
                    VarBinding::Bound(var_ack),    // ack in ack!(true)
                ],
                for_scope,
            );
            expect::captures(&[var_port], for_scope);

    });

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"new port1, port2, port3 in {
        for(@"set", @arg1 <- port1 & @arg2 <- port2 & @{ for (@value <- @{arg1 | table}) { @{arg1 | table}!(value) | ack!(_) } } <- port3) {
            @{arg1 | table}!(arg2) |
            ack!(true)
        }
    }"#, pipeline = pipeline
)]
fn test_pattern_within_pattern_concurrent_scoping<'test>(
    tree: ProcRef<'test>,
    db: &'test SemanticDb<'test>,
) {
    match_proc!(tree.proc,
        ast::Proc::New {
            decls: _,
            proc:
                inner_for @ ast::AnnProc {
                    proc: ast::Proc::ForComprehension { .. },
                    ..
                },
        } => {
            let for_scope = expect::scope(db, inner_for, 6);
            let [var_arg1_1, _, _, _, _] = expect::free(
                db,
                [
                    ("arg1", BinderKind::Proc),
                    ("arg2", BinderKind::Proc),
                    ("arg1", BinderKind::Proc),
                    ("table", BinderKind::Proc),
                    ("ack", BinderKind::Name(None)),
                ],
                for_scope,
            );
            let _ = expect::binder(db, "value", for_scope);

            let var_arg1_1_info = db[var_arg1_1];
            expect::error(db, ErrorKind::DuplicateVarDef { original: var_arg1_1_info.into() }, inner_for);
            // first arg1 is also unused
            expect::warning(db, WarningKind::UnusedVariable(var_arg1_1, var_arg1_1_info.name), inner_for);
    });
}

#[test_rholang_code(
    r#"new port in {
        for(@"set", @arg1, @arg2 <- port ; @{ for (@value <- @{arg1 | table}) { @{arg1 | table}!(value) | ack!(_) } } <- port) {
            @{arg1 | table}!(arg2) |
            ack!(true)
        }
    }"#, pipeline = pipeline
)]
fn test_pattern_within_pattern_sequential_captures<'test>(
    tree: ProcRef<'test>,
    db: &'test SemanticDb<'test>,
) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 1);
    match_proc!(tree.proc,
        ast::Proc::New {
            decls: _,
            proc:
                inner_for @ ast::AnnProc {
                    proc: ast::Proc::ForComprehension { .. },
                    ..
                },
        } => {
            let var_port = expect::binder(db, "port", root_scope);

            let for_scope = expect::scope(db, inner_for, 6);
            let [var_arg1_1, var_arg2, var_arg1_2, var_table, var_ack] = expect::free(
                db,
                [
                    ("arg1", BinderKind::Proc),
                    ("arg2", BinderKind::Proc),
                    ("arg1", BinderKind::Proc),
                    ("table", BinderKind::Proc),
                    ("ack", BinderKind::Name(None)),
                ],
                for_scope,
            );
            let var_value = expect::binder(db, "value", for_scope);

            expect::bound_in_scope(
                db,
                &[
                    // first pattern
                    VarBinding::Free { index: 0 }, // @arg1
                    VarBinding::Free { index: 1 }, // @arg2
                    VarBinding::Bound(var_port),   // source
                    // second pattern
                    VarBinding::Free { index: 4 }, // @value in (@value <- @{arg1 | table})
                    VarBinding::Free { index: 2 }, // @arg1 in (@value <- @{arg1 | table})
                    VarBinding::Free { index: 3 }, // table in (@value <- @{arg1 | table})
                    VarBinding::Bound(var_arg1_2), // @arg1 in @{arg1 | table}!(@value) | P
                    VarBinding::Bound(var_table),  // table in @{arg1 | table}!(@value) | P
                    VarBinding::Bound(var_value),  // @value in @{arg1 | table}!(@value) | P
                    VarBinding::Free { index: 5 }, // ack in ack(_)
                    VarBinding::Bound(var_port),   // source
                    // body
                    VarBinding::Bound(var_arg1_2), // arg1 in @{ arg1 | table }!(arg2)
                    VarBinding::Bound(var_table),  // table in @{ arg1 | table }!(arg2)
                    VarBinding::Bound(var_arg2),   // arg2 in @{ arg1 | table }!(arg2)
                    VarBinding::Bound(var_ack),    // ack in ack!(true)
                ],
                for_scope,
            );
            expect::captures(&[var_port], for_scope);

            // arg1 is shadowed
            let var_arg1_1_info = db[var_arg1_1];
            expect::warning(db, WarningKind::ShadowedVar { original: var_arg1_1_info.into() }, inner_for);
            expect::warning(db, WarningKind::UnusedVariable(var_arg1_1, var_arg1_1_info.name), inner_for);
    });
}

#[test_rholang_code(
    r#"new port in {
        for(@"set", @arg1, @arg2 <- port ; @{ for (@value <- @{=arg1 | table}) { @{=arg1 | table}!(value) | ack!(_) } } <- port) {
            @{arg1 | table}!(arg2) |
            ack!(true)
        }
    }"#, pipeline = pipeline
)]
fn test_pattern_within_pattern_with_var_ref<'test>(
    tree: ProcRef<'test>,
    db: &'test SemanticDb<'test>,
) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 1);
    match_proc!(tree.proc,
        ast::Proc::New {
            decls: _,
            proc:
                inner_for @ ast::AnnProc {
                    proc: ast::Proc::ForComprehension { .. },
                    ..
                },
        } => {
            let var_port = expect::binder(db, "port", root_scope);

            let for_scope = expect::scope(db, inner_for, 5);
            let [var_arg1, var_arg2, var_table, var_ack] = expect::free(
                db,
                [
                    ("arg1", BinderKind::Proc),
                    ("arg2", BinderKind::Proc),
                    ("table", BinderKind::Proc),
                    ("ack", BinderKind::Name(None)),
                ],
                for_scope,
            );
            let var_value = expect::binder(db, "value", for_scope);

            expect::bound_in_scope(
                db,
                &[
                    // first pattern
                    VarBinding::Free { index: 0 }, // @arg1
                    VarBinding::Free { index: 1 }, // @arg2
                    VarBinding::Bound(var_port),   // source
                    // second pattern
                    VarBinding::Free { index: 3 }, // @value in (@value <- @{=arg1 | table})
                    VarBinding::Bound(var_arg1),   // arg1 in (@value <- @{=arg1 | table})
                    VarBinding::Free { index: 2 }, // table in (@value <- @{=arg1 | table})
                    VarBinding::Bound(var_arg1),   // @arg1 in @{=arg1 | table}!(@value) | P
                    VarBinding::Bound(var_table),  // table in @{arg1 | table}!(@value) | P
                    VarBinding::Bound(var_value),  // @value in @{arg1 | table}!(@value) | P
                    VarBinding::Free { index: 4 }, // ack in ack(_)
                    VarBinding::Bound(var_port),   // source
                    // body
                    VarBinding::Bound(var_arg1),   // arg1 in @{ arg1 | table }!(arg2)
                    VarBinding::Bound(var_table),  // table in @{ arg1 | table }!(arg2)
                    VarBinding::Bound(var_arg2),   // arg2 in @{ arg1 | table }!(arg2)
                    VarBinding::Bound(var_ack),    // ack in ack!(true)
                ],
                for_scope,
            );
            expect::captures(&[var_port], for_scope);
    });

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(r#"
new boardMembers, executeProposal, proposals, treasuryBalance in {
  contract executeProposal(@proposalIndex, ret) = {
    for (@currentBoardMembers <- boardMembers & @currentProposals <- proposals) {
      let proposal = currentProposals.get(proposalIndex);
          approvalCount = proposal.get("approvals").size() 
       in if (approvalCount > currentBoardMembers.size() / 2) {
            for (@balance <- treasuryBalance) {
              if (balance >= proposal.get("amount")) {
                let updatedBalance = balance - proposal.get("amount") 
                in { treasuryBalance!(updatedBalance) | ret!("Proposal executed successfully.") }
              } else {
                ret!("Insufficient funds in the treasury.")
              }
            }
          } else {
            ret!("Not enough approvals to execute the proposal.")
          }
    }
  }
}"#, pipeline = pipeline)]
fn test_let<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 4);

    let (root_binders, contract_node) = match_proc!(tree.proc,
        ast::Proc::New { decls, proc } => {
            let root_binders: Vec<BinderId> = expect::name_decls(db, decls, root_scope).collect();
            (root_binders, proc)
        }
    );
    let var_treasury_balance = root_binders[3];

    let contract_scope = expect::scope(db, contract_node, 2);
    let [var_index, var_ret] = expect::free(
        db,
        [
            ("proposalIndex", BinderKind::Proc),
            ("ret", BinderKind::Name(None)),
        ],
        contract_scope,
    );

    let for_node = match_proc!(contract_node.proc, ast::Proc::Contract { body, .. } => body);
    let for_scope = expect::scope(db, for_node, 2);
    let [var_current_mems, var_current_props] = expect::free(
        db,
        [
            ("currentBoardMembers", BinderKind::Proc),
            ("currentProposals", BinderKind::Proc),
        ],
        for_scope,
    );

    let (let_node, inner_for_node) = match_proc!(for_node.proc,
        ast::Proc::ForComprehension {
            proc:
                body @ ast::AnnProc {
                    proc:
                        ast::Proc::Let {
                            body:
                                ast::AnnProc {
                                    proc: ast::Proc::IfThenElse { if_true, .. },
                                    ..
                                },
                            ..
                        },
                    ..
                },
            ..
        } => (body, if_true)
    );

    let let_scope = expect::scope(db, let_node, 2);
    let [var_proposal, var_approval_cnt] = expect::free(
        db,
        [
            ("proposal", BinderKind::Proc),
            ("approvalCount", BinderKind::Proc),
        ],
        let_scope,
    );
    let inner_for_scope = expect::scope(db, inner_for_node, 1);
    let [var_balance] = expect::free(db, [("balance", BinderKind::Proc)], inner_for_scope);

    let inner_let_node = match_proc!(inner_for_node.proc,
        ast::Proc::ForComprehension {
            proc:
                ast::AnnProc {
                    proc: ast::Proc::IfThenElse { if_true, .. },
                    ..
                },
            ..
        } => if_true
    );
    let inner_let_scope = expect::scope(db, inner_let_node, 1);
    let [var_updated_balance] =
        expect::free(db, [("updatedBalance", BinderKind::Proc)], inner_let_scope);

    expect::captures(
        &[var_treasury_balance, var_ret, var_proposal, var_balance],
        inner_let_scope,
    );

    expect::bound_in_scope(
        db,
        &[
            // -- first let
            // proposal = currentProposals.get(proposalIndex);
            VarBinding::Free { index: 0 },
            VarBinding::Bound(var_current_props),
            VarBinding::Bound(var_index),
            // approvalCount = proposal.get("approvals").size()
            VarBinding::Free { index: 1 },
            VarBinding::Bound(var_proposal),
            // -- in
            // approvalCount > currentBoardMembers.size() / 2)
            VarBinding::Bound(var_approval_cnt),
            VarBinding::Bound(var_current_mems),
            // -- -- for
            // @balance <- treasuryBalance
            VarBinding::Free { index: 0 },
            VarBinding::Bound(var_treasury_balance),
            // balance >= proposal.get("amount")
            VarBinding::Bound(var_balance),
            VarBinding::Bound(var_proposal),
            // -- -- second let
            // updatedBalance = balance - proposal.get("amount")
            VarBinding::Free { index: 0 },
            VarBinding::Bound(var_balance),
            VarBinding::Bound(var_proposal),
            // -- -- in
            // treasuryBalance!(updatedBalance) | ret!("Proposal executed successfully.")
            VarBinding::Bound(var_treasury_balance),
            VarBinding::Bound(var_updated_balance),
            VarBinding::Bound(var_ret),
            // --
            VarBinding::Bound(var_ret),
            VarBinding::Bound(var_ret),
        ],
        let_scope,
    );

    expect::no_warnings_or_errors(db);
}

#[test_rholang_code(
    r#"
new anyone, unused_rtn in {
    for (auction_contract <- rtn) {
        @auction_contract.get("auction_end")!([anyone, unused_rtn])
    }
}"#, pipeline = pipeline
)]
fn test_error_proc_name<'test>(tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let root = db[tree];
    let root_scope = expect::scope(db, root, 2);
    let anyone = expect::binder(db, "anyone", root_scope);
    let anyone_info = db[anyone];
    let unused_rtn = expect::binder(db, "unused_rtn", root_scope);
    let unused_rtn_info = db[unused_rtn];

    let inner_for_scope = expect::scope(db, matches::first_for_comprehension(), 1);
    let auction_contract = expect::binder(db, "auction_contract", inner_for_scope);
    let auction_contract_info = db[auction_contract];
    // another way of finding a process
    let inner_for = auction_contract_info.scope;

    expect::error(db, ErrorKind::UnboundVariable, inner_for);
    expect::error(
        db,
        ErrorKind::NameInProcPosition(auction_contract, auction_contract_info.name),
        matches::proc_var("auction_contract"),
    );
    expect::error(
        db,
        ErrorKind::NameInProcPosition(anyone, anyone_info.name),
        matches::proc_var("anyone"),
    );
    expect::error(
        db,
        ErrorKind::NameInProcPosition(unused_rtn, unused_rtn_info.name),
        matches::proc_var("unused_rtn"),
    )
}

#[test_rholang_code(
    r#"
new StudentGradeLevel, stdout(`rho:io:stdout`) in {
    StudentGradeLevel!(@"grade"!(10)) |
    for(@{ 10 \/ 20 /\ x } <- @"chan"){ stdout!(x) } |
    for(@{ @"grade"!(x) \/ @y!(10) } <- StudentGradeLevel) { stdout!(x) }
}"#, pipeline = pipeline
)]
fn test_disjunctions<'test>(_tree: ProcRef<'test>, db: &'test SemanticDb<'test>) {
    let first_for = expect::node(db, matches::first_for_comprehension());
    let second_for = expect::node(db, matches::for_with_channel("StudentGradeLevel"));
    let x = db.intern("x");
    let y = db.intern("y");
    expect::error(db, ErrorKind::UnmatchedVarInDisjunction(x), first_for);
    expect::error(db, ErrorKind::UnmatchedVarInDisjunction(x), second_for);
    expect::error(db, ErrorKind::UnmatchedVarInDisjunction(y), second_for);
    expect::errors(db, 3);
}
