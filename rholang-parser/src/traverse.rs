use std::iter::{self, FusedIterator};

use smallvec::{SmallVec, smallvec};

use crate::ast::*;

/// Preorder DFS traversal over `AnnProc`.
pub(crate) struct PreorderDfsIter<'a, const S: usize> {
    stack: SmallVec<[&'a AnnProc<'a>; S]>,
}

impl<'a, const S: usize> PreorderDfsIter<'a, S> {
    /// Start traversal from the given root.
    pub(crate) fn new(root: &'a AnnProc<'a>) -> Self {
        Self {
            stack: smallvec![root],
        }
    }

    #[inline]
    fn push_pair(&mut self, left: &'a AnnProc<'a>, right: &'a AnnProc<'a>) {
        self.stack.push(right);
        self.stack.push(left);
    }

    #[inline]
    fn push_name(&mut self, name: &'a Name<'a>) {
        if let Name::Quote(quoted) = name {
            self.stack.push(quoted);
        }
    }

    fn remember<I: IntoIterator<Item = &'a AnnProc<'a>, IntoIter: DoubleEndedIterator>>(
        &mut self,
        nodes: I,
    ) {
        self.stack.extend(nodes.into_iter().rev());
    }
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
                self.remember(for_comprehension_inputs(receipts));
                self.stack.push(proc);
            }

            Proc::Let { bindings, body, .. } => {
                self.remember(let_rhss(bindings));
                self.stack.push(body);
            }

            Proc::Contract { name, body, .. } => {
                self.stack.push(body);
                self.push_name(name);
            }
            Proc::New { proc: inner, .. }
            | Proc::Bundle { proc: inner, .. }
            | Proc::UnaryExp { arg: inner, .. } => {
                self.stack.push(inner);
            }

            Proc::Send {
                inputs, channel, ..
            } => {
                self.remember(inputs);
                self.push_name(channel);
            }
            Proc::SendSync {
                channel,
                inputs,
                cont,
                ..
            } => {
                if let SyncSendCont::NonEmpty(proc) = cont {
                    self.stack.push(proc);
                }
                self.remember(inputs);
                self.push_name(channel);
            }

            Proc::Match { expression, cases } => {
                self.remember(match_cases(cases));
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
                self.remember(args);
                self.stack.push(receiver);
            }

            Proc::Collection(collection) => match collection {
                Collection::List { elements, .. }
                | Collection::Set { elements, .. }
                | Collection::Tuple(elements) => {
                    self.remember(elements);
                }
                Collection::Map { elements, .. } => {
                    self.remember(map_elements(elements));
                }
            },

            Proc::Eval { name } => {
                self.push_name(name);
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
            | Proc::VarRef { .. }
            | Proc::Bad => {}

            Proc::Select { .. } => {
                unimplemented!("Select is not implemented in this version of Rholang")
            }
        }

        Some(node)
    }
}

impl<'a, const S: usize> FusedIterator for PreorderDfsIter<'a, S> {}

#[derive(Clone, Copy, Debug)]
pub enum DfsEvent<'a> {
    Enter(&'a AnnProc<'a>),
    Exit(&'a AnnProc<'a>),
}

/*
`PreorderDfsIter`` can be implemented in terms of this iterator. But it might not be worth it:

1. The `DfsEventIter`` adds an extra push for each node (`Exit` scheduling) and a branch per iteration.
Current `PreorderDfsIter` is very branch-efficient — the CPU will love it in tight traversal code.
So if you’re in a hot path and don’t need the Exit events, the hand-tuned version could be slightly faster.

2. Inlining and branch prediction
Current PreorderDfsIter is straightforward enough to inline well; wrapping it in another iterator layer might inhibit some of that in release builds.
 */
pub(crate) struct DfsEventIter<'a, const S: usize> {
    stack: SmallVec<[Frame<'a>; S]>,
}

/// Stack item: either a node to expand, or an already-built event.
#[derive(Copy, Clone)]
enum Frame<'a> {
    Node(&'a AnnProc<'a>),
    Event(DfsEvent<'a>),
}

