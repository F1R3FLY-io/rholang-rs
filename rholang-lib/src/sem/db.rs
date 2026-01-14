use super::interner::Interner;
use ahash::RandomState;
use std::ops::Index;

use super::*;

pub type Iter<'db, 'a> = std::iter::Map<
    indexmap::map::Iter<'a, ByAddress<ProcRef<'db>>, PID>,
    fn((&ByAddress<ProcRef<'db>>, &PID)) -> (PID, ProcRef<'db>),
>;

pub type Scopes<'a> = intmap::Values<'a, PID, ScopeInfo>;
pub type ScopesFull<'a> = intmap::Iter<'a, PID, ScopeInfo>;

const DEFAULT_INDEX_CAPACITY: usize = 64;
const DEFAULT_BINDERS_CAPACITY: usize = 16;
const DEFAULT_SCOPES_CAPACITY: usize = 16;

impl<'a> SemanticDb<'a> {
    pub fn new() -> Self {
        Self {
            rev: IndexMap::with_capacity_and_hasher(DEFAULT_INDEX_CAPACITY, RandomState::new()),
            interner: Interner::new(),
            diagnostics: Vec::new(),
            has_errors: false,
            binder_is_name: BitVec::with_capacity(DEFAULT_BINDERS_CAPACITY),
            binders: Vec::with_capacity(DEFAULT_BINDERS_CAPACITY),
            proc_to_scope: IntMap::with_capacity(DEFAULT_SCOPES_CAPACITY),
            enclosing_pids: Vec::new(),
            var_to_binder: BTreeMap::new(),
        }
    }

    /// Returns a [`Symbol`] that uniquely represents the given string.
    ///
    /// If the string was already interned, returns the existing symbol;
    /// otherwise, creates a new symbol. Symbols are unique and stable
    /// within this `SemanticDb`.
    pub fn intern(&self, str: &str) -> Symbol {
        self.interner.intern(str)
    }

    /// Resolves a [`Symbol`] back to the original string.
    ///
    /// Returns `None` if the symbol was never interned in this database.
    pub fn resolve_symbol(&self, sym: Symbol) -> Option<&str> {
        self.interner.resolve(sym)
    }

    /// Resolves a [`Symbol`] back to the original string, which becomes owned by the caller.
    ///
    /// Returns `None` if the symbol was never interned in this database.
    pub fn resolve_symbol_owned(&self, sym: Symbol) -> Option<String> {
        self.interner.resolve_owned(sym)
    }

    pub(super) fn fresh_binder(&mut self, binder: Binder) -> BinderId {
        let id = self.next_binder();
        if id == BinderId::MAX {
            panic!("Too many binders")
        }
        let is_proc = binder.kind == BinderKind::Proc;

        self.binder_is_name.push(!is_proc);
        self.binders.push(binder);

        id
    }

    /// Returns the first unassigned [`BinderId`]
    pub fn next_binder(&self) -> BinderId {
        BinderId(self.binders.len() as u32) // SAFETY: we never allow to add more than |u32| binders
    }

    /// Returns a reference to the binder for the given [`BinderId`],
    /// or `None` if not found.
    pub fn get_binder(&self, bid: BinderId) -> Option<&Binder> {
        self.binders.get(bid.0 as usize)
    }

    /// Builds an index for all processes in preorder DFS.
    /// Returns the [`PID`] of the root.
    pub fn build_index(&mut self, root: ProcRef<'a>) -> PID {
        let start_id = self.pid_count();
        let result = PID(start_id as u32); // SAFETY: we never allow to add more than u32 elements to the index

        root.iter_preorder_dfs().enumerate().for_each(|(i, proc)| {
            let key = ByAddress(proc);
            // SAFETY: below we check if `next` is equal to u32::MAX. So even if the enumeration
            // starts from u32::MAX, the first item will panic
            let next = (start_id + i) as u32;
            if next == u32::MAX {
                // u32::MAX is reserved for top-level (dummy) PID
                panic!("Too many elements in the index");
            }
            self.rev.insert(key, PID(next));
        });

        result
    }

    /// Checks if the given [`ProcRef`] is indexed
    pub fn contains(&self, proc: ProcRef<'a>) -> bool {
        self.lookup(proc).is_some()
    }

    /// Returns a reference to the process by [`PID`], or None if out of bounds.
    pub fn get(&self, id: PID) -> Option<ProcRef<'a>> {
        self.rev.get_index(id.0 as usize).map(|(proc, _)| **proc)
    }

    /// Returns the [`PID`] corresponding to a given [`ProcRef`], if it exists in the DB
    pub fn lookup(&self, proc: ProcRef<'a>) -> Option<PID> {
        self.rev
            .get_index_of(&ByAddress(proc))
            .map(|i| PID(i as u32)) // SAFETY: we never allow to add more than u32 elements to the index
    }

    pub fn pid_count(&self) -> usize {
        self.rev.len()
    }

    /// Iterate over all PIDs in indexing order
    pub fn pids(&self) -> impl DoubleEndedIterator<Item = PID> + ExactSizeIterator {
        self.rev.values().copied()
    }

    /// Iterate over all ([`PID`], [`ProcRef`]) pairs in indexing order.
    pub fn iter(&self) -> Iter<'a, '_> {
        self.rev.iter().map(|(proc, pid)| (*pid, **proc))
    }

    /// Finds the first process node that matches the given predicate.
    ///
    /// This is a convenience for common queries like:
    /// ```example
    /// db.find_proc(|p| matches!(p.proc, ast::Proc::ForComprehension { .. }))
    /// ```
    ///
    /// # Returns
    /// - Some([`PID`], [`ProcRef`]) if a matching process is found.
    /// - `None` if no process satisfies the predicate.
    pub fn find_proc<P>(&self, predicate: P) -> Option<(PID, ProcRef<'a>)>
    where
        P: Fn(ProcRef<'a>) -> bool,
    {
        self.iter().find(|candidate| predicate(candidate.1))
    }

    /// Returns an iterator over all process nodes satisfying the given predicate.
    ///
    /// This is useful when you expect multiple matches, e.g. all `for` comprehensions.
    ///
    pub fn filter_procs<P>(&self, predicate: P) -> impl Iterator<Item = (PID, ProcRef<'a>)>
    where
        P: Fn(ProcRef<'a>) -> bool,
    {
        self.iter().filter(move |candidate| predicate(candidate.1))
    }

    pub fn emit_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
        if let DiagnosticKind::Error(_) = diagnostic.kind {
            self.has_errors = true;
        }
    }

    pub fn push_diagnostics<D>(&mut self, diagnostics: D)
    where
        D: IntoIterator<Item = Diagnostic>,
    {
        if self.has_errors {
            self.diagnostics.extend(diagnostics);
            return;
        }

        let old_len = self.diagnostics.len();
        self.diagnostics.extend(diagnostics);
        self.has_errors = self.diagnostics[old_len..]
            .iter()
            .any(|d| matches!(d.kind, DiagnosticKind::Error(_)));
    }

    pub fn error(&mut self, pid: PID, kind: ErrorKind, pos: Option<SourcePos>) {
        self.emit_diagnostic(Diagnostic::error(pid, kind, pos));
    }

    pub fn warning(&mut self, pid: PID, kind: WarningKind, pos: Option<SourcePos>) {
        self.emit_diagnostic(Diagnostic::warning(pid, kind, pos));
    }

    pub fn has_errors(&self) -> bool {
        self.has_errors
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| matches!(d.kind, DiagnosticKind::Error(_)))
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| matches!(d.kind, DiagnosticKind::Warning(_)))
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    #[inline]
    fn assert_scope_ib(&self, rng: &std::ops::Range<usize>) {
        let next_binder = self.binders.len();
        assert!(
            rng.end <= next_binder,
            "Scope spans beyond the range of binders: {rng:#?} ends beyond {next_binder}"
        );
    }

    /// Adds scope information for the given process.
    ///
    /// Returns `true` if the scope was newly inserted, or `false` if a scope
    /// for this process already existed.
    ///
    /// # Panics
    ///
    /// Panics if the scope’s binder range is out of bounds of the database.
    #[must_use]
    pub(super) fn add_scope(&mut self, proc: PID, scope: ScopeInfo) -> bool {
        self.assert_scope_ib(&scope.as_range());
        self.proc_to_scope.insert_checked(proc, scope)
    }

    /// Returns the scope information associated with the given process, if any.
    pub fn get_scope(&self, proc: PID) -> Option<&ScopeInfo> {
        self.proc_to_scope.get(proc)
    }

    /// Checks if the given process introduced a lexical scope
    pub fn is_scoped(&self, pid: PID) -> bool {
        self.get_scope(pid).is_some()
    }

    /// Returns all binders introduced by the given process.
    ///
    /// Returns `None` if the process does not introduce any binders.
    ///
    /// This is the safe counterpart of [`Self::binders`], which will panic if the
    /// scope is invalid.
    pub fn binders_of(&self, proc: PID) -> Option<&[Binder]> {
        self.get_scope(proc)
            .map(|scope| unsafe { self.binders.get_unchecked(scope.as_range()) })
    }

    /// Checks if the given process has unresolved variables
    pub fn has_free(&self, proc: PID) -> bool {
        self.get_scope(proc)
            .is_some_and(|scope| scope.num_free() != 0)
    }

    /// Returns an iterator over the free binders introduced by the given process
    pub fn free_of(&self, proc: PID) -> Free<'_> {
        self.get_scope(proc)
            .map(|scope| scope.free())
            .unwrap_or_default()
    }

    /// Returns an iterator over all scopes.
    ///
    /// The iteration order is unspecified.
    pub fn scopes(&self) -> Scopes<'_> {
        self.proc_to_scope.values()
    }

    /// Returns an iterator over all processes and their associated scopes.
    ///
    /// The iteration order is unspecified.
    pub fn scopes_full(&self) -> ScopesFull<'_> {
        self.proc_to_scope.iter()
    }

    /// Returns a slice of all binders introduced by the given scope.
    ///
    /// # Panics
    ///
    /// Panics if the scope’s binder range is out of bounds of the database.
    /// Consider using [`Self::binders_of`] if you want a safe version that returns
    /// `None` instead of panicking.
    pub fn binders(&self, scope: &ScopeInfo) -> &[Binder] {
        let rng = scope.as_range();
        self.assert_scope_ib(&rng);
        unsafe { self.binders.get_unchecked(rng) }
    }

    /// Returns an iterator over the binders introduced by the given scope,
    /// along with their [`BinderId`]s.
    ///
    /// # Panics
    ///
    /// Panics if the scope’s binder range is out of bounds of the database.
    pub fn binders_full(
        &self,
        scope: &ScopeInfo,
    ) -> impl ExactSizeIterator<Item = (BinderId, &Binder)> + DoubleEndedIterator {
        let binders = self.binders(scope);
        scope.binder_range().zip(binders)
    }

    /// Returns an iterator over the free binders introduced by the given scope, along with their
    /// [`BinderId`]s.
    ///
    /// # Panics
    ///
    /// The iterator will panic if any of its `next` binders is out of bounds of the database.
    pub fn free_binders_of(
        &self,
        scope: &ScopeInfo,
    ) -> impl ExactSizeIterator<Item = (BinderId, &Binder)> {
        scope.free().map(|bid| (bid, &self[bid]))
    }

    #[inline]
    fn assert_binder_ib(&self, binder: BinderId) {
        assert!(
            binder < self.next_binder(),
            "binder {binder} not within the allocated range of binders: 0..{}",
            self.next_binder()
        );
    }

    fn check_kind(
        &mut self,
        occ: SymbolOccurrence,
        binder: BinderId,
        expects_name: bool,
        site: PID,
    ) {
        let is_name_binder = self.is_name(binder);

        if is_name_binder != expects_name {
            self.error(
                site,
                ErrorKind::kind_mismatch(binder, occ.symbol, expects_name),
                Some(occ.position),
            );
        }
    }

    /// Maps a variable occurrence ([`SymbolOccurence`]) to its binder
    ///
    /// # Panics
    ///
    /// Panics if `binder` is not within the allocated range of binders.
    #[must_use]
    pub(super) fn map_symbol_to_binder(
        &mut self,
        occ: SymbolOccurrence,
        binder: BinderId,
        expects_name: bool,
        site: PID,
    ) -> bool {
        self.assert_binder_ib(binder);
        self.check_kind(occ, binder, expects_name, site);
        let old = self.var_to_binder.insert(occ, VarBinding::Bound(binder));
        old.is_none()
    }

    /// Maps a variable occurrence ([`SymbolOccurence`]) as a free variable.
    /// This is used only in patterns
    #[must_use]
    pub(super) fn map_symbol_as_free(&mut self, occ: SymbolOccurrence, index: usize) -> bool {
        let old = self.var_to_binder.insert(occ, VarBinding::Free { index });
        old.is_none()
    }

    /// Performs a vary fast check if the given binder is name-bound
    pub fn is_name(&self, binder: BinderId) -> bool {
        self.binder_is_name[binder.0 as usize]
    }

    /// Query the binder for a given variable occurrence
    pub fn binder_of(&self, occurence: SymbolOccurrence) -> Option<VarBinding> {
        self.var_to_binder.get(&occurence).copied()
    }

    /// Query the binder for a given [`rholang_parser::ast::Id`]
    pub fn binder_of_id(&self, id: rholang_parser::ast::Id) -> Option<VarBinding> {
        let occurence = SymbolOccurrence::from_id(id, self);
        self.binder_of(occurence)
    }

    /// Returns an iterator over all variable bindings.
    ///
    /// The iteration is in order of appearance in the source code.
    pub fn bound_positions(&self) -> impl ExactSizeIterator<Item = BoundOccurence> {
        self.var_to_binder
            .iter()
            .map(|(occ, binding)| BoundOccurence {
                occurence: *occ,
                binding: *binding,
            })
    }

    /// Returns an iterator over variable bindings that occur within a given source span.
    ///
    /// The range is inclusive–exclusive: `[span.start, span.end)`.
    pub fn bound_in_range(
        &self,
        span: SourceSpan,
    ) -> impl DoubleEndedIterator<Item = BoundOccurence> {
        use std::ops::Bound::*;

        // Construct range bounds for the BTreeMap key type
        let start_key = SymbolOccurrence {
            position: span.start,
            symbol: Symbol::MIN,
        };
        let end_key = SymbolOccurrence {
            position: span.end,
            symbol: Symbol::MIN,
        };

        self.var_to_binder
            .range((Included(start_key), Excluded(end_key)))
            .map(|(occ, binding)| BoundOccurence {
                occurence: *occ,
                binding: *binding,
            })
    }

    /// Returns an iterator over all variable bindings within the given scope.
    pub fn bound_in_scope(
        &self,
        scope: &ScopeInfo,
    ) -> impl DoubleEndedIterator<Item = BoundOccurence> {
        self.bound_in_range(scope.span)
    }

    /// Returns an iterator over free variables that occur within a given source span.
    ///
    /// The range is inclusive–exclusive: `[span.start, span.end)`.
    pub fn free_in_range(&self, span: SourceSpan) -> impl DoubleEndedIterator<Item = VarBinding> {
        self.bound_in_range(span)
            .filter_map(|occ| occ.binding.is_free().then_some(occ.binding))
    }

    /// Finds the binder corresponding to a given symbol within a specific scope.
    ///
    /// This method searches all variable binders declared in `scope`
    /// (and only within that scope) and returns the LAST [`BinderId`] whose bound name
    /// matches the provided [`Symbol`].
    ///
    /// This function does not search parent or nested scopes — only binders declared directly in `scope`.
    /// For this, use [`Self::lookup_in_scope_chain`].
    pub fn find_binder_for_symbol(&self, sym: Symbol, scope: &ScopeInfo) -> Option<BinderId> {
        self.binders_full(scope)
            .rfind(|(_, binder)| binder.name == sym)
            .map(|(bid, _)| bid)
    }

    /// Returns the PID of the immediately enclosing scope-introducing process.
    ///
    ///# Enclosure Map Invariant
    ///
    /// Each process `pid` has exactly one *enclosing* process recorded in `enclosing_pids[pid]`,
    /// corresponding to the nearest **scope-introducing** ancestor in the AST traversal.
    ///
    /// ```text
    ///        +-------------------+        +-------------------+
    ///        |  @Top-level (⊤)  |◄───────┤  P0: "contract"   |   (introduces scope)
    ///        +-------------------+        +-------------------+
    ///                                        │
    ///                                        │  enclosing_pids[P1] = P0
    ///                                        ▼
    ///                                +-------------------+
    ///                                |  P1: "for"        |   (introduces scope)
    ///                                +-------------------+
    ///                                        │
    ///                                        │  enclosing_pids[P2] = P1
    ///                                        ▼
    ///                                +-------------------+
    ///                                |  P2: "send"       |   (no scope)
    ///                                +-------------------+
    /// ```
    /// Thus, for any process `pid`, the chain
    ///
    /// ```text
    /// pid → enclosing_process(pid) → enclosing_process(...) → ... → ⊤
    /// ```
    ///
    /// walks outward through the nested lexical scopes.
    #[inline]
    pub fn enclosing_process(&self, pid: PID) -> Option<PID> {
        self.enclosing_pids
            .get(pid.0 as usize)
            .and_then(|parent| (*parent != PID::TOP_LEVEL).then_some(*parent))
    }

    /// Returns the [`ScopeInfo`] of the nearest enclosing scope for the given process.
    pub fn enclosing_scope(&self, pid: PID) -> Option<&ScopeInfo> {
        self.enclosing_process(pid)
            .and_then(|parent| self.get_scope(parent))
    }

    /// Iterates from the nearest enclosing scope to the outermost one.
    ///
    /// Useful for resolving symbols that may be shadowed across nested scopes.
    pub fn scope_chain(&self, pid: PID) -> impl Iterator<Item = &ScopeInfo> {
        self.process_scope_chain(pid).map(|(_, scope)| scope)
    }

    /// Iterates over `(process, scope)` pairs starting from the given process
    /// and walking up the enclosing chain.
    ///
    /// Precisely:
    /// - If `pid` introduces a scope, it is yielded first.
    /// - Then, ascends through each enclosing process that has a scope.
    /// - Stops when the top-level is reached.
    #[inline]
    pub fn process_scope_chain(&self, pid: PID) -> impl Iterator<Item = (PID, &ScopeInfo)> {
        let mut first = true;
        let mut next_pid: Option<PID> = self.enclosing_process(pid);
        std::iter::from_fn(move || {
            if first {
                // first iteration: check if pid itself has a scope
                first = false;
                if let Some(scope) = self.get_scope(pid) {
                    // include it
                    return Some((pid, scope));
                }
            }
            let current = next_pid?;
            next_pid = self.enclosing_process(current);

            self.get_scope(current).map(|scope| (current, scope))
        })
    }

    /// Looks up a symbol by name, searching outward through the enclosing scopes.
    ///
    /// Returns the first matching binding, starting from the nearest enclosing scope.
    pub fn lookup_in_scope_chain(&self, sym: Symbol, start_pid: PID) -> Option<BinderId> {
        self.scope_chain(start_pid)
            .find_map(|scope| self.find_binder_for_symbol(sym, scope))
    }

    pub fn resolve_var_binding(&self, pid: PID, vb: VarBinding) -> BinderId {
        match vb {
            // For regular bound vars, just look up directly
            VarBinding::Bound(bid) => bid,
            // A “free variable” is free relative to its binder, not globally.
            VarBinding::Free { index } => {
                let scope = self
                    .get_scope(pid)
                    .or_else(|| self.enclosing_scope(pid))
                    .unwrap_or_else(|| panic!("Free var in non-scoped process"));

                scope
                    .binder_range()
                    .nth(index)
                    .unwrap_or_else(|| panic!("Invalid free var index"))
            }
        }
    }

    /// Resolves the defining binder for a symbol either through direct binding
    /// (if known) or via lexical lookup up the scope chain.
    pub fn resolve_occurence(&self, occ: SymbolOccurrence, pid: PID) -> Option<BinderId> {
        self.binder_of(occ)
            .map(|vb| self.resolve_var_binding(pid, vb))
            .or_else(|| // fallback for unresolved or partial symbols
        self.lookup_in_scope_chain(occ.symbol, pid))
    }
}

