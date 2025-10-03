use std::iter;

use smallvec::SmallVec;

use crate::ast::*;

/// Preorder DFS traversal over `AnnProc`.
pub(crate) struct PreorderDfsIter<'a, const S: usize> {
    stack: SmallVec<[&'a AnnProc<'a>; S]>,
}

impl<'a, const S: usize> PreorderDfsIter<'a, S> {
    /// Start traversal from the given root.
    pub(crate) fn new(root: &'a AnnProc<'a>) -> Self {
        let mut stack = SmallVec::new();
        stack.push(root);
        Self { stack }
    }

    #[inline]
    fn push_pair(&mut self, left: &'a AnnProc<'a>, right: &'a AnnProc<'a>) {
        self.stack.push(right);
        self.stack.push(left);
    }
}

/// Helper: extract inputs from `ForComprehension` receipts.
fn for_comprehension_inputs<'a>(
    receipts: &'a [Receipt<'a>],
) -> impl DoubleEndedIterator<Item = &'a AnnProc<'a>> {
    receipts
        .iter()
        .flat_map(|bindings| bindings.iter())
        .filter_map(|binding| match binding {
            Bind::Linear {
                rhs: Source::SendReceive { inputs, .. },
                ..
            } => Some(inputs),
            _ => None,
        })
        .flat_map(|inputs| inputs.iter())
}

/// Helper: extract expression + cases from `Match`.
fn match_cases<'a>(cases: &'a [Case<'a>]) -> impl DoubleEndedIterator<Item = &'a AnnProc<'a>> {
    cases
        .iter()
        .flat_map(|case| iter::once(&case.pattern).chain(iter::once(&case.proc)))
}

/// Helper: extract inputs + branch body from `Select`.
fn select_branches<'a>(
    branches: &'a [Branch<'a>],
) -> impl DoubleEndedIterator<Item = &'a AnnProc<'a>> {
    branches.iter().flat_map(|branch| {
        branch
            .patterns
            .iter()
            .filter_map(|ptrn| match &ptrn.rhs {
                Source::SendReceive { inputs, .. } => Some(inputs),
                _ => None,
            })
            .flat_map(|inputs| inputs.iter())
            .chain(iter::once(&branch.proc))
    })
}

/// Helper: extract key–value children from `Collection::Map`.
fn map_elements<'a>(
    elements: &'a [(AnnProc<'a>, AnnProc<'a>)],
) -> impl DoubleEndedIterator<Item = &'a AnnProc<'a>> {
    elements
        .iter()
        .flat_map(|(k, v)| iter::once(k).chain(iter::once(v)))
}

impl<'a, const S: usize> Iterator for PreorderDfsIter<'a, S> {
    type Item = &'a AnnProc<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.stack.pop()?;

        // push children in reverse for left-to-right order
        match node.proc {
            Proc::Par { left, right } | Proc::BinaryExp { left, right, .. } => {
                self.push_pair(left, right);
            }

            Proc::ForComprehension { receipts, proc } => {
                self.stack.push(proc);
                self.stack.extend(for_comprehension_inputs(receipts).rev());
            }

            Proc::Let { bindings, body, .. } => {
                for binding in bindings.iter().rev() {
                    match binding {
                        LetBinding::Single { lhs: _, rhs } => {
                            self.stack.push(rhs);
                        }
                        LetBinding::Multiple { lhs: _, rhs } => {
                            self.stack.extend(rhs.iter().rev());
                        }
                    }
                }
                self.stack.push(body);
            }

            Proc::Contract { body, .. } => {
                self.stack.push(body);
            }

            Proc::New { proc, .. } | Proc::Bundle { proc, .. } => {
                self.stack.push(proc);
            }

            Proc::Send { inputs, .. } | Proc::SendSync { inputs, .. } => {
                self.stack.extend(inputs.iter().rev());
            }

            Proc::Match { expression, cases } => {
                self.stack.extend(match_cases(cases).rev());
                self.stack.push(expression);
            }

            Proc::IfThenElse {
                condition,
                if_true,
                if_false,
            } => {
                if let Some(proc) = if_false {
                    self.stack.push(proc);
                }
                self.push_pair(condition, if_true);
            }

            Proc::Method { receiver, args, .. } => {
                self.stack.extend(args.iter().rev());
                self.stack.push(receiver);
            }

            Proc::Collection(collection) => match collection {
                Collection::List { elements, .. }
                | Collection::Set { elements, .. }
                | Collection::Pathmap { elements, .. }
                | Collection::Tuple(elements) => {
                    self.stack.extend(elements.iter().rev());
                }
                Collection::Map { elements, .. } => {
                    self.stack.extend(map_elements(elements).rev());
                }
            },

            Proc::UnaryExp { arg, .. } => {
                self.stack.push(arg);
            }

            Proc::PathmapDrop { pathmap, .. } => {
                self.stack.push(pathmap);
            }

            Proc::Select { branches } => {
                self.stack.extend(select_branches(branches).rev());
            }

            // leaves
            Proc::Nil
            | Proc::Unit
            | Proc::BoolLiteral(_)
            | Proc::LongLiteral(_)
            | Proc::StringLiteral(_)
            | Proc::UriLiteral(_)
            | Proc::SimpleType(_)
            | Proc::ProcVar(_)
            | Proc::Eval { .. }
            | Proc::VarRef { .. }
            | Proc::Bad => {}
        }

