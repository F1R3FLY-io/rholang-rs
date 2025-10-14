use super::*;
use ast::Proc::*;
use ast::Var;
use rholang_parser::ast::BinaryExpOp;
use rholang_parser::ast::SyncSendCont;

pub(super) fn resolve_proc_pattern<'a>(
    db: &mut SemanticDb<'a>,
    env: &mut BindingStack,
    pid: PID,
    span: SourceSpan,
) -> ScopeInfo {
    let pattern = db[pid];
    let first_binder = db.next_binder();
    // shortcut, we can return right away for simple patterns
    let pattern_proc = pattern.proc;
    if pattern_proc.is_ground() {
        return ScopeInfo::ground(first_binder, span);
    }
    match pattern_proc {
        ProcVar(Var::Id(id)) => {
            new_free(pid, *id, BinderKind::Proc, 0, db);
            ScopeInfo::free_var(first_binder, span)
        }
        VarRef { kind, var } => match resolve_var_ref(*var, *kind, pid, db, env) {
            Some(ref_binder) => ScopeInfo::var_ref(first_binder, ref_binder, span),
            None => {
                db.error(pid, ErrorKind::UnboundVariable, Some(var.pos));
                ScopeInfo::ground(first_binder, span)
            }
        },
        // go into recursive mode for complex
        _ => {
            let mut res = PatternResolver::new(pid, first_binder, 0);
            resolve_proc_pattern_rec(db, env, &mut res, pattern);
            res.take(span)
        }
    }
}

pub(super) fn resolve_name_pattern<'a>(
    db: &mut SemanticDb<'a>,
    env: &mut BindingStack,
    scope: PID,
    span: SourceSpan,
    pattern: NamePattern<'a>,
    proc_var_index: usize,
) -> ScopeInfo {
    use ast::Name;
    use ast::NamesKind;

    let first_binder = db.next_binder();
    match pattern.kind() {
        NamesKind::Empty
        | NamesKind::SingleRemainder(Var::Wildcard)
        | NamesKind::SingleName(Name::NameVar(Var::Wildcard)) => {
            ScopeInfo::ground(first_binder, span)
        }
        NamesKind::SingleName(Name::NameVar(Var::Id(var))) => {
            new_free(scope, *var, BinderKind::Name(None), proc_var_index, db);
            ScopeInfo::free_var(first_binder, span)
        }
        NamesKind::SingleName(Name::Quote(quoted)) if quoted.is_ground() => {
            ScopeInfo::ground(first_binder, span)
        }
        NamesKind::SingleName(Name::Quote(ast::AnnProc {
            proc: ProcVar(Var::Id(id)),
            ..
        })) => {
            new_free(scope, *id, BinderKind::Proc, proc_var_index, db);
            ScopeInfo::free_var(first_binder, span)
        }
        NamesKind::SingleRemainder(Var::Id(var)) => {
            new_free(scope, var, BinderKind::Proc, proc_var_index, db);
            ScopeInfo::free_var(first_binder, span)
        }
        _ => {
            let mut res = PatternResolver::new(scope, first_binder, proc_var_index);
            resolve_names(db, env, &mut res, pattern);
            res.take(span)
        }
    }
}

fn resolve_names<'a>(
    db: &mut SemanticDb<'a>,
    env: &mut BindingStack,
    res: &mut PatternResolver,
    pattern: NamePattern<'a>,
) {
    for n in &pattern.names {
        resolve_single_name(db, env, res, n);
    }
    if let Some(Var::Id(rem)) = pattern.remainder {
        res.resolve_or_introduce_var(rem, BinderKind::Proc, db);
    }
}