impl<'a, const S: usize> DfsEventIter<'a, S> {
    pub fn new(root: &'a AnnProc<'a>) -> Self {
        Self {
            stack: smallvec![Frame::Node(root)],
        }
    }

    /// Push children of `node` as `Node(child)` in reverse order so they are visited left->right.
    #[inline]
    fn push_children<I: IntoIterator<Item = &'a AnnProc<'a>, IntoIter: DoubleEndedIterator>>(
        &mut self,
        children: I,
    ) {
        for child in children.into_iter().rev() {
            self.stack.push(Frame::Node(child));
        }
    }

    fn expand_node_naked(&mut self, node: &'a AnnProc<'a>) {
        match node.proc {
            Proc::Par { left, right } | Proc::BinaryExp { left, right, .. } => {
                self.push_children([left, right]);
            }

            Proc::ForComprehension { receipts, proc } => {
                self.push_children(iter::once(proc).chain(for_comprehension_inputs(receipts)));
            }

            Proc::Let { bindings, body, .. } => {
                self.push_children(iter::once(body).chain(let_rhss(bindings)));
            }

            Proc::Contract { name, body, .. } => {
                // pattern/name may be a quoted proc, include it before body if present
                let quoted = match name {
                    Name::Quote(q) => Some(q),
                    _ => None,
                };
                self.push_children(quoted.into_iter().chain(iter::once(body)));
            }

            // --- Important: CASE handling ---
            Proc::Match { expression, cases } => {
                // Visit the match expression first
                // Then for each case we must treat the case node as the *parent* of its body:
                // order: Enter(case) -> pattern children -> Enter(body) ... Exit(body) -> Exit(case)
                // so children of Match node are: expression, then for each case we push the Case node,
                // and when the Case node is expanded it should push its pattern children then its body.
                // Here we treat Case itself as an AnnProc-like node whose children() returns [pattern*, body].

                // Exit(Match) will be pushed by expand_node()
                // expand children manually:
                for case in cases.iter().rev() {
                    let Case {
                        pattern,
                        proc: body,
                    } = case;

                    // pattern begins
                    self.stack.push(Frame::Event(DfsEvent::Exit(pattern)));
                    // now push body expansion (normal node)
                    self.expand_node(body);
                    // expand pattern children inline
                    self.expand_node_naked(pattern);
                    // pattern enter
                    self.stack.push(Frame::Event(DfsEvent::Enter(pattern)));
                }
                self.push_children(iter::once(expression));

                // Enter(Match) will be pushed by expand_node()
            }

            Proc::IfThenElse {
                condition,
                if_true,
                if_false: None,
            } => {
                self.push_children([condition, if_true]);
            }
            Proc::IfThenElse {
                condition,
                if_true,
                if_false: Some(if_false),
            } => {
                self.push_children([condition, if_true, if_false]);
            }

            Proc::New { proc: inner, .. }
            | Proc::Bundle { proc: inner, .. }
            | Proc::UnaryExp { arg: inner, .. } => {
                self.push_children(iter::once(inner));
            }

            Proc::Send {
                inputs, channel, ..
            } => {
                let quoted = match channel {
                    Name::Quote(q) => Some(q),
                    _ => None,
                };
                self.push_children(quoted.into_iter().chain(inputs));
            }
            Proc::SendSync {
                channel,
                inputs,
                cont,
                ..
            } => {
                let quoted = match channel {
                    Name::Quote(q) => Some(q),
                    _ => None,
                };
                let cont_iter = match cont {
                    SyncSendCont::NonEmpty(p) => Some(p),
                    _ => None,
                };
                self.push_children(
                    quoted
                        .into_iter()
                        .chain(inputs)
                        .chain(cont_iter.into_iter()),
                );
            }

            Proc::Method { receiver, args, .. } => {
                self.push_children(iter::once(receiver).chain(args));
            }

            Proc::Collection(collection) => match collection {
                Collection::List { elements, .. }
                | Collection::Set { elements, .. }
                | Collection::Tuple(elements) => {
                    self.push_children(elements);
                }
                Collection::Map { elements, .. } => {
                    self.push_children(map_elements(elements));
                }
            },

            Proc::Eval { name } => {
                if let Name::Quote(q) = name {
                    self.push_children(iter::once(q));
                }
            }

            // leaves: no children
            Proc::Nil
            | Proc::Unit
            | Proc::BoolLiteral(_)
            | Proc::LongLiteral(_)
            | Proc::StringLiteral(_)
            | Proc::UriLiteral(_)
            | Proc::SimpleType(_)
            | Proc::ProcVar(_)
            | Proc::VarRef { .. }
            | Proc::Bad => {}

            Proc::Select { .. } => {
                unimplemented!("Select is not implemented in this version of Rholang")
            }
        }
    }