        Some(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourcePos, SourceSpan, ast::Proc, parser::ast_builder::ASTBuilder};
    use pretty_assertions::{assert_eq, assert_matches};
    use smallvec::smallvec;

    #[test]
    fn single_leaf() {
        let root = Proc::Nil.ann(SourcePos::default().span_of(3));
        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();

        assert_eq!(nodes.len(), 1);
        assert_matches!(nodes[0].proc, Proc::Nil);
    }

    #[test]
    fn binary_tree() {
        let left = Proc::BoolLiteral(true).ann(SourcePos::default().span_of(5));
        let right = Proc::BoolLiteral(false).ann(SourcePos::at_col(9).span_of(5));
        let par = Proc::Par { left, right };
        let root = par.ann(SourceSpan {
            start: left.span.start,
            end: right.span.end,
        });

        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();

        assert_eq!(nodes.len(), 3);
        // preorder: root → left → right
        assert_matches!(nodes[0].proc, Proc::Par { .. });
        assert_matches!(nodes[1].proc, Proc::BoolLiteral(true));
        assert_matches!(nodes[2].proc, Proc::BoolLiteral(false));
    }

    #[test]
    fn nested_let_and_body() {
        // let x = 42 in ()
        let rhs = Proc::LongLiteral(42).ann(SourcePos::at_col(9).span_of(2));
        let body = Proc::Unit.ann(SourcePos::at_col(15).span_of(2));
        let x = Id {
            name: "x",
            pos: SourcePos::at_col(5),
        };
        let binding = LetBinding::Single { lhs: x.into(), rhs };
        let let_proc = Proc::Let {
            bindings: smallvec![binding],
            body,
            concurrent: false,
        };
        let root = let_proc.ann(SourceSpan {
            start: SourcePos::default(),
            end: body.span.end,
        });

        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();

        assert_eq!(nodes.len(), 3);
        // preorder: root → body → binding.rhs
        assert_matches!(nodes[0].proc, Proc::Let { .. });
        assert_matches!(nodes[1].proc, Proc::Unit);
        assert_matches!(nodes[2].proc, Proc::LongLiteral(42));
    }

    #[test]
    fn if_then_else_full() {
        // if (true) "yes" else "no"
        let cond = Proc::BoolLiteral(true).ann(SourcePos::at_col(5).span_of(5));
        let if_true = Proc::StringLiteral("yes").ann(SourcePos::at_col(11).span_of(5));
        let if_false = Proc::StringLiteral("no").ann(SourcePos::at_col(22).span_of(4));
        let if_then_else = Proc::IfThenElse {
            condition: cond,
            if_true,
            if_false: Some(if_false),
        };
        let root = if_then_else.ann(SourceSpan {
            start: SourcePos::default(),
            end: if_false.span.end,
        });

        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();

        assert_eq!(nodes.len(), 4);
        // preorder: root → cond → if_true → if_false
        assert_matches!(nodes[0].proc, Proc::IfThenElse { .. });
        assert_matches!(nodes[1].proc, Proc::BoolLiteral(true));
        assert_matches!(nodes[2].proc, Proc::StringLiteral("yes"));
        assert_matches!(nodes[3].proc, Proc::StringLiteral("no"));
    }

    #[test]
    fn collection_map() {
        // { "k1": 1, "k2": 2 }
        let k1 = Proc::StringLiteral("k1").ann(SourcePos::at_col(3).span_of(4));
        let v1 = Proc::LongLiteral(1).ann(SourcePos::at_col(9).span_of(1));
        let k2 = Proc::StringLiteral("k2").ann(SourcePos::at_col(12).span_of(4));
        let v2 = Proc::LongLiteral(2).ann(SourcePos::at_col(18).span_of(1));
        let map = Proc::Collection(Collection::Map {
            elements: vec![(k1, v1), (k2, v2)],
            remainder: None,
        });
        let root = map.ann(SourcePos::default().span_of(20));

        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();

        assert_eq!(nodes.len(), 5);
        // preorder: root → k1 → v1 → k2 → v2
        assert_matches!(nodes[0].proc, Proc::Collection(Collection::Map { .. }));
        assert_matches!(nodes[1].proc, Proc::StringLiteral("k1"));
        assert_matches!(nodes[2].proc, Proc::LongLiteral(1));
        assert_matches!(nodes[3].proc, Proc::StringLiteral("k2"));
        assert_matches!(nodes[4].proc, Proc::LongLiteral(2));
    }

