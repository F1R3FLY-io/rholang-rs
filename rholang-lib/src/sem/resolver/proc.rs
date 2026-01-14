use rholang_parser::ast::{SyncSendCont, inputs, source_names};

use crate::sem::resolver::pattern::{resolve_name_pattern, resolve_proc_pattern};

use super::*;

pub(super) fn resolve(db: &mut SemanticDb, stack: &mut BindingStack, root: PID) {
    let node = db[root];
    resolve_rec(db, stack, node);
}

const STACK_RED_ZONE: usize = 4 * 1024;
const SWEET_STACK_SIZE: usize = 16 * STACK_RED_ZONE;

fn resolve_rec<'a>(db: &mut SemanticDb<'a>, stack: &mut BindingStack, root: ProcRef<'a>) {
    stacker::maybe_grow(STACK_RED_ZONE, SWEET_STACK_SIZE, || {
        resolve_unguarded(db, stack, root)
    })
}

fn resolve_unguarded<'a>(db: &mut SemanticDb<'a>, stack: &mut BindingStack, this: ProcRef<'a>) {
    use super::ErrorKind;
    use ast::BinaryExpOp::*;
    use ast::Case;
    use ast::Collection::*;
    use ast::Proc::*;
    use ast::UnaryExpOp::Negation;
    use ast::Var::*;

    match this.proc {
        // -- connectives (can only be used in patterns) --
        SimpleType(_)
        | VarRef { .. }
        | ProcVar(Wildcard)
        | UnaryExp { op: Negation, .. }
        | BinaryExp {
            op: Conjunction | Disjunction,
            ..
        } => {
            db.error(db[this], ErrorKind::ConnectiveOutsidePattern, None);
        }
        Collection(
            List {
                remainder: Some(var),
                ..
            }
            | Set {
                remainder: Some(var),
                ..
            }
            | Map {
                remainder: Some(var),
                ..
            }
            | PathMap {
                remainder: Some(var),
                ..
            },
        ) => {
            db.error(
                db[this],
                ErrorKind::ConnectiveOutsidePattern,
                var.get_position(),
            );
        }

        // -- ground expressions that do not contain names --
        Nil | Unit | BoolLiteral(_) | LongLiteral(_) | StringLiteral(_) | UriLiteral(_)
        | TheoryCall(_) | Bad => {}

        // -- variables --
        ProcVar(Id(id)) => {
            let resolved = resolve_var(*id, false, db[this], db, stack);
            if resolved.is_none() {
                db.error(db[this], ErrorKind::UnboundVariable, Some(id.pos));
            }
        }

        // -- expressions --
        BinaryExp {
            op: Matches,
            left: target,
            right: pattern,
        } => {
            resolve_unguarded(db, stack, target);
            let pat_id = db[pattern];
            let pattern_scope = resolve_proc_pattern(db, stack, pat_id, pattern.span);
            let _ = db.add_scope(pat_id, pattern_scope); // free variables from this pattern should not be visible at the top level
        }
        Par { left, right } | BinaryExp { left, right, .. } => {
            // left: could be deep → recurse through guarded entry
            resolve_rec(db, stack, left);
            // Right: assumed shallow → recurse directly (unguarded)
            resolve_unguarded(db, stack, right);
        }

        UnaryExp { arg: proc, .. } | Bundle { proc, .. } => {
            resolve_rec(db, stack, proc);
        }

        IfThenElse {
            condition,
            if_true,
            if_false,
        } => {
            resolve_unguarded(db, stack, condition);
            resolve_rec(db, stack, if_true);
            if let Some(branch) = if_false {
                resolve_rec(db, stack, branch);
            }
        }

        Method { receiver, args, .. } => {
            resolve_unguarded(db, stack, receiver);
            for arg in args {
                resolve_unguarded(db, stack, arg);
            }
        }

        Collection(collection) => resolve_collection(collection, db, stack),

        Eval { name } => resolve_name(name, db[this], db, stack),

        Match { expression, cases } => {
            fn resolve_case<'a>(
                pattern: ProcRef<'a>,
                proc: ProcRef<'a>,
                db: &mut SemanticDb<'a>,
                stack: &mut BindingStack,
            ) {
                let pat_id = db[pattern];
                let pattern_scope = resolve_proc_pattern(
                    db,
                    stack,
                    pat_id,
                    SourceSpan {
                        start: pattern.span.start,
                        end: proc.span.end,
                    },
                );

                let mut body = LexicallyScoped::free(db, stack, pat_id, pattern_scope);
                body.with(|db, scoped_stack| resolve_rec(db, scoped_stack, proc))
            }

            resolve_unguarded(db, stack, expression);
            for Case { pattern, proc } in cases {
                resolve_case(pattern, proc, db, stack);
            }
        }

        // -- sends --
        Send {
            channel, inputs, ..
        }
        | SendSync {
            channel,
            inputs,
            cont: SyncSendCont::Empty,
            ..
        } => {
            resolve_send(db[this], channel, inputs, None, db, stack);
        }
        SendSync {
            channel,
            inputs,
            cont: SyncSendCont::NonEmpty(proc),
            ..
        } => resolve_send(db[this], channel, inputs, Some(proc), db, stack),

        // -- new --
        New { decls, proc } => {
            fn bind_decls<'a>(
                db: &mut SemanticDb<'a>,
                decls: &[ast::NameDecl<'a>],
                new: PID,
                span: SourceSpan,
            ) -> ScopeInfo {
                let binder_start = db.next_binder();
                for (i, n) in decls.iter().enumerate() {
                    let name = db.intern(n.id.name);
                    let interned_uri = n.uri.map(|uri| db.intern(&uri));
                    db.fresh_binder(Binder {
                        name,
                        kind: BinderKind::Name(interned_uri),
                        scope: new,
                        index: i,
                        source_position: n.id.pos,
                    });
                }
                ScopeInfo::new(binder_start, decls.len(), span)
            }

            let current = db[this];
            let locals = bind_decls(db, decls, current, this.span);

            let mut block = LexicallyScoped::new(db, stack, current, locals);
            block.with(|db, scoped_stack| resolve_rec(db, scoped_stack, proc))
        }

        // -- for comprehension --
        ForComprehension {
            receipts: sequential,
            proc,
        } => {
            let current = db[this];

            let mut for_scope =
                LexicallyScoped::<AllowDups, PopFree>::empty(db, stack, current, this.span);
            for_scope.with_shadowed(|db, scoped_stack, shadowed| {
                // Resolves a "concurrent group" inside a parallel-for construct.
                // Each concurrent group looks like: `pat1 <- v1 & pat2 <- v2 & ...`.
                // All patterns are resolved independently and then merged into one scope.
                resolve_sequence(
                    sequential,
                    current,
                    this.span,
                    shadowed,
                    db,
                    scoped_stack,
                    |concurrent, db, stack| {
                        // 1. Resolve sources first (unguarded).
                        for arg in inputs(concurrent) {
                            resolve_unguarded(db, stack, arg);
                        }

                        // 2. Resolve names bound by sources
                        for source_name in source_names(concurrent) {
                            resolve_name(source_name, current, db, stack);
                        }

                        // 3. Fold over the patterns, resolving each and merging scopes.
                        concurrent.iter().map(|bind| bind.names())
                    },
                );

                resolve_rec(db, scoped_stack, proc);
            });
        }
        Contract {
            name,
            formals,
            body,
        } => {
            let current = db[this];
            resolve_name(name, current, db, stack);

            let vars = resolve_name_pattern(db, stack, current, this.span, formals, 0);
            let mut contract_body = LexicallyScoped::free(db, stack, current, vars);
            contract_body.with(|db, scoped_stack| resolve_rec(db, scoped_stack, body))
        }

        // let
        Let {
            bindings,
            body,
            concurrent,
        } => {
            let current = db[this];

            // Resolves a `let` expression.
            //
            // - In **concurrent let**, all RHS expressions are resolved first in the *current* scope.
            //   Then, all LHS patterns are resolved together, with duplicate-binder checks across them.
            //   This models `let x = e1 & y = e2 in body` where `x` and `y` do not see each other.
            //
            // - In **sequential let**, each declaration is processed in order:
            //   1. RHS is resolved in the current environment (ignoring its own LHS).
            //   2. LHS pattern is resolved and its binders are immediately available
            //      for subsequent declarations.
            //   This models `let x = e1; y = e2 in body`, where `y` (and `e2`) can see `x`.
            //   (think let rec)
            //
            // In both modes, the `body` is resolved in the extended environment.
            let mut let_scope =
                LexicallyScoped::<AllowDups, PopFree>::empty(db, stack, current, this.span);

            let_scope.with_shadowed(|db, scoped_stack, shadowed| {
                if *concurrent {
                    // concurrent-let: all bindings independent
                    // Step 1: resolve all RHS in the *old* environment.
                    for rhs in bindings.iter().flat_map(|decl| &decl.rhs) {
                        resolve_unguarded(db, scoped_stack, rhs);
                    }

                    // Step 2: resolve all LHS patterns *together*.
                    let lhss = bindings.iter().map(|decl| &decl.lhs);
                    let lhs_scope =
                        resolve_concurrent_patterns(lhss, current, this.span, 0, db, scoped_stack);

                    // Merge new binders into the environment (with duplicate checks).
                    scoped_stack.absorb_free(lhs_scope, db, shadowed);
                } else {
                    // sequential-let (let rec): process one by one
                    resolve_sequence(
                        bindings,
                        current,
                        this.span,
                        shadowed,
                        db,
                        scoped_stack,
                        |let_decl, db, stack| {
                            // Resolve RHS first (unguarded).
                            for rhs in &let_decl.rhs {
                                resolve_unguarded(db, stack, rhs);
                            }
                            // Return just this LHS.
                            std::iter::once(&let_decl.lhs)
                        },
                    );
                }

                // Finally, resolve the body in the extended environment.
                resolve_rec(db, scoped_stack, body);
            });
        }

        Select { branches: _ } => {
            unimplemented!("Select is not implemented in this version of Rholang")
        }

        UseBlock { space, proc } => {
            // Resolve the space name and the body process
            resolve_name(space, db[this], db, stack);
            resolve_rec(db, stack, proc);
        }
    }
}