/// Enable `db[pid]` syntax to access the process by PID.
impl<'a> Index<PID> for SemanticDb<'a> {
    type Output = ProcRef<'a>;

    fn index(&self, id: PID) -> &Self::Output {
        self.rev
            .get_index(id.0 as usize)
            .map(|(proc, _)| proc)
            .expect("PID out of bounds")
    }
}

/// Enable `db[ref]` syntax to access process' PID.
impl<'a> Index<ProcRef<'a>> for SemanticDb<'a> {
    type Output = PID;

    fn index(&self, proc: ProcRef<'a>) -> &Self::Output {
        self.rev
            .get(&ByAddress(proc))
            .unwrap_or_else(|| panic!("process not present in the semantic db:\n-- \n{proc:#?}"))
    }
}

/// Enable `db[bid]` syntax to access binder by its id.
impl<'a> Index<BinderId> for SemanticDb<'a> {
    type Output = Binder;

    fn index(&self, id: BinderId) -> &Self::Output {
        self.assert_binder_ib(id);
        unsafe { self.binders.get_unchecked(id.0 as usize) }
    }
}

/// Enable `db[symbol]` syntax to access String corresponding to the symbol.
impl<'a> Index<Symbol> for SemanticDb<'a> {
    type Output = str;

    fn index(&self, sym: Symbol) -> &Self::Output {
        &self.interner[sym]
    }
}

