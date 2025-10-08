use std::{marker::PhantomData, ops::Range};

use super::*;
use smallvec::SmallVec;

mod pattern;
mod proc;

#[cfg(test)]
mod tests;

pub struct ResolverPass {
    root: PID,
}

impl ResolverPass {
    pub fn new(root: PID) -> Self {
        Self { root }
    }
}

impl Pass for ResolverPass {
    fn name(&self) -> Cow<'static, str> {
        let name = format!("ResolverPass({})", self.root);
        Cow::Owned(name)
    }
}

impl FactPass for ResolverPass {
    fn run(&self, db: &mut SemanticDb) {
        let mut stack = BindingStack::new();
        proc::resolve(db, &mut stack, self.root);
    }
}

pub type NamePattern<'a> = &'a ast::Names<'a>;

struct BindingStack {
    scopes: SmallVec<[ScopeInfo; 4]>,
    env: Env,
}

impl BindingStack {
    fn new() -> Self {
        Self {
            scopes: SmallVec::new(),
            env: Env::new(),
        }
    }

    fn push(&mut self, scope: ScopeInfo, db: &SemanticDb, shadowed: &mut Vec<Shadowed>) {
        self.push_symbols(db.binders_full(&scope), shadowed);
        self.scopes.push(scope);
    }

    fn push_emtpy(&mut self, db: &SemanticDb, span: SourceSpan) {
        self.scopes.push(ScopeInfo::ground(db.next_binder(), span));
    }

    #[must_use]
    fn pop(&mut self) -> ScopeInfo {
        let scope = self.scopes.pop().expect("pop from empty scope stack");
        self.env.shrink(scope.num_binders());
        scope
    }

    fn push_free(&mut self, scope: ScopeInfo, db: &SemanticDb, shadowed: &mut Vec<Shadowed>) {
        self.push_symbols(db.free_binders_of(&scope), shadowed);
        self.scopes.push(scope);
    }

    #[must_use]
    fn pop_free(&mut self) -> ScopeInfo {
        let scope = self.scopes.pop().expect("pop from empty scope stack");
        self.env.shrink(scope.num_free());
        scope
    }

    fn absorb_free(&mut self, scope: ScopeInfo, db: &SemanticDb, shadowed: &mut Vec<Shadowed>) {
        self.push_symbols(db.free_binders_of(&scope), shadowed);
        if let Some(top) = self.scopes.last_mut() {
            top.absorb(scope);
        } else {
            self.scopes.push(scope);
        }
    }

    fn push_symbols<'x, L>(&mut self, locals: L, shadowed: &mut Vec<Shadowed>)
    where
        L: Iterator<Item = (BinderId, &'x Binder)> + ExactSizeIterator,
    {
        self.env.reserve(locals.len());
        locals.for_each(|(id, binder)| {
            if let Some(old_binder) = self.env.lookup(binder.name) {
                shadowed.push(Shadowed {
                    new: id,
                    old: old_binder,
                });
            }
            self.env.push(binder.name, id);
        });
    }

    fn lookup(&self, sym: Symbol) -> Option<BinderId> {
        self.env.lookup(sym)
    }

    fn current_mut(&mut self) -> Option<&mut ScopeInfo> {
        self.scopes.last_mut()
    }

    fn capturing_mut(&mut self, bid: BinderId) -> Option<&mut ScopeInfo> {
        self.scopes.iter_mut().rfind(|scope| scope.contains(bid))
    }
}

fn resolve_var<'a>(
    var: ast::Id,
    expects_name: bool,
    site: PID,
    db: &mut SemanticDb<'a>,
    stack: &mut BindingStack,
) -> Option<(SymbolOccurence, BinderId)> {
    let sym = db.intern(var.name);
    // Step 1: try to resolve against lexical scopes
    if let Some(binder) = stack.lookup(sym) {
        let current = stack
            .current_mut()
            .unwrap_or_else(|| panic!("bug: variable {var} fell off lexical scope!!!"));
        if current.contains(binder) {
            // Case A: binder belongs to *this* scope
            current.mark_used(binder);
        } else {
            // Case B: binder belongs to an *outer* scope
            current.mark_captured(binder);
            if let Some(owner) = stack.capturing_mut(binder) {
                owner.mark_used(binder);
            } else {
                panic!("bug: dangling variable {var} (no owning scope found)");
            }
        }

        // Step 2: record in semantic db
        let occ = SymbolOccurence {
            symbol: sym,
            position: var.pos,
        };
        assert!(
            db.map_symbol_to_binder(occ, binder, expects_name, site),
            "bug: variable {var} already bound!!!"
        );

        Some((occ, binder))
    } else {
        // Case C: not found anywhere
        None
    }
}