fn resolve_send<'a>(
    send: PID,
    channel: &'a ast::Name<'a>,
    inputs: &'a [ast::AnnProc<'a>],
    continuation: Option<ProcRef<'a>>,
    db: &mut SemanticDb<'a>,
    stack: &mut BindingStack,
) {
    resolve_name(channel, send, db, stack);
    for input in inputs {
        resolve_unguarded(db, stack, input);
    }
    if let Some(cont) = continuation {
        resolve_rec(db, stack, cont);
    }
}

fn resolve_collection<'a>(
    collection: &'a ast::Collection<'a>,
    db: &mut SemanticDb<'a>,
    stack: &mut BindingStack,
) {
    use ast::Collection::*;

    match collection {
        List { elements, .. }
        | Set { elements, .. }
        | PathMap { elements, .. }
        | Tuple(elements) => {
            for elt in elements {
                resolve_unguarded(db, stack, elt);
            }
        }
        Map { elements, .. } => {
            for (k, v) in elements {
                resolve_unguarded(db, stack, k);
                resolve_unguarded(db, stack, v);
            }
        }
    }
}

fn resolve_name<'a>(
    name: &'a ast::Name<'a>,
    name_proc: PID,
    db: &mut SemanticDb<'a>,
    stack: &mut BindingStack,
) {
    use super::ErrorKind;
    use ast::Name::*;
    use ast::Var::*;

    match name {
        NameVar(Wildcard) => {
            db.error(name_proc, ErrorKind::ConnectiveOutsidePattern, None);
        }
        NameVar(Id(id)) => {
            let resolved = resolve_var(*id, true, name_proc, db, stack);
            if resolved.is_none() {
                db.error(name_proc, ErrorKind::UnboundVariable, Some(id.pos));
            }
        }
        Quote(p) => {
            resolve_unguarded(db, stack, p);
        }
    }
}

