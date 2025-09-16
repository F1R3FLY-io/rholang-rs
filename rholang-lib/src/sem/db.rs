use std::ops::Index;

use by_address::ByAddress;
use indexmap::IndexMap;

use super::*;

const DEFAULT_REV_CAPACITY: usize = 64;

pub type Iter<'db, 'a> = std::iter::Map<
    indexmap::map::Iter<'a, ByAddress<ProcRef<'db>>, PID>,
    fn((&ByAddress<ProcRef<'db>>, &PID)) -> (PID, ProcRef<'db>),
>;

impl<'a> SemanticDb<'a> {
    pub fn new() -> Self {
        Self {
            rev: IndexMap::with_capacity(DEFAULT_REV_CAPACITY),
            diagnostics: Vec::new(),
            has_errors: false,
        }
    }

    /// Builds an index for all processes in preorder DFS.
    /// Returns the PID of the root.
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

    /// Returns a reference to a process by PID, or None if out of bounds.
    pub fn get(&self, id: PID) -> Option<ProcRef<'a>> {
        self.rev.get_index(id.0 as usize).map(|(proc, _)| **proc)
    }

    /// Returns the PID corresponding to a given ProcRef, if it exists in the DB
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

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
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
            .expect("process not found in the DB")
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
