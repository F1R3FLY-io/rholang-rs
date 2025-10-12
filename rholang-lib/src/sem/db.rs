use super::interner::Interner;
use ahash::RandomState;
use std::ops::Index;

use super::*;

pub type Iter<'db, 'a> = std::iter::Map<
    indexmap::map::Iter<'a, ByAddress<ProcRef<'db>>, PID>,
    fn((&ByAddress<ProcRef<'db>>, &PID)) -> (PID, ProcRef<'db>),
>;

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
            next_binder: 0,
            binder_is_name: BitVec::with_capacity(DEFAULT_BINDERS_CAPACITY),
            binders: Vec::with_capacity(DEFAULT_BINDERS_CAPACITY),
            proc_to_scope: IntMap::with_capacity(DEFAULT_SCOPES_CAPACITY),
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

    pub(super) fn fresh_binder(&mut self, binder: Binder) -> BinderId {
        let id = self.next_binder();
        let is_proc = binder.kind == BinderKind::Proc;

        self.next_binder += 1;
        self.binder_is_name.push(!is_proc);
        self.binders.push(binder);

        id
    }

    /// Returns the first unassigned [`BinderId`]
    pub fn next_binder(&self) -> BinderId {
        BinderId(self.next_binder)
    }

    /// Returns a reference to the binder for the given [`BinderId`],
    /// or `None` if not found.
    pub fn get_binder(&self, bid: BinderId) -> Option<&Binder> {
        self.binders.get(bid.0 as usize)
    }

    /// Builds an index for all processes in preorder DFS.
    /// Returns the [`PID`] of the root.
    pub fn build_index(&mut self, root: ProcRef<'a>) -> PID {
        let start_id = self.rev.len();
        let result = PID(start_id as u32); // SAFETY: we never allow to add more than u32 elements to the index

        root.iter_preorder_dfs().enumerate().for_each(|(i, proc)| {
            let key = ByAddress(proc);
            let next = (start_id + i)
                .try_into()
                .expect("Too many elements in the index");
            self.rev.insert(key, PID(next));
        });

        result
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

    /// Iterate over all PIDs in indexing order
    pub fn pids(&self) -> impl DoubleEndedIterator<Item = PID> + ExactSizeIterator {
        self.rev.values().copied()
    }

    /// Iterate over all (PID, ProcRef) pairs in indexing order.
    pub fn iter(&self) -> Iter<'a, '_> {
        self.rev.iter().map(|(proc, pid)| (*pid, **proc))
    }

    pub fn emit_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
        if let DiagnosticKind::Error(_) = diagnostic.kind {
            self.has_errors = true;
        }
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

    /// Adds scope information for the given process.
    ///
    /// Returns `true` if the scope was newly inserted, or `false` if a scope
    /// for this process already existed.
    pub(super) fn add_scope(&mut self, proc: PID, scope: ScopeInfo) -> bool {
        self.proc_to_scope.insert_checked(proc, scope)
    }

    /// Returns the scope information associated with the given process, if any.
    pub fn get_scope(&self, proc: PID) -> Option<&ScopeInfo> {
        self.proc_to_scope.get(proc)
    }

    /// Returns all binders introduced by the given process.
    ///
    /// Returns `None` if the process does not introduce any binders.
    ///
    /// This is the safe counterpart of [`Self::binders`], which will panic if the
    /// scope is invalid.
    pub fn binders_of(&self, proc: PID) -> Option<&[Binder]> {
        self.get_scope(proc).map(|scope| self.binders(scope))
    }

    /// Returns an iterator over all scopes.
    ///
    /// The iteration order is unspecified.
    pub fn scopes(&self) -> impl Iterator<Item = &ScopeInfo> {
        self.proc_to_scope.values()
    }

    /// Returns an iterator over all processes and their associated scopes.
    ///
    /// The iteration order is unspecified.
    pub fn scopes_full(&self) -> impl Iterator<Item = (PID, &ScopeInfo)> {
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
        assert!(
            rng.end <= (self.next_binder as usize),
            "Scope spans beyond the range of binders"
        );
        unsafe { self.binders.get_unchecked(rng) }
    }

    /// Returns an iterator over the binders introduced by the given scope,
    /// along with their [`BinderId`]s.
    pub fn binders_full(
        &self,
        scope: &ScopeInfo,
    ) -> impl Iterator<Item = (BinderId, &Binder)> + ExactSizeIterator {
        scope.binder_range().map(|bid| (bid, &self[bid]))
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

    fn index(&self, index: ProcRef<'a>) -> &Self::Output {
        self.rev
            .get(&ByAddress(index))
            .expect("process not present in the semantic db")
    }
}

/// Enable `db[bid]` syntax to access binder by its id.
impl<'a> Index<BinderId> for SemanticDb<'a> {
    type Output = Binder;

    fn index(&self, id: BinderId) -> &Self::Output {
        assert!(id.0 < self.next_binder, "unassigned BinderId");
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
