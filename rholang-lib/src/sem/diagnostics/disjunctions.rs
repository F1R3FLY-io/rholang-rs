use bitvec::prelude::*;
use rholang_parser::{
    DfsEventExt,
    ast::{self, BinaryExpOp},
};
use smallvec::SmallVec;

use crate::sem::{
    Diagnostic, DiagnosticPass, ErrorKind, PID, Pass, ProcRef, SemanticDb, SymbolOccurrence,
    diagnostics::DisjunctionConsistencyCheck,
};
use std::borrow::Cow;

impl Pass for DisjunctionConsistencyCheck {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("∨ Consistency Check")
    }
}

impl DiagnosticPass for DisjunctionConsistencyCheck {
    fn run(&self, db: &SemanticDb) -> Vec<Diagnostic> {
        let mut result = Vec::new();
        for (pid, ast) in db {
            let proc = ast.proc;
            // check if we encounter disjunction as part of a proc-pattern (match, matches)
            if let ast::Proc::BinaryExp {
                op: BinaryExpOp::Disjunction,
                left,
                right,
            } = proc
            {
                check_disjunction(db, left, right, pid, &mut result);
                continue; // no need to descend into sub-disjunctions manually.
                // The DB iteration will revisit all sub-procs independently as top-level AST roots.
            }

            // Skip ASTs that don't introduce a lexical scope.
            // Only scoped processes can contain disjunctive name-patterns worth checking.
            if !db.is_scoped(pid) {
                continue;
            }

            // else, check if it hides in a name-pattern
            for name in ast.iter_names_direct() {
                check_deep(db, name, pid, &mut result)
            }
        }
        result
    }
}

fn check_deep<'a>(
    db: &SemanticDb<'a>,
    name: &'a ast::Name<'a>,
    site: PID,
    result: &mut Vec<Diagnostic>,
) {
    fn is_atom<'x>(name: &'x ast::Name<'x>) -> bool {
        name.is_trivially_ground()
            || name.as_var().is_some()
            || name.as_quote().is_some_and(|q| q.as_var().is_some())
    }

    if is_atom(name) {
        return;
    }
    if let Some(q) = name.as_quote()
        && db.contains(q)
    {
        // if it is indexed we will visit it later on
        return;
    }

    name.iter_into_deep().for_each(|ev| {
        if let DfsEventExt::Enter(node) = ev
            && let ast::Proc::BinaryExp {
                op: BinaryExpOp::Disjunction,
                left,
                right,
            } = node.proc
        {
            check_disjunction(db, left, right, site, result);
        }
    });
}

fn check_disjunction<'a>(
    db: &SemanticDb<'a>,
    left: ProcRef<'a>,
    right: ProcRef<'a>,
    site: PID,
    result: &mut Vec<Diagnostic>,
) {
    /// Construct a diagnostic for a variable that doesn't match between disjuncts.
    fn error(site: PID, occ: SymbolOccurrence) -> Diagnostic {
        Diagnostic::error(
            site,
            ErrorKind::UnmatchedVarInDisjunction(occ.symbol),
            Some(occ.position),
        )
    }

    // In a disjunctive pattern (p \/ q), the left-hand side declares
    // the "shape" of the pattern — its free variables.
    // The right-hand side must bind *exactly the same* variables,
    // neither introducing new ones nor omitting any.
    let right_bindings = db.bound_in_range(right.span);
    let left_free: SmallVec<[_; 1]> = db
        .free_in_range(left.span)
        .map(|free| db.resolve_var_binding(site, free))
        .collect();

    // Track which left-hand variables were successfully matched on the right.
    let mut seen: BitVec = BitVec::repeat(false, left_free.len());

    for rb in right_bindings {
        // If the RHS introduces a variable that is not bound to anything from the LHS,
        // it's an error — disjunction branches must share the same variable interface.
        if rb.binding.is_free() {
            result.push(error(site, rb.occurence));
            continue;
        }

        // If the RHS binds to an existing LHS variable, mark it as "seen".
        let bound = db.resolve_var_binding(site, rb.binding);
        if let Some(pos_on_left) = left_free.iter().position(|free| *free == bound) {
            seen.set(pos_on_left, true);
        }
        // If the binding corresponds to something unrelated, that's fine —
        // only missing or extra variables are errors.
    }

    // If every LHS variable was matched, we're done.
    if seen.all() {
        return;
    }

    // Otherwise, report every free LHS variable that had no corresponding binding on RHS.
    for i in seen.iter_zeros() {
        let binder = left_free[i];
        result.push(error(site, db[binder].into()));
    }
}