    /// Expand a node by pushing Exit(node), its children (Node(...)), and Enter(node).
    fn expand_node(&mut self, node: &'a AnnProc<'a>) {
        // push Exit first (it should be at bottom)
        self.stack.push(Frame::Event(DfsEvent::Exit(node)));

        self.expand_node_naked(node);

        // finally push Enter on top so it's the next popped item
        self.stack.push(Frame::Event(DfsEvent::Enter(node)));
    }
}

impl<'a, const S: usize> Iterator for DfsEventIter<'a, S> {
    type Item = DfsEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.stack.pop() {
            match item {
                Frame::Event(ev) => return Some(ev),
                Frame::Node(node) => {
                    // expand node into Event::Enter, Node(children), Event::Exit
                    self.expand_node(node);
                    // and loop: the next iteration will pop the Enter event we just pushed
                    continue;
                }
            }
        }
        None
    }
}

/// Helper: extract right-hand sides of let bindings
fn let_rhss<'a>(
    bindings: &'a [LetBinding<'a>],
) -> impl DoubleEndedIterator<Item = &'a AnnProc<'a>> {
    bindings.iter().flat_map(|binding| &binding.rhs)
}

/// Helper: extract sources + their inputs from `ForComprehension` receipts.
fn for_comprehension_inputs<'a>(
    receipts: &'a [Receipt<'a>],
) -> impl DoubleEndedIterator<Item = &'a AnnProc<'a>> {
    receipts.iter().flatten().flat_map(|binding| {
        let name_proc = if let Name::Quote(quoted) = binding.source_name() {
            Some(quoted)
        } else {
            None
        };
        let quoted_iter = name_proc.into_iter();
        let input_iter = binding.input().into_iter().flatten();
        quoted_iter.chain(input_iter)
    })
}

/// Helper: extract expression + cases from `Match`.
fn match_cases<'a>(cases: &'a [Case<'a>]) -> impl DoubleEndedIterator<Item = &'a AnnProc<'a>> {
    cases
        .iter()
        .flat_map(|case| iter::once(&case.pattern).chain(iter::once(&case.proc)))
}

// /// Helper: extract inputs + branch body from `Select`.
// fn select_branches<'a>(
//     branches: &'a [Branch<'a>],
// ) -> impl DoubleEndedIterator<Item = &'a AnnProc<'a>> {
//     branches.iter().flat_map(|branch| {
//         branch
//             .patterns
//             .iter()
//             .filter_map(|ptrn| match &ptrn.rhs {
//                 Source::SendReceive { inputs, .. } => Some(inputs),
//                 _ => None,
//             })
//             .flatten()
//             .chain(iter::once(&branch.proc))
//     })
// }

