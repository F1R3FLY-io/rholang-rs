use std::{iter::Map, ops::Index};

use by_address::ByAddress;
use indexmap::IndexMap;
use rholang_parser::ast;

use crate::sem::{PID, ProcRef, SemanticDb};

const DEFAULT_REV_CAPACITY: usize = 64;

impl<'a> SemanticDb<'a> {
    pub fn new() -> Self {
        Self {
            rev: IndexMap::with_capacity(DEFAULT_REV_CAPACITY),
        }
    }

    /// Builds an index for all processes in preorder DFS.
    /// Returns the PID of the root.
    pub fn build_index(&mut self, root: ProcRef<'a>) -> PID {
        let start_id = self
            .rev
            .len()
            .try_into()
            .expect("Too many elements in the index");
        let result = PID(start_id);

        root.iter_preorder_dfs().enumerate().for_each(|(i, proc)| {
            let key = ByAddress(proc);
            self.rev.insert(key, PID(start_id + i as u32));
        });

        result
    }

    /// Returns a reference to a process by PID, or None if out of bounds.
    pub fn get(&self, id: PID) -> Option<ProcRef<'a>> {
        self.rev.get_index(id.0 as usize).map(|(proc, _)| **proc)
    }

    /// Returns the PID corresponding to a given ProcRef, if it exists in the DB
    pub fn lookup(&self, proc: ProcRef<'a>) -> Option<PID> {
        self.rev.get(&ByAddress(proc)).copied()
    }

    /// Iterate over all PIDs in indexing order
    pub fn pids(&self) -> impl DoubleEndedIterator<Item = PID> + ExactSizeIterator {
        self.rev.values().copied()
    }

    /// Iterate over all (PID, ProcRef) pairs in indexing order.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (PID, ProcRef<'a>)> + ExactSizeIterator {
        self.rev.iter().map(|(proc, pid)| (*pid, **proc))
    }
}

/// Enable `db[pid]` syntax to access the process by PID.
impl<'a> Index<PID> for SemanticDb<'a> {
    type Output = ast::AnnProc<'a>;

    fn index(&self, id: PID) -> &Self::Output {
        self.get(id).expect("PID out of bounds")
    }
}

/// Allow `for (pid, proc) in &db` syntax
impl<'a> IntoIterator for &'a SemanticDb<'a> {
    type Item = (PID, ProcRef<'a>);
    // ugly but works on stable
    type IntoIter = Map<
        indexmap::map::Iter<'a, ByAddress<ProcRef<'a>>, PID>,
        fn((&ByAddress<ProcRef<'a>>, &PID)) -> (PID, ProcRef<'a>),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.rev.iter().map(|(proc, pid)| (*pid, **proc))
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::{assert_eq, assert_matches};
    use rholang_parser::{RholangParser, SourcePos, SourceSpan};
    use smallvec::smallvec;

    use super::*;

    #[test]
    fn test_build_index_single_node() {
        let mut db = SemanticDb::new();
        let root = ast::Proc::Nil.ann(SourcePos::default().span_of(3));

        let root_pid = db.build_index(&root);

        // Test Index<PID>
        assert_matches!(db[root_pid].proc, ast::Proc::Nil);
    }

    #[test]
    fn test_build_index_multiple_nodes() {
        // Create a simple tree: Nil | ()
        let left = ast::Proc::Nil.ann(SourcePos::default().span_of(3));
        let right = ast::Proc::Unit.ann(SourcePos::at_col(7).span_of(2));
        let par_proc = ast::Proc::Par { left, right };
        let root = par_proc.ann(SourceSpan {
            start: SourcePos::default(),
            end: right.span.end,
        });

        let mut db = SemanticDb::new();
        let root_pid = db.build_index(&root);

        // Check root PID
        let root_proc = db[root_pid].proc;
        assert_matches!(root_proc, ast::Proc::Par { .. });
        if let ast::Proc::Par { left: l, right: r } = root_proc {
            assert_matches!(l.proc, ast::Proc::Nil);
            assert_matches!(r.proc, ast::Proc::Unit);

            // Reverse lookup should find PID for each node
            let left_pid = db.lookup(l).unwrap();
            let right_pid = db.lookup(r).unwrap();

            assert_eq!(db[left_pid], left);
            assert_eq!(db[right_pid], right);
        }
    }

    #[test]
    fn test_build_index_nested_nodes() {
        // Construct a small tree:
        // if (true) chan!(Nil) else ()

        let condition = ast::Proc::BoolLiteral(true).ann(SourcePos::at_col(5).span_of(5));

        let send_inputs = smallvec![ast::Proc::Nil.ann(SourcePos::at_col(17).span_of(3))];
        let send_proc = ast::Proc::Send {
            channel: ast::Id {
                name: "chan",
                pos: SourcePos::at_col(11),
            }
            .into(),
            send_type: ast::SendType::Single,
            inputs: send_inputs,
        };
        let if_true = send_proc.ann(SourcePos::at_col(11).span_of(10));

        let if_false = ast::Proc::Unit.ann(SourcePos::at_col(27).span_of(2));

        let if_then_else_proc = ast::Proc::IfThenElse {
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
        assert_matches!(root_proc, ast::Proc::IfThenElse { .. });
        if let ast::Proc::IfThenElse {
            condition: c,
            if_true: t,
            if_false: Some(f),
        } = root_proc
        {
            assert_matches!(c.proc, ast::Proc::BoolLiteral(true));
            assert_matches!(f.proc, ast::Proc::Unit);

            // Reverse lookup should find PID for each node
            let condition_pid = db.lookup(c).unwrap();
            let if_true_pid = db.lookup(t).unwrap();
            let if_false_pid = db.lookup(f).unwrap();

            assert_eq!(db[condition_pid], condition);
            assert_eq!(db[if_true_pid], if_true);
            assert_eq!(db[if_false_pid], if_false);

            // this should also work for nested nodes
            assert_matches!(t.proc, ast::Proc::Send { .. });
            if let ast::Proc::Send { inputs, .. } = t.proc {
                let input_pid = db.lookup(&inputs[0]).unwrap();
                assert_eq!(*db[input_pid].proc, ast::Proc::Nil);
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
            let proc_fwd = &db[pid];
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
        assert_matches!(first_proc.proc, ast::Proc::Let { .. });

        let (_, last_proc) = all.next_back().unwrap();
        assert_matches!(last_proc.proc, ast::Proc::LongLiteral(42));
    }
}