fn resolve_var_ref<'a>(
    var: ast::Id,
    kind: ast::VarRefKind,
    pattern: PID,
    db: &mut SemanticDb<'a>,
    stack: &mut BindingStack,
) -> Option<(SymbolOccurence, BinderId)> {
    let sym = db.intern(var.name);

    if let Some(binder) = stack.lookup(sym) {
        let scope = stack
            .capturing_mut(binder)
            .unwrap_or_else(|| panic!("bug: dangling variable {var} (no owning scope found)"));
        scope.mark_used(binder);

        let occ = SymbolOccurence {
            symbol: sym,
            position: var.pos,
        };
        assert!(
            db.map_symbol_to_binder(occ, binder, kind == ast::VarRefKind::Name, pattern),
            "bug: variable {var} already bound!!!"
        );

        Some((occ, binder))
    } else {
        None
    }
}

struct Env {
    locals: SmallVec<[(Symbol, BinderId); 8]>,
}

impl Env {
    fn new() -> Self {
        Self {
            locals: SmallVec::new(),
        }
    }

    fn reserve(&mut self, n: usize) {
        self.locals.reserve_exact(n);
    }

    fn push(&mut self, sym: Symbol, id: BinderId) {
        self.locals.push((sym, id));
    }

    #[inline(always)]
    fn shrink(&mut self, n: usize) {
        let new_len = self
            .locals
            .len()
            .checked_sub(n)
            .expect("shrinking more locals than are present");
        self.locals.truncate(new_len);
    }

    fn lookup(&self, sym: Symbol) -> Option<BinderId> {
        self.locals
            .iter()
            .rfind(|(s, _)| *s == sym)
            .map(|(_, bid)| *bid)
    }

    #[inline(always)]
    fn len(&self) -> usize {
        self.locals.len()
    }

    fn forget(&mut self, range: Range<usize>) {
        let len = self.len();

        if range.start >= len {
            // nothing to remove
            return;
        }

        if range.end >= len {
            // range goes to or past the end → truncate
            self.locals.truncate(range.start);
        } else {
            // otherwise, drain and discard
            self.locals.drain(range);
        }
    }
}

struct Shadowed {
    new: BinderId,
    old: BinderId,
}

/// Controls how to treat shadowed variables when lexical scope is dropped
trait ShadowedStrategy {
    fn report_shadowed(db: &mut SemanticDb, current: PID, shadowed: &[Shadowed]);
}

/// Disallow shadowing within the same process ([`ErrorKind::DuplicateVarDef`]); allow shadowing accross processes
enum DisallowDups {}

/// allow shadowing accross scope, emit a warning ([`WarningKind::ShadowedVar`])
enum AllowDups {}

impl ShadowedStrategy for DisallowDups {
    fn report_shadowed(db: &mut SemanticDb, current: PID, shadowed: &[Shadowed]) {
        use super::{ErrorKind, WarningKind};

        for shadow in shadowed {
            let new_binder = db[shadow.new];
            let old_binder = db[shadow.old];
            if old_binder.scope == current {
                db.error(
                    current,
                    ErrorKind::DuplicateVarDef {
                        original: old_binder.into(),
                    },
                    Some(new_binder.source_position),
                );
            } else {
                db.warning(
                    current,
                    WarningKind::ShadowedVar {
                        original: old_binder.into(),
                    },
                    Some(new_binder.source_position),
                );
            }
        }
    }
}