fn resolve_single_name<'a>(
    db: &mut SemanticDb<'a>,
    env: &mut BindingStack,
    res: &mut PatternResolver,
    n: &'a ast::Name<'a>,
) {
    use ast::Name;
    match n {
        Name::NameVar(Var::Wildcard) => {}
        Name::NameVar(Var::Id(var)) => {
            res.resolve_or_introduce_var(*var, BinderKind::Name(None), db);
        }
        Name::Quote(quoted) => {
            resolve_proc_pattern_rec(db, env, res, quoted);
        }
    }
}

fn resolve_proc_pattern_rec<'a>(
    db: &mut SemanticDb<'a>,
    env: &mut BindingStack,
    res: &mut PatternResolver,
    pattern: ProcRef<'a>,
) {
    match pattern.proc {
        Nil
        | Unit
        | BoolLiteral(_)
        | LongLiteral(_)
        | StringLiteral(_)
        | UriLiteral(_)
        | SimpleType(_)
        | ProcVar(Var::Wildcard) => {}
        ProcVar(Var::Id(id)) => {
            res.resolve_or_introduce_var(*id, BinderKind::Proc, db);
        }
        VarRef { kind, var } => {
            let resolution = res.resolve_ref(*var, *kind, db, env);
            if resolution.is_none() {
                db.error(res.id, ErrorKind::UnboundVariable, Some(var.pos));
            }
        }

        // expressions
        BinaryExp {
            left: target,
            right: pattern,
            op: BinaryExpOp::Matches,
        } => {
            resolve_proc_pattern_rec(db, env, res, target);
            res.with_fresh_scope(|f| resolve_proc_pattern_rec(db, env, f, pattern));

            if res.top_level() {
                db.warning(
                    res.id,
                    WarningKind::TopLevelPatternExpr { span: pattern.span },
                    None,
                );
            }
        }
        BinaryExp { left, right, op } if op.is_connective() => {
            if *op == BinaryExpOp::Conjunction {
                res.with_fresh_scope(|f| resolve_proc_pattern_rec(db, env, f, left));
                res.with_fresh_scope(|f| resolve_proc_pattern_rec(db, env, f, right));
            } else {
                // here the trick is we want to allow expressions like: [x] \/ [1, x]
                res.with_fresh_scope(|f| {
                    resolve_proc_pattern_rec(db, env, f, left);
                    resolve_proc_pattern_rec(db, env, f, right);
                })
            }
        }
        BinaryExp { left, right, op: _ } => {
            resolve_proc_pattern_rec(db, env, res, left);
            resolve_proc_pattern_rec(db, env, res, right);

            if res.top_level() {
                db.warning(
                    res.id,
                    WarningKind::TopLevelPatternExpr { span: pattern.span },
                    None,
                )
            }
        }
        UnaryExp { arg, op } => {
            let connective = op.is_connective();

            if connective {
                res.with_fresh_scope(|f| resolve_proc_pattern_rec(db, env, f, arg));
            } else {
                resolve_proc_pattern_rec(db, env, res, arg);
            }

            if res.top_level() && !connective {
                db.warning(
                    res.id,
                    WarningKind::TopLevelPatternExpr { span: pattern.span },
                    None,
                )
            }
        }
        Par { left, right } => {
            resolve_proc_pattern_rec(db, env, res, left);
            resolve_proc_pattern_rec(db, env, res, right);
        }
        IfThenElse {
            condition,
            if_true,
            if_false,
        } => {
            resolve_proc_pattern_rec(db, env, res, condition);
            resolve_proc_pattern_rec(db, env, res, if_true);
            if let Some(branch) = if_false {
                resolve_proc_pattern_rec(db, env, res, branch);
            }

            if res.top_level() {
                db.warning(
                    res.id,
                    WarningKind::TopLevelPatternExpr { span: pattern.span },
                    None,
                )
            }
        }
        Method { receiver, args, .. } => {
            resolve_proc_pattern_rec(db, env, res, receiver);
            for arg in args {
                resolve_proc_pattern_rec(db, env, res, arg);
            }

            if res.top_level() {
                db.warning(
                    res.id,
                    WarningKind::TopLevelPatternExpr { span: pattern.span },
                    None,
                )
            }
        }
        Match { expression, cases } => {
            resolve_proc_pattern_rec(db, env, res, expression);
            for ast::Case { pattern, proc } in cases {
                res.with_subpattern(SubPattern::Proc(pattern), db, env, |db, env, res| {
                    resolve_proc_pattern_rec(db, env, res, proc);
                });
            }

            if res.top_level() {
                db.warning(
                    res.id,
                    WarningKind::TopLevelPatternExpr { span: pattern.span },
                    None,
                )
            }
        }
        Collection(collection) => {
            use ast::Collection::*;

            match collection {
                List { elements, .. } | Set { elements, .. } | Tuple(elements) => {
                    for elt in elements {
                        resolve_proc_pattern_rec(db, env, res, elt);
                    }
                }
                Map { elements, .. } => {
                    for (k, v) in elements {
                        resolve_proc_pattern_rec(db, env, res, k);
                        resolve_proc_pattern_rec(db, env, res, v);
                    }
                }
            }

            if let Some(Var::Id(rem)) = collection.remainder() {
                res.resolve_or_introduce_var(rem, BinderKind::Proc, db);
            }
        }
        Eval { name } => {
            resolve_single_name(db, env, res, name);
            if res.top_level() {
                db.warning(
                    res.id,
                    WarningKind::TopLevelPatternExpr { span: pattern.span },
                    None,
                )
            }
        }

        // sends
        Send {
            channel, inputs, ..
        }
        | SendSync {
            channel,
            inputs,
            cont: SyncSendCont::Empty,
        } => {
            resolve_send_pattern(channel, inputs, None, db, env, res);
        }
        SendSync {
            channel,
            inputs,
            cont: SyncSendCont::NonEmpty(cont),
        } => {
            resolve_send_pattern(channel, inputs, Some(cont), db, env, res);
        }

        // for-comprehension
        ForComprehension {
            receipts: sequential,
            proc,
        } => {
            resolve_pattern_chain(
                sequential.iter().flatten(),
                db,
                env,
                res,
                proc,
                |db, env, res, bind| {
                    // 1. Resolve the source name
                    resolve_single_name(db, env, res, bind.source_name());

                    // 2. Resolve inputs if present
                    if let Some(inputs) = bind.input() {
                        for input_pattern in inputs {
                            resolve_proc_pattern_rec(db, env, res, input_pattern);
                        }
                    }

                    // 3. Produce the SubPattern to scope over
                    SubPattern::Name(bind.names())
                },
            );
        }
        Contract {
            name,
            formals,
            body,
        } => {
            resolve_single_name(db, env, res, name);
            res.with_subpattern(SubPattern::Name(formals), db, env, |db, env, res| {
                resolve_proc_pattern_rec(db, env, res, body)
            })
        }

        // let
        Let { bindings, body, .. } => {
            resolve_pattern_chain(
                bindings.iter(),
                db,
                env,
                res,
                body,
                |db, env, res, let_binding| {
                    // 1. Resolve RHS
                    for rhs in &let_binding.rhs {
                        resolve_proc_pattern_rec(db, env, res, rhs);
                    }

                    // 2. Return LHS
                    SubPattern::Name(&let_binding.lhs)
                },
            );
        }

        // new
        New { decls, proc } => {
            for n in decls {
                let interned_uri = n.uri.map(|uri| db.intern(&uri));
                res.introduce_free(n.id, BinderKind::Name(interned_uri), db);
            }
            resolve_proc_pattern_rec(db, env, res, proc);
        }

        Bundle { .. } => {
            db.error(
                res.id,
                ErrorKind::BundleInsidePattern,
                Some(pattern.span.start),
            );
        }
        Select { branches: _ } => {
            unimplemented!("Select is not implemented in this version of Rholang")
        }
        Bad => unreachable!(),
    }
}