    #[test]
    fn mixed_tree() {
        // true | { let z = 7 in () }
        let leaf1 = Proc::BoolLiteral(true);
        let leaf2 = Proc::LongLiteral(7);
        let z = Id {
            name: "z",
            pos: SourcePos::at_col(14),
        };
        let binding = LetBinding::Single {
            lhs: z.into(),
            rhs: leaf2.ann(SourcePos::at_col(18).span_of(1)),
        };
        let let_body = Proc::Unit.ann(SourcePos::at_col(23).span_of(2));
        let let_proc = Proc::Let {
            bindings: smallvec![binding],
            body: let_body,
            concurrent: false,
        };
        let par_proc = Proc::Par {
            left: leaf1.ann(SourcePos::default().span_of(5)),
            right: let_proc.ann(SourceSpan {
                start: SourcePos::at_col(10),
                end: let_body.span.end,
            }),
        };
        let root = par_proc.ann(SourcePos::default().span_of(26));

        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();

        assert_eq!(nodes.len(), 5);
        // preorder: par → left → right → let body → leaf2
        assert_matches!(nodes[0].proc, Proc::Par { .. });
        assert_matches!(nodes[1].proc, Proc::BoolLiteral(true));
        assert_matches!(nodes[2].proc, Proc::Let { .. });
        assert_matches!(nodes[3].proc, Proc::Unit);
        assert_matches!(nodes[4].proc, Proc::LongLiteral(7));
    }

    #[test]
    fn nested_for_comprehension() {
        // for( y <- z!?(99, { for ( x <- z!?(42) ) { 42 } }) ) { 99 }
        let inner_rhs = Proc::LongLiteral(42);
        let x = Id {
            name: "x",
            pos: SourcePos::at_col(27),
        };
        let inner_bind = Bind::Linear {
            lhs: Names::single(x.into()),
            rhs: Source::SendReceive {
                name: Id {
                    name: "z",
                    pos: SourcePos::at_col(32),
                }
                .into(),
                inputs: smallvec![inner_rhs.ann(SourcePos::at_col(36).span_of(2))],
            },
        };

        let inner_proc = Proc::ForComprehension {
            receipts: smallvec![smallvec![inner_bind]],
            proc: inner_rhs.ann(SourcePos::at_col(44).span_of(2)),
        };

        let outer_rhs = Proc::LongLiteral(99);
        let y = Id {
            name: "y",
            pos: SourcePos::at_col(6),
        };
        let outer_bind = Bind::Linear {
            lhs: Names::single(y.into()),
            rhs: Source::SendReceive {
                name: Id {
                    name: "z",
                    pos: SourcePos::at_col(11),
                }
                .into(),
                inputs: smallvec![
                    outer_rhs.ann(SourcePos::at_col(15).span_of(2)),
                    inner_proc.ann(SourcePos::at_col(21).span_of(27))
                ],
            },
        };

        let outer_proc = Proc::ForComprehension {
            receipts: smallvec![smallvec![outer_bind]],
            proc: outer_rhs.ann(SourcePos::at_col(56).span_of(2)),
        };

        let root = outer_proc.ann(SourcePos::default().span_of(59));

        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();

        assert_eq!(nodes.len(), 6);
        // ensure the outermost node is root
        assert_matches!(nodes[0].proc, Proc::ForComprehension { .. });
        // then comes outer_rhs in preorder
        assert_matches!(nodes[1].proc, Proc::LongLiteral(99));
        assert_matches!(nodes[2].proc, Proc::ForComprehension { .. });
        // then comes inner_rhs
        assert_matches!(nodes[3].proc, Proc::LongLiteral(42));
        assert_matches!(nodes[4].proc, Proc::LongLiteral(42));
        // and the body
        // the traversal includes all inputs and procs in preorder
        assert_matches!(nodes[5].proc, Proc::LongLiteral(99));
    }

    #[test]
    fn deep_unary_chain() {
        // Create a chain of 1000 unary nodes
        let arena = ASTBuilder::with_capacity(1001);
        let mut node = arena.const_nil();
        node = arena
            .chain(node, |node| {
                Some(Proc::UnaryExp {
                    op: UnaryExpOp::Negation,
                    arg: AnnProc {
                        proc: node,
                        span: SourceSpan::default(),
                    },
                })
            })
            .take(1000)
            .last()
            .unwrap();

        let root = node.ann(SourceSpan::default());
        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();

        assert_eq!(nodes.len(), 1001);
        // first node is the outermost UnaryExp
        assert_matches!(nodes[0].proc, Proc::UnaryExp { .. });
        // last node is the leaf
        assert_matches!(nodes.last().unwrap().proc, Proc::Nil);
    }
}