/// Allow `for (pid, proc) in &db` syntax
impl<'db, 'a> IntoIterator for &'a SemanticDb<'db> {
    type Item = (PID, ProcRef<'db>);
    type IntoIter = Iter<'db, 'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> Default for SemanticDb<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use ast::Proc::*;
    use pretty_assertions::{assert_eq, assert_matches};
    use rholang_parser::{RholangParser, SourcePos, SourceSpan};
    use smallvec::smallvec;

    use super::*;

    #[test]
    fn test_build_index_single_node() {
        let mut db = SemanticDb::new();
        let root = Nil.ann(SourcePos::default().span_of(3));

        let root_pid = db.build_index(&root);

        // Test Index<PID>
        assert_matches!(db[root_pid].proc, Nil);
    }

    #[test]
    fn test_build_index_multiple_nodes() {
        // Create a simple tree: Nil | ()
        let left = Nil.ann(SourcePos::default().span_of(3));
        let right = Unit.ann(SourcePos::at_col(7).span_of(2));
        let par_proc = Par { left, right };
        let root = par_proc.ann(SourceSpan {
            start: SourcePos::default(),
            end: right.span.end,
        });

        let mut db = SemanticDb::new();
        let root_pid = db.build_index(&root);

        // Check root PID
        let root_proc = db[root_pid].proc;
        assert_matches!(root_proc, Par { .. });
        if let Par { left: l, right: r } = root_proc {
            assert_matches!(l.proc, Nil);
            assert_matches!(r.proc, Unit);

            // Reverse lookup should find PID for each node
            let left_pid = db.lookup(l).unwrap();
            let right_pid = db.lookup(r).unwrap();

            assert_eq!(*db[left_pid], left);
            assert_eq!(*db[right_pid], right);
        }
    }

    #[test]
    fn test_build_index_nested_nodes() {
        // Construct a small tree:
        // if (true) chan!(Nil) else ()

        let condition = BoolLiteral(true).ann(SourcePos::at_col(5).span_of(5));

        let send_inputs = smallvec![Nil.ann(SourcePos::at_col(17).span_of(3))];
        let send_proc = Send {
            channel: ast::Id {
                name: "chan",
                pos: SourcePos::at_col(11),
            }
            .into(),
            hyperparams: None,
            send_type: ast::SendType::Single,
            inputs: send_inputs,
        };
        let if_true = send_proc.ann(SourcePos::at_col(11).span_of(10));

        let if_false = Unit.ann(SourcePos::at_col(27).span_of(2));

        let if_then_else_proc = IfThenElse {
            condition,
            if_true,
            if_false: Some(if_false),
        };
        let root = if_then_else_proc.ann(SourcePos::default().span_of(38));

        let mut db = SemanticDb::new();
        let root_pid = db.build_index(&root);

        // The DB should contain 4 nodes in preorder:
        // 0 -> IfThenElse (root)
        // 1 -> condition (BoolLiteral)
        // 2 -> if_true (Send)
        // 3 -> input to Send (Nil)
        // 4 -> if_false (Unit)

        assert_eq!(db.pids().len(), 5);

        // Check root PID
        let root_proc = db[root_pid].proc;
        assert_matches!(root_proc, IfThenElse { .. });
        if let IfThenElse {
            condition: c,
            if_true: t,
            if_false: Some(f),
        } = root_proc
        {
            assert_matches!(c.proc, BoolLiteral(true));
            assert_matches!(f.proc, Unit);

            // Reverse lookup should find PID for each node
            let condition_pid = db.lookup(c).unwrap();
            let if_true_pid = db.lookup(t).unwrap();
            let if_false_pid = db.lookup(f).unwrap();

            assert_eq!(*db[condition_pid], condition);
            assert_eq!(*db[if_true_pid], if_true);
            assert_eq!(*db[if_false_pid], if_false);

            // this should also work for nested nodes
            assert_matches!(t.proc, Send { .. });
            if let Send { inputs, .. } = t.proc {
                let input_pid = db.lookup(&inputs[0]).unwrap();
                assert_eq!(*db[input_pid].proc, Nil);
            }
        }
    }

    #[test]
    fn test_tricky_duplicate_literals() {
        // AST:
        let code = "true | if (true) true else true";

        let parser = RholangParser::new();

        let ast = parser.parse(code).unwrap();
        let mut db = SemanticDb::new();
        ast.iter().for_each(|proc| {
            db.build_index(proc);
        });

        // We expect 6 nodes: Par, lit1, IfThenElse, cond, if_true, if_false
        // Collect all PIDs from reverse lookup
        let mut pids: Vec<_> = db.pids().collect();
        assert_eq!(pids.len(), 6);

        // All PIDs must be distinct
        pids.sort();
        pids.dedup();
        assert_eq!(pids.len(), 6, "All PIDs should be unique");

        // Forward and reverse consistency:
        for pid in pids {
            let proc_fwd = db[pid];
            let proc_back = db.lookup(proc_fwd).unwrap();
            assert_eq!(pid, proc_back, "Forward and reverse lookup must agree");
        }
    }

    #[test]
    fn iterates_complex_nested_structure_correctly() {
        let code = r#"
        let x = 42 in
          bundle {
            match x {
              42 => true | 42
              _  => "hello"
            }}"#;

        let parser = RholangParser::new();

        let ast = parser.parse(code).unwrap();
        let mut db = SemanticDb::new();
        ast.iter().for_each(|proc| {
            db.build_index(proc);
        });

        // We expect the following nodes:
        // 1 -> let
        // 2 -> let body (bundle)
        // 3 -> bundle insides (match)
        // 4 -> match expression (x)
        // 5 -> 1st case (42)
        // 6 -> 1st case (Par)
        // 7 -> left (true)
        // 8 -> right (42)
        // 9 -> 2nd case (_)
        //10 -> 2nd case ("hello")
        //11 -> let binding (42)

        let mut all = db.iter();
        assert_eq!(all.len(), 11);

        // Reverse lookup (we assume DFS preorder, it's internal knowledge, if it changes, change the test)
        let (_, first_proc) = all.next().unwrap();
        assert_matches!(first_proc.proc, Let { .. });

        let (_, last_proc) = all.next_back().unwrap();
        assert_matches!(last_proc.proc, LongLiteral(42));
    }
}