fn resolve_send_pattern<'a>(
    channel: &'a ast::Name<'a>,
    inputs: &'a [ast::AnnProc<'a>],
    continuation: Option<ProcRef<'a>>,
    db: &mut SemanticDb<'a>,
    env: &mut BindingStack,
    res: &mut PatternResolver,
) {
    resolve_single_name(db, env, res, channel);
    for input in inputs {
        resolve_proc_pattern_rec(db, env, res, input);
    }

    if let Some(cont) = continuation {
        resolve_proc_pattern_rec(db, env, res, cont)
    }
}

/// Resolves a chain of `for`-like bind patterns, one by one.
///
/// At each step:
/// - We resolve any input patterns.
/// - Then we enter the binder scope (`with_subpattern`) for the LHS.
/// - Finally, we recurse into the remainder of the chain.
///
/// When the iterator is exhausted, `body` is resolved
fn resolve_pattern_chain<'a, I, F>(
    mut iter: I,
    db: &mut SemanticDb<'a>,
    env: &mut BindingStack,
    res: &mut PatternResolver,
    rem: ProcRef<'a>,
    resolve: F,
) where
    I: Iterator,
    I::Item: 'a,
    F: Fn(&mut SemanticDb<'a>, &mut BindingStack, &mut PatternResolver, I::Item) -> SubPattern<'a>,
{
    if let Some(pattern) = iter.next() {
        // Step 1: resolve any input patterns
        let sub_pattern = resolve(db, env, res, pattern);

        // Step 2: open subpattern scope, recurse
        res.with_subpattern(sub_pattern, db, env, |db, env, res| {
            resolve_pattern_chain(iter, db, env, res, rem, resolve);
        });
    } else {
        // No more patterns → resolve the remainder
        resolve_proc_pattern_rec(db, env, res, rem);
    }
}

struct PatternResolver {
    id: PID,
    first_binder: BinderId,
    depth: usize,
    var_index: usize,
    scope_guard: usize,
    env: Env,
    free: BitVec,
    used: BitVec,
    refs: BitSet,
}

impl PatternResolver {
    fn new(pattern_id: PID, first_binder: BinderId, var_index: usize) -> Self {
        Self {
            id: pattern_id,
            first_binder,
            depth: 0,
            var_index,
            scope_guard: 0,
            env: Env::new(),
            free: BitVec::new(),
            used: BitVec::new(),
            refs: BitSet::new(),
        }
    }

    fn take(self, span: SourceSpan) -> ScopeInfo {
        let mut result = ScopeInfo::from_parts(self.first_binder, self.free, self.refs, span);
        result.set_uses(self.used);
        result
    }

    fn top_level(&self) -> bool {
        self.depth == 0
    }

    fn binder_index(&self, binder: BinderId) -> usize {
        (binder - self.first_binder).0 as usize
    }

    fn in_scope(&self, binder: BinderId) -> bool {
        self.binder_index(binder) >= self.scope_guard
    }

    fn set_guard(&mut self, new_guard: usize) -> usize {
        std::mem::replace(&mut self.scope_guard, new_guard)
    }

    #[inline(always)]
    fn next_index(&mut self) -> usize {
        let next = self.var_index + 1;
        std::mem::replace(&mut self.var_index, next)
    }

    #[inline]
    fn __internal_resolve<'b>(
        &mut self,
        var: ast::Id,
        expects_name: bool,
        db: &mut SemanticDb<'b>,
        check_scope: bool,
    ) -> Option<BinderId> {
        let sym = db.intern(var.name);
        if let Some(binder) = self.env.lookup(sym)
            && (!check_scope || self.in_scope(binder))
        {
            // Record the binder as used and add its symbol to the semantic db
            let idx = self.binder_index(binder);
            self.used.set(idx, true);
            let occ = SymbolOccurence {
                symbol: sym,
                position: var.pos,
            };
            assert!(
                db.map_symbol_to_binder(occ, binder, expects_name, self.id),
                "bug: pattern variable {var} already bound!!!"
            );
            return Some(binder);
        }
        None
    }

    fn resolve_pattern_var<'b>(
        &mut self,
        var: ast::Id,
        expects_name: bool,
        db: &mut SemanticDb<'b>,
    ) -> Option<BinderId> {
        self.__internal_resolve(var, expects_name, db, true)
    }

    fn resolve_or_introduce_var<'a>(
        &mut self,
        var: ast::Id,
        kind: BinderKind,
        db: &mut SemanticDb<'a>,
    ) -> BinderId {
        self.resolve_pattern_var(var, kind != BinderKind::Proc, db)
            .unwrap_or_else(|| self.introduce_free(var, kind, db))
    }

    fn resolve_ref<'a>(
        &mut self,
        var: ast::Id,
        kind: ast::VarRefKind,
        db: &mut SemanticDb<'a>,
        lex_env: &mut BindingStack,
    ) -> Option<BinderId> {
        self.__internal_resolve(var, kind == ast::VarRefKind::Name, db, false)
            .or_else(|| {
                let binder = resolve_var_ref(var, kind, self.id, db, lex_env)?;
                self.refs.grow_and_insert(binder.0 as usize);
                Some(binder)
            })
    }

    fn introduce_free(&mut self, id: ast::Id, kind: BinderKind, db: &mut SemanticDb) -> BinderId {
        let (name, fresh) = new_free(self.id, id, kind, self.next_index(), db);

        self.free.push(true);
        self.used.push(false);
        self.env.push(name, fresh);

        fresh
    }

    fn with_fresh_scope<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let old_guard = self.set_guard(self.free.len());
        let result = f(self);
        self.set_guard(old_guard);

        result
    }

    #[inline(never)]
    fn with_subpattern<'a, F, R>(
        &mut self,
        sub: SubPattern<'a>,
        db: &mut SemanticDb<'a>,
        bindings: &mut BindingStack,
        f: F,
    ) -> R
    where
        F: FnOnce(&mut SemanticDb<'a>, &mut BindingStack, &mut Self) -> R,
    {
        let sub_range = self.with_fresh_scope(|fresh| {
            let scope_guard = fresh.scope_guard;

            let start_bound = fresh.env.len();
            sub.call(fresh, db, bindings);
            let end_bound = fresh.env.len();

            // Mark new subpattern binders as non-free for the duration of the callback:
            // the callback resolves expressions that may reference these binders
            fresh.free[scope_guard..].fill(false);

            start_bound..end_bound
        });

        // let caller work inside that subpattern scope
        self.depth += 1;
        let result = f(db, bindings, self);
        self.depth -= 1;

        // We have a contiguous range of binders we want to forget
        self.env.forget(sub_range);
        result
    }
}

enum SubPattern<'p> {
    Proc(ProcRef<'p>),
    Name(NamePattern<'p>),
}

impl<'p> SubPattern<'p> {
    fn call(self, res: &mut PatternResolver, db: &mut SemanticDb<'p>, bindings: &mut BindingStack) {
        match self {
            SubPattern::Proc(proc) => resolve_proc_pattern_rec(db, bindings, res, proc),
            SubPattern::Name(names) => resolve_names(db, bindings, res, names),
        }
    }
}

fn new_free(
    pid: PID,
    id: ast::Id<'_>,
    kind: BinderKind,
    index: usize,
    db: &mut SemanticDb<'_>,
) -> (Symbol, BinderId) {
    let name = db.intern(id.name);
    let pos = id.pos;

    let fresh = db.fresh_binder(Binder {
        name,
        kind,
        scope: pid,
        index,
        source_position: pos,
    });
    assert!(
        db.map_symbol_as_free(
            SymbolOccurence {
                symbol: name,
                position: pos
            },
            index
        ),
        "pattern variable {id} already bound"
    );
    (name, fresh)
}