impl ShadowedStrategy for AllowDups {
    fn report_shadowed(db: &mut SemanticDb, current: PID, shadowed: &[Shadowed]) {
        use super::WarningKind;

        for shadow in shadowed {
            let new_binder = db[shadow.new];
            let old_binder = db[shadow.old];
            db.warning(
                current,
                WarningKind::ShadowedVar {
                    original: old_binder.into(),
                },
                Some(new_binder.source_position),
            );
        }
    }
}

/// Controls what happens on exiting lexical scope
trait PoppingStrategy {
    fn pop(stack: &mut BindingStack) -> ScopeInfo;
}

/// All binders are removed from the [`Env`]
enum PopAll {}

/// Only free binders are removed from the [`Env`]
enum PopFree {}

impl PoppingStrategy for PopAll {
    #[inline(always)]
    fn pop(stack: &mut BindingStack) -> ScopeInfo {
        stack.pop()
    }
}

impl PoppingStrategy for PopFree {
    #[inline(always)]
    fn pop(stack: &mut BindingStack) -> ScopeInfo {
        stack.pop_free()
    }
}

struct LexicallyScoped<'s, 'd, 'a, S = DisallowDups, P = PopAll>
where
    S: ShadowedStrategy,
    P: PoppingStrategy,
{
    stack: &'s mut BindingStack,
    db: &'d mut SemanticDb<'a>,
    scope: PID,
    shadowed: Vec<Shadowed>,
    _ss: PhantomData<S>,
    _ps: PhantomData<P>,
}

impl<'s, 'd, 'a, S, P> LexicallyScoped<'s, 'd, 'a, S, P>
where
    S: ShadowedStrategy,
    P: PoppingStrategy,
{
    #[must_use]
    fn empty(
        db: &'d mut SemanticDb<'a>,
        stack: &'s mut BindingStack,
        scope: PID,
        span: SourceSpan,
    ) -> Self {
        stack.push_emtpy(db, span);
        Self {
            stack,
            db,
            scope,
            shadowed: Vec::new(),
            _ss: PhantomData,
            _ps: PhantomData,
        }
    }

    fn with<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut SemanticDb<'a>, &mut BindingStack) -> R,
    {
        f(self.db, self.stack)
    }

    fn with_shadowed<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut SemanticDb<'a>, &mut BindingStack, &mut Vec<Shadowed>) -> R,
    {
        f(self.db, self.stack, &mut self.shadowed)
    }
}

impl<'s, 'd, 'a> LexicallyScoped<'s, 'd, 'a> {
    fn new(
        db: &'d mut SemanticDb<'a>,
        stack: &'s mut BindingStack,
        scope: PID,
        locals: ScopeInfo,
    ) -> Self {
        let mut shadowed = Vec::new();
        stack.push(locals, db, &mut shadowed);

        Self {
            stack,
            db,
            scope,
            shadowed,
            _ss: PhantomData,
            _ps: PhantomData,
        }
    }
}

impl<'s, 'd, 'a> LexicallyScoped<'s, 'd, 'a, AllowDups, PopFree> {
    fn free(
        db: &'d mut SemanticDb<'a>,
        stack: &'s mut BindingStack,
        scope: PID,
        pattern: ScopeInfo,
    ) -> Self {
        let mut shadowed = Vec::new();
        stack.push_free(pattern, db, &mut shadowed);

        Self {
            stack,
            db,
            scope,
            shadowed,
            _ss: PhantomData,
            _ps: PhantomData,
        }
    }
}

impl<S, P> Drop for LexicallyScoped<'_, '_, '_, S, P>
where
    S: ShadowedStrategy,
    P: PoppingStrategy,
{
    fn drop(&mut self) {
        S::report_shadowed(self.db, self.scope, &self.shadowed);

        let popped_scope = P::pop(self.stack);
        assert!(
            self.db.add_scope(self.scope, popped_scope),
            "bug: scope {} already visited!!!",
            self.scope
        );
    }
}