/// Resolves a sequence of binding groups, tracking binder indices across them.
///
/// Each group may introduce multiple binders concurrently (patterns in the same group
/// are independent). Before resolving LHS binders, the caller can perform arbitrary
/// RHS resolution work via the `resolve_rhs` callback.
///
/// - `groups`: the collection of groups to process sequentially.
/// - `resolve_rhs`: called once per group, before binders are introduced.
/// - `lhs_patterns`: extracts an iterator over the LHS patterns for that group.
///
/// Binder indices are accumulated across groups. Free binders are absorbed
/// into `stack` (with shadowing tracked in `shadowed`)
fn resolve_sequence<'a, G, Rhs, Lhs>(
    groups: G,
    current: PID,
    span: SourceSpan,
    shadowed: &mut Vec<Shadowed>,
    db: &mut SemanticDb<'a>,
    stack: &mut BindingStack,

    mut resolve_rhs: Rhs,
) where
    G: IntoIterator,
    G::Item: 'a,
    Rhs: FnMut(G::Item, &mut SemanticDb<'a>, &mut BindingStack) -> Lhs,
    Lhs: IntoIterator<Item = NamePattern<'a>>,
{
    let mut num_binders = 0;

    for group in groups {
        // Resolve RHS before introducing LHS binders.
        let lhs = resolve_rhs(group, db, stack);

        // Resolve LHS binders for this group.
        let scope = resolve_concurrent_patterns(lhs, current, span, num_binders, db, stack);

        num_binders += scope.num_binders();
        stack.absorb_free(scope, db, shadowed);
    }
}

fn resolve_concurrent_patterns<'a, P>(
    patterns: P,
    proc: PID,
    span: SourceSpan,
    proc_var_index: usize,
    db: &mut SemanticDb<'a>,
    env: &mut BindingStack,
) -> ScopeInfo
where
    P: IntoIterator<Item = NamePattern<'a>>,
{
    fn absorb_checking_dups(
        acc: &mut ScopeInfo,
        scope: ScopeInfo,
        db: &mut SemanticDb,
        current: PID,
    ) {
        // Check for duplicate free binders

        if acc.num_free() != 0 {
            // That’s quadratic in the number of free binders, but if patterns are small that’s
            // perfectly fine and super simple.
            for new in scope.free() {
                let new_binder = db[new];
                let new_sym = new_binder.name;

                for old in acc.free() {
                    let old_binder = db[old];
                    if old_binder.name == new_sym {
                        db.error(
                            current,
                            ErrorKind::DuplicateVarDef {
                                original: old_binder.into(),
                            },
                            Some(new_binder.source_position),
                        );
                    }
                }
            }
        }

        // Safe to merge now
        acc.absorb(scope);
    }

    let start = db.next_binder();
    patterns
        .into_iter()
        .fold(ScopeInfo::ground(start, span), |mut acc, pattern| {
            let pattern_scope = resolve_name_pattern(
                db,
                env,
                proc,
                SourceSpan::default(),
                pattern,
                proc_var_index + acc.num_binders(),
            );
            absorb_checking_dups(&mut acc, pattern_scope, db, proc);
            acc
        })
}
