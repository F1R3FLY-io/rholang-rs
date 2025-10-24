use test_macros::test_rholang_code;

use crate::{
    count_tests,
    sem::{
        EnclosureAnalysisPass, PID, ProcRef, ResolverPass, SemanticDb,
        diagnostics::UnusedVarsPass,
        pipeline::Pipeline,
        tests::expect::{self, matches},
    },
};

use rholang_parser::ast::{self, BinaryExpOp};

fn pipeline<I>(roots: I) -> Pipeline
where
    I: Iterator<Item = PID>,
{
    let pipeline = roots
        .fold(Pipeline::new(), |pipeline, root| {
            pipeline
                .add_fact(ResolverPass::new(root))
                .add_fact(EnclosureAnalysisPass::new(root))
        })
        .add_diagnostic(UnusedVarsPass);
    pipeline
}

#[test_rholang_code(
    r#"
new blockData(`rho:block:data`), retCh, stdout(`rho:io:stdout`) in {
  blockData!(*retCh) |
  for(@blockNumber, @timestamp, @sender <- retCh) {
      stdout!({"block number": blockNumber}) |
      stdout!({"block time": timestamp}) |
      stdout!({"block sender": sender})
  }
}"#, pipeline = pipeline
)]
fn test_symbol_lookup<'test>(tree: ProcRef<'test>, db: &'test mut SemanticDb<'test>) {
    let root = db[tree];
    // find all three sends on stdout
    let stdout_sends_iter = db.filter_procs(
        |node| matches!(node.proc, ast::Proc::Send { channel, .. } if channel.is_ident("stdout")),
    );

    count_tests!(3, for (i, (send_pid, send_node)) in stdout_sends_iter.enumerate() => {
        // check if send symbols bind to corresponding binders in the enclosing scope
        let mut send_vars = send_node.iter_vars();
        let stdout_var = send_vars.next().expect("<expected_var_here>!(...)");
        let input_var = send_vars.next().expect("stdout!(<expected_var_here>)");
        // check if `stdout` resolves correctly to the last binder in the root scope
        let stdout_resolved = expect::symbol_resolution(db, stdout_var.as_ident(), send_pid, root, 2);
        let input_resolved =
            expect::symbol_resolution(db, input_var.as_ident(), send_pid, expect::enclosing_process(db, send_pid), i);

        // and we can also ask "precisely" for the same thing
        expect::var_resolution(db, stdout_var, send_pid, &stdout_resolved);
        expect::var_resolution(db, input_var, send_pid, &input_resolved);
    });
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
fn test_process_scope_chain<'test>(_tree: ProcRef<'test>, db: &'test mut SemanticDb<'test>) {
    let prime_check_call_node = expect::node(db, matches::send_on_channel("primeCheck"));
    let prime_check_call_pid = db[prime_check_call_node];
    let [
        (new_pid, _),
        (match_arm_pid, _),
        _, /* contract loop */
        (root_pid, _),
    ] = expect::process_scope_chain::<4>(db, prime_check_call_pid);

    // resolve all the symbols from `primeCheck!(head, *ret)`
    let mut vars = prime_check_call_node.iter_vars();
    let prime_check_var = vars.next().expect("<expected_var_here>!(head, *ret)");
    let head_var = vars.next().expect("primeCheck!(<expected_var_here>, *ret)");
    let ret_var = vars
        .next()
        .expect("primeCheck!(head, *<expected_var_here>)");
    expect::symbol_resolution(
        db,
        prime_check_var.as_ident(),
        prime_check_call_pid,
        root_pid,
        1,
    );
    expect::symbol_resolution(
        db,
        head_var.as_ident(),
        prime_check_call_pid,
        match_arm_pid,
        0,
    );
    expect::symbol_resolution(db, ret_var.as_ident(), prime_check_call_pid, new_pid, 0);
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
fn test_free_var_resolution_in_proc_pattern<'test>(
    _tree: ProcRef<'test>,
    db: &'test mut SemanticDb<'test>,
) {
    // find all conjunctions
    let conjunctions_iter = db.filter_procs(|node| {
        matches!(
            node.proc,
            ast::Proc::BinaryExp {
                op: BinaryExpOp::Conjunction,
                ..
            }
        )
    });

    count_tests!(2, for (i, (pid, node)) in conjunctions_iter.enumerate() => {
        let free_var = node
            .iter_proc_vars()
            .next()
            .expect("{ <expected_var_here> /\\ _}");
        let proc_pattern_scope = expect::enclosing_scope(db, pid);
        let binders = db.binders(proc_pattern_scope);
        expect::var_resolution(db, free_var, pid, &binders[i]);
    });
}