/// Helper: extract key–value children from `Collection::Map`.
fn map_elements<'a>(
    elements: &'a [(AnnProc<'a>, AnnProc<'a>)],
) -> impl DoubleEndedIterator<Item = &'a AnnProc<'a>> {
    elements
        .iter()
        .flat_map(|(k, v)| iter::once(k).chain(iter::once(v)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourcePos, SourceSpan, ast::AnnProc, ast::Proc, parser::ast_builder::ASTBuilder};
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

        // events
        let events: Vec<_> = (&root).iter_dfs_event().collect();
        assert_matches!(
            events.as_slice(),
            [
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Par { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::BoolLiteral(true),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::BoolLiteral(true),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::BoolLiteral(false),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::BoolLiteral(false),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Par { .. },
                    ..
                })
            ]
        );
    }

    #[test]
    fn nested_let_and_body() {
        // let x <- 42 in ()
        let rhs = Proc::LongLiteral(42).ann(SourcePos::at_col(10).span_of(2));
        let body = Proc::Unit.ann(SourcePos::at_col(16).span_of(2));
        let x = Id {
            name: "x",
            pos: SourcePos::at_col(5),
        };
        let binding = LetBinding::single(x.into(), rhs);
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

        // events
        let events: Vec<_> = (&root).iter_dfs_event().collect();
        assert_matches!(
            events.as_slice(),
            [
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Let { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Unit,
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Unit,
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::LongLiteral(42),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::LongLiteral(42),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Let { .. },
                    ..
                })
            ]
        );
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

        let events: Vec<_> = (&root).iter_dfs_event().collect();
        assert_matches!(
            events.as_slice(),
            [
                DfsEvent::Enter(AnnProc {
                    proc: Proc::IfThenElse { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::BoolLiteral(true),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::BoolLiteral(true),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::StringLiteral("yes"),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::StringLiteral("yes"),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::StringLiteral("no"),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::StringLiteral("no"),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::IfThenElse { .. },
                    ..
                })
            ]
        );
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

        let events: Vec<_> = (&root).iter_dfs_event().collect();
        assert_matches!(
            events.as_slice(),
            [
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Collection(Collection::Map { .. }),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::StringLiteral("k1"),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::StringLiteral("k1"),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::LongLiteral(1),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::LongLiteral(1),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::StringLiteral("k2"),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::StringLiteral("k2"),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::LongLiteral(2),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::LongLiteral(2),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Collection(Collection::Map { .. }),
                    ..
                })
            ]
        );
    }

    #[test]
    fn mixed_tree() {
        // true | { let z <- 7 in () }
        let leaf1 = Proc::BoolLiteral(true);
        let leaf2 = Proc::LongLiteral(7);
        let z = Id {
            name: "z",
            pos: SourcePos::at_col(14),
        };
        let binding = LetBinding::single(z.into(), leaf2.ann(SourcePos::at_col(19).span_of(1)));
        let let_body = Proc::Unit.ann(SourcePos::at_col(24).span_of(2));
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
        let root = par_proc.ann(SourcePos::default().span_of(27));

        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();

        assert_eq!(nodes.len(), 5);
        // preorder: par → left → right → let body → leaf2
        assert_matches!(nodes[0].proc, Proc::Par { .. });
        assert_matches!(nodes[1].proc, Proc::BoolLiteral(true));
        assert_matches!(nodes[2].proc, Proc::Let { .. });
        assert_matches!(nodes[3].proc, Proc::Unit);
        assert_matches!(nodes[4].proc, Proc::LongLiteral(7));

        let events: Vec<_> = (&root).iter_dfs_event().collect();
        assert_matches!(
            events.as_slice(),
            [
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Par { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::BoolLiteral(true),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::BoolLiteral(true),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Let { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Unit,
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Unit,
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::LongLiteral(7),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::LongLiteral(7),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Let { .. },
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Par { .. },
                    ..
                })
            ]
        );
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
        // then the body
        assert_matches!(nodes[1].proc, Proc::LongLiteral(99));
        // the traversal includes all inputs and procs in preorder
        // then comes outer_rhs in preorder
        assert_matches!(nodes[2].proc, Proc::LongLiteral(99));
        assert_matches!(nodes[3].proc, Proc::ForComprehension { .. });
        // then comes inner_rhs
        assert_matches!(nodes[4].proc, Proc::LongLiteral(42));
        assert_matches!(nodes[5].proc, Proc::LongLiteral(42));

        let events: Vec<_> = (&root).iter_dfs_event().collect();
        assert_matches!(
            events.as_slice(),
            [
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ForComprehension { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::LongLiteral(99),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::LongLiteral(99),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::LongLiteral(99),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::LongLiteral(99),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ForComprehension { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::LongLiteral(42),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::LongLiteral(42),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::LongLiteral(42),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::LongLiteral(42),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ForComprehension { .. },
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ForComprehension { .. },
                    ..
                })
            ]
        );
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

    #[test]
    fn quoted_names() {
        /*
        for (_ <- @{arg1 | *table}) {
          @{arg1 | *table}!(arg2) |
          ack!(true)
        }
        */

        let arg1_in_bind = Proc::ProcVar(Var::Id(Id {
            name: "arg1",
            pos: SourcePos::at_col(13),
        }));
        let eval_table_in_bind = Proc::Eval {
            name: Id {
                name: "table",
                pos: SourcePos::at_col(21),
            }
            .into(),
        };
        let par_in_bind = Proc::Par {
            left: arg1_in_bind.ann(SourcePos::at_col(13).span_of(4)),
            right: eval_table_in_bind.ann(SourcePos::at_col(20).span_of(6)),
        };

        let bind = Bind::Linear {
            lhs: Names::single(Name::NameVar(Var::Wildcard)),
            rhs: Source::Simple {
                name: Name::Quote(par_in_bind.ann(SourcePos::at_col(12).span_of(15))),
            },
        };

        let arg1_in_send = Proc::ProcVar(Var::Id(Id {
            name: "arg1",
            pos: SourcePos { line: 2, col: 5 },
        }));
        let eval_table_in_send = Proc::Eval {
            name: Id {
                name: "table",
                pos: SourcePos { line: 2, col: 13 },
            }
            .into(),
        };
        let par_in_send = Proc::Par {
            left: arg1_in_send.ann(SourcePos::at_col(13).span_of(4)),
            right: eval_table_in_send.ann(SourcePos { line: 2, col: 12 }.span_of(6)),
        };

        let arg2 = Proc::ProcVar(Var::Id(Id {
            name: "arg2",
            pos: SourcePos { line: 2, col: 21 },
        }));
        let first_send = Proc::Send {
            channel: Name::Quote(par_in_send.ann(SourcePos { line: 2, col: 4 }.span_of(15))),
            send_type: SendType::Single,
            inputs: smallvec![arg2.ann(SourcePos { line: 2, col: 21 }.span_of(4))],
        };

        let true_lit = Proc::BoolLiteral(true);
        let second_send = Proc::Send {
            channel: Name::NameVar(Var::Id(Id {
                name: "ack",
                pos: SourcePos { line: 3, col: 3 },
            })),
            send_type: SendType::Single,
            inputs: smallvec![true_lit.ann(SourcePos { line: 3, col: 8 }.span_of(4))],
        };

        let for_body = Proc::Par {
            left: first_send.ann(SourcePos { line: 2, col: 3 }.span_of(23)),
            right: second_send.ann(SourcePos { line: 3, col: 3 }.span_of(10)),
        };

        let for_comprehension = Proc::ForComprehension {
            receipts: smallvec![smallvec![bind]],
            proc: for_body.ann(SourceSpan {
                start: SourcePos { line: 1, col: 29 },
                end: SourcePos { line: 4, col: 2 },
            }),
        };

        let root = for_comprehension.ann(SourceSpan {
            start: SourcePos::default(),
            end: SourcePos { line: 4, col: 1 },
        });

        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();

        assert_eq!(nodes.len(), 12);
        // preorder: for →
        //             for body → par → left send → quote → (par → arg → eval) → right send  →
        //           quote → (par → arg → eval)
        assert_matches!(nodes[0].proc, Proc::ForComprehension { .. });
        assert_matches!(nodes[1].proc, Proc::Par { .. });
        assert_matches!(nodes[2].proc, Proc::Send { .. });
        assert_matches!(nodes[3].proc, Proc::Par { .. });
        assert_matches!(
            nodes[4].proc,
            Proc::ProcVar(Var::Id(Id { name: "arg1", .. }))
        );
        assert_matches!(nodes[5].proc, Proc::Eval { .. });
        assert_matches!(
            nodes[6].proc,
            Proc::ProcVar(Var::Id(Id { name: "arg2", .. }))
        );
        assert_matches!(nodes[7].proc, Proc::Send { .. });
        assert_matches!(nodes[8].proc, Proc::BoolLiteral(true));
        assert_matches!(nodes[9].proc, Proc::Par { .. });
        assert_matches!(
            nodes[10].proc,
            Proc::ProcVar(Var::Id(Id { name: "arg1", .. }))
        );
        assert_matches!(nodes[11].proc, Proc::Eval { .. });

        let events: Vec<_> = (&root).iter_dfs_event().collect();
        assert_matches!(
            events.as_slice(),
            [
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ForComprehension { .. },
                    ..
                }),
                // Enter(for body)
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Par { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Send { .. },
                    ..
                }),
                // Enter (quote)
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Par { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "arg1", .. })),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "arg1", .. })),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Eval { .. },
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Eval { .. },
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Par { .. },
                    ..
                }),
                // Exit (quote)
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "arg2", .. })),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "arg2", .. })),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Send { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Send { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::BoolLiteral(true),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::BoolLiteral(true),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Send { .. },
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Par { .. },
                    ..
                }),
                // Exit(for body)

                // Enter (quote)
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Par { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "arg1", .. })),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "arg1", .. })),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Eval { .. },
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Eval { .. },
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Par { .. },
                    ..
                }),
                // Exit (quote)
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ForComprehension { .. },
                    ..
                })
            ]
        );
    }

    #[test]
    fn match_expression() {
        /* match x { p1 => y1; p2 => y2 } */
        let x = Proc::ProcVar(Var::Id(Id {
            name: "x",
            pos: SourcePos::at_col(7),
        }));
        let p1 = Proc::ProcVar(Var::Id(Id {
            name: "p1",
            pos: SourcePos::at_col(11),
        }));
        let y1 = Proc::ProcVar(Var::Id(Id {
            name: "y1",
            pos: SourcePos::at_col(17),
        }));
        let p2 = Proc::ProcVar(Var::Id(Id {
            name: "p2",
            pos: SourcePos::at_col(21),
        }));
        let y2 = Proc::ProcVar(Var::Id(Id {
            name: "y2",
            pos: SourcePos::at_col(27),
        }));

        let match_exp = Proc::Match {
            expression: x.ann(SourcePos::at_col(7).span_of(1)),
            cases: vec![
                Case {
                    pattern: p1.ann(SourcePos::at_col(11).span_of(2)),
                    proc: y1.ann(SourcePos::at_col(17).span_of(2)),
                },
                Case {
                    pattern: p2.ann(SourcePos::at_col(21).span_of(2)),
                    proc: y2.ann(SourcePos::at_col(27).span_of(2)),
                },
            ],
        };
        let root = match_exp.ann(SourceSpan {
            start: SourcePos::default(),
            end: SourcePos { line: 1, col: 31 },
        });

        let nodes: Vec<_> = (&root).iter_preorder_dfs().collect();
        assert_eq!(nodes.len(), 6);
        assert_matches!(nodes[0].proc, Proc::Match { .. });
        // expression
        assert_matches!(nodes[1].proc, Proc::ProcVar(Var::Id(Id { name: "x", .. })));
        // cases
        assert_matches!(nodes[2].proc, Proc::ProcVar(Var::Id(Id { name: "p1", .. })));
        assert_matches!(nodes[3].proc, Proc::ProcVar(Var::Id(Id { name: "y1", .. })));
        assert_matches!(nodes[4].proc, Proc::ProcVar(Var::Id(Id { name: "p2", .. })));
        assert_matches!(nodes[5].proc, Proc::ProcVar(Var::Id(Id { name: "y2", .. })));

        let events: Vec<_> = (&root).iter_dfs_event().collect();
        assert_matches!(
            events.as_slice(),
            [
                DfsEvent::Enter(AnnProc {
                    proc: Proc::Match { .. },
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "x", .. })),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "x", .. })),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "p1", .. })),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "y1", .. })),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "y1", .. })),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "p1", .. })),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "p2", .. })),
                    ..
                }),
                DfsEvent::Enter(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "y2", .. })),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "y2", .. })),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::ProcVar(Var::Id(Id { name: "p2", .. })),
                    ..
                }),
                DfsEvent::Exit(AnnProc {
                    proc: Proc::Match { .. },
                    ..
                })
            ]
        );
    }
}
