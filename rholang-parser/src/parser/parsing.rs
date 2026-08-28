use bitvec::slice::BitSlice;
use bitvec::vec::BitVec;
use nonempty_collections::NEVec;
use rholang_tree_sitter_proc_macro::{field, kind};
use smallvec::{SmallVec, ToSmallVec};
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::iter::FusedIterator;
use std::slice::Iter as SliceIter;
use validated::Validated;

use crate::SourcePos;
use crate::ast::Name;
use crate::parser::errors::{self, ParsingFailure};
use crate::{
    SourceSpan,
    ast::{
        AnnProc, BinaryExpOp, Bind, BundleType, Id, LetBinding, NameDecl, Names, Proc, SendType,
        Signature, SimpleType, Source, TokenStack, UnaryExpOp, Var, VarRefKind,
    },
    parser::{
        ast_builder::ASTBuilder,
        errors::{AnnParsingError, ParsingError},
    },
};

/// Per-decl metadata collected while walking an `agent_block`'s
/// declarations. Carried in `K::ConsumeAgentBlock` so the consume
/// step can route slice elements (body + formals AnnProcs on
/// `proc_stack`) to the right slot of the desugaring.
#[derive(Debug, Clone)]
pub(super) enum AgentDeclDesc<'ast> {
    Constructor {
        arity: usize,
        has_cont: bool,
    },
    Method {
        name: Id<'ast>,
        arity: usize,
        has_cont: bool,
        is_private: bool,
    },
    Default {
        arity: usize,
        has_cont: bool,
        is_private: bool,
    },
}

impl<'ast> AgentDeclDesc<'ast> {
    fn slice_len(&self) -> usize {
        // body + formals
        match self {
            AgentDeclDesc::Constructor { arity, .. }
            | AgentDeclDesc::Method { arity, .. }
            | AgentDeclDesc::Default { arity, .. } => 1 + arity,
        }
    }
}

pub(super) fn parse_to_tree(source: &str) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    let rholang_language = rholang_tree_sitter::LANGUAGE.into();
    parser
        .set_language(&rholang_language)
        .expect("Error loading Rholang parser");
    parser
        .parse(source, None)
        .expect("Failed to produce syntax tree")
}

pub(super) fn node_to_ast<'ast>(
    start_node: &tree_sitter::Node,
    ast_builder: &'ast ASTBuilder<'ast>,
    source: &'ast str,
) -> Validated<AnnProc<'ast>, ParsingFailure<'ast>> {
    let mut errors = Vec::new();
    let mut proc_stack = ProcStack::new();
    let mut cont_stack = Vec::with_capacity(32);
    // sometimes a small temporary stack is needed - allocate it here so it is re-used
    let mut temp_cont_stack = Vec::new();
    let mut node = *start_node;

    'parse: loop {
        let mut bad = false;

        if node.is_error() || node.is_missing() {
            // the errors will be discovered when parsing is done
            bad = true;
        } else {
            fn eval_named_pairs<'a>(
                of: &tree_sitter::Node<'a>,
                kind: u16,
                fst_selector: u16,
                snd_selector: u16,
                cont_stack: &mut Vec<K<'a, '_>>,
            ) -> usize {
                let mut arity = 0;
                for child in named_children_of_kind(of, kind, &mut of.walk()) {
                    cont_stack.push(K::EvalDelayed(get_field(&child, fst_selector)));
                    cont_stack.push(K::EvalDelayed(get_field(&child, snd_selector)));
                    arity += 1;
                }
                cont_stack.reverse();

                arity
            }

            let span = node.range().into();
            match node.kind_id() {
                kind!("block") => {
                    node = get_first_child(&node);
                    continue 'parse;
                }

                kind!("wildcard") => proc_stack.push(ast_builder.const_wild(), span),
                kind!("var") => {
                    let id = Id {
                        name: get_node_value(&node, source),
                        pos: span.start,
                    };
                    proc_stack.push(ast_builder.alloc_var(id), span);
                }

                kind!("nil") => proc_stack.push(ast_builder.const_nil(), span),
                kind!("unit") => proc_stack.push(ast_builder.const_unit(), span),
                kind!("simple_type") => {
                    let lit_value = get_node_value(&node, source);
                    let simple_type_value = match lit_value {
                        "Bool" => SimpleType::Bool,
                        "Int" => SimpleType::Int,
                        "String" => SimpleType::String,
                        "Uri" => SimpleType::Uri,
                        "ByteArray" => SimpleType::ByteArray,
                        _ => unreachable!(
                            "Simple type is always 'Bool', 'Int', 'String', 'Uri', or 'ByteArray'"
                        ),
                    };
                    proc_stack.push(ast_builder.alloc_simple_type(simple_type_value), span);
                }
                kind!("bool_literal") => {
                    let lit_value = get_node_value(&node, source);
                    let bool_proc = match lit_value {
                        "true" => ast_builder.const_true(),
                        "false" => ast_builder.const_false(),
                        _ => unreachable!("Boolean literal is always 'true' or 'false'"),
                    };
                    proc_stack.push(bool_proc, span);
                }
                kind!("long_literal") => {
                    let lit_value = get_node_value(&node, source);
                    match lit_value.parse::<i64>() {
                        Ok(i64_value) => {
                            proc_stack.push(ast_builder.alloc_long_literal(i64_value), span)
                        }
                        Err(_) => {
                            // the only possibility is pos/neg overflow
                            errors
                                .push(AnnParsingError::new(ParsingError::NumberOutOfRange, &node));
                            bad = true;
                        }
                    }
                }
                kind!("signed_int_literal") => {
                    let lit_value = get_node_value(&node, source);
                    if let Some((value, bits)) = parse_sized_int_literal(lit_value, 'i') {
                        proc_stack.push(ast_builder.alloc_signed_int_literal(value, bits), span);
                    } else {
                        errors.push(AnnParsingError::new(ParsingError::NumberOutOfRange, &node));
                        bad = true;
                    }
                }
                kind!("unsigned_int_literal") => {
                    let lit_value = get_node_value(&node, source);
                    if let Some((value, bits)) = parse_sized_int_literal(lit_value, 'u') {
                        proc_stack.push(ast_builder.alloc_unsigned_int_literal(value, bits), span);
                    } else {
                        errors.push(AnnParsingError::new(ParsingError::NumberOutOfRange, &node));
                        bad = true;
                    }
                }
                kind!("bigint_literal") => {
                    let lit_value = get_node_value(&node, source);
                    if let Some(value) = lit_value.strip_suffix('n') {
                        proc_stack.push(ast_builder.alloc_bigint_literal(value), span);
                    } else {
                        errors.push(AnnParsingError::new(ParsingError::NumberOutOfRange, &node));
                        bad = true;
                    }
                }
                kind!("bigrat_literal") => {
                    let lit_value = get_node_value(&node, source);
                    if let Some(value) = lit_value.strip_suffix('r') {
                        proc_stack.push(ast_builder.alloc_bigrat_literal(value), span);
                    } else {
                        errors.push(AnnParsingError::new(ParsingError::NumberOutOfRange, &node));
                        bad = true;
                    }
                }
                kind!("float_literal") => {
                    let lit_value = get_node_value(&node, source);
                    if let Some((value, bits)) = parse_float_literal(lit_value) {
                        proc_stack.push(ast_builder.alloc_float_literal(value, bits), span);
                    } else {
                        errors.push(AnnParsingError::new(ParsingError::NumberOutOfRange, &node));
                        bad = true;
                    }
                }
                kind!("fixed_point_literal") => {
                    let lit_value = get_node_value(&node, source);
                    if let Some((value, scale)) = parse_fixed_point_literal(lit_value) {
                        proc_stack.push(ast_builder.alloc_fixed_point_literal(value, scale), span);
                    } else {
                        errors.push(AnnParsingError::new(ParsingError::NumberOutOfRange, &node));
                        bad = true;
                    }
                }
                kind!("string_literal") => {
                    let lit_value = get_node_value(&node, source);
                    proc_stack.push(ast_builder.alloc_string_literal(lit_value), span);
                }
                kind!("uri_literal") => {
                    let lit_value = get_node_value(&node, source);
                    proc_stack.push(ast_builder.alloc_uri_literal(lit_value), span);
                }

                kind!("par") => {
                    let (left, right) = get_left_and_right(&node);
                    cont_stack.push(K::ConsumePar { span });
                    cont_stack.push(K::EvalDelayed(right));
                    node = left;
                    continue 'parse;
                }
                kind!("eval") => {
                    cont_stack.push(K::ConsumeEval { span });
                    node = get_first_child(&node);
                    continue 'parse;
                }
                kind!("quote") => {
                    cont_stack.push(K::ConsumeQuote);
                    node = get_first_child(&node);
                    continue 'parse;
                }
                kind!("method") => {
                    let receiver_node = get_field(&node, field!("receiver"));
                    let name_node = get_field(&node, field!("name"));
                    let args_node = get_field(&node, field!("args"));

                    let arity = args_node.named_child_count();
                    cont_stack.push(K::ConsumeMethod {
                        id: Id {
                            name: get_node_value(&name_node, source),
                            pos: name_node.start_position().into(),
                        },
                        arity,
                        span,
                    });

                    if arity > 0 {
                        cont_stack.push(K::EvalList(args_node.walk()));
                    }
                    node = receiver_node;
                    continue 'parse;
                }
                kind!("or")
                | kind!("and")
                | kind!("matches")
                | kind!("eq")
                | kind!("neq")
                | kind!("lt")
                | kind!("lte")
                | kind!("gt")
                | kind!("gte")
                | kind!("concat")
                | kind!("diff")
                | kind!("add")
                | kind!("sub")
                | kind!("interpolation")
                | kind!("mult")
                | kind!("div")
                | kind!("mod")
                | kind!("disjunction")
                | kind!("conjunction") => {
                    let (left, right) = get_left_and_right(&node);
                    cont_stack.push(K::ConsumeBinaryExp {
                        op: match node.kind_id() {
                            kind!("or") => BinaryExpOp::Or,
                            kind!("and") => BinaryExpOp::And,
                            kind!("matches") => BinaryExpOp::Matches,
                            kind!("eq") => BinaryExpOp::Eq,
                            kind!("neq") => BinaryExpOp::Neq,
                            kind!("lt") => BinaryExpOp::Lt,
                            kind!("lte") => BinaryExpOp::Lte,
                            kind!("gt") => BinaryExpOp::Gt,
                            kind!("gte") => BinaryExpOp::Gte,
                            kind!("concat") => BinaryExpOp::Concat,
                            kind!("diff") => BinaryExpOp::Diff,
                            kind!("add") => BinaryExpOp::Add,
                            kind!("sub") => BinaryExpOp::Sub,
                            kind!("interpolation") => BinaryExpOp::Interpolation,
                            kind!("mult") => BinaryExpOp::Mult,
                            kind!("div") => BinaryExpOp::Div,
                            kind!("mod") => BinaryExpOp::Mod,
                            kind!("disjunction") => BinaryExpOp::Disjunction,
                            _ => BinaryExpOp::Conjunction,
                        },
                        span,
                    });
                    cont_stack.push(K::EvalDelayed(right));
                    node = left;
                    continue 'parse;
                }
                kind!("neg") | kind!("not") | kind!("negation") => {
                    let proc_node = get_first_child(&node);
                    cont_stack.push(K::ConsumeUnaryExp {
                        op: match node.kind_id() {
                            kind!("neg") => UnaryExpOp::Neg,
                            kind!("not") => UnaryExpOp::Not,
                            _ => UnaryExpOp::Negation,
                        },
                        span,
                    });
                    node = proc_node;
                    continue 'parse;
                }

                kind!("collection") => {
                    let collection_node = get_first_child(&node);
                    let collection_type = collection_node.kind_id();
                    let is_tuple = collection_type == kind!("tuple");
                    let remainder_node = if is_tuple {
                        None
                    } else {
                        collection_node.child_by_field_id(field!("remainder"))
                    };
                    let has_remainder = remainder_node.is_some();
                    match collection_type {
                        kind!("list") => {
                            let arity = collection_node.named_child_count();
                            if arity == 0 {
                                proc_stack.push(ast_builder.const_empty_list(), span);
                            } else {
                                cont_stack.push(K::ConsumeList {
                                    arity,
                                    has_remainder,
                                    span,
                                });
                                cont_stack.push(K::EvalList(collection_node.walk()));
                            }
                        }
                        kind!("set") => {
                            cont_stack.push(K::ConsumeSet {
                                arity: collection_node.named_child_count(),
                                has_remainder,
                                span,
                            });
                            cont_stack.push(K::EvalList(collection_node.walk()));
                        }
                        kind!("tuple") => {
                            cont_stack.push(K::ConsumeTuple {
                                arity: collection_node.named_child_count(),
                                span,
                            });
                            cont_stack.push(K::EvalList(collection_node.walk()));
                        }
                        kind!("map") => {
                            temp_cont_stack.reserve(collection_node.named_child_count() * 2);
                            let arity = eval_named_pairs(
                                &collection_node,
                                kind!("key_value_pair"),
                                field!("key"),
                                field!("value"),
                                &mut temp_cont_stack,
                            );
                            if arity == 0 {
                                proc_stack.push(ast_builder.const_empty_map(), span);
                            } else {
                                cont_stack.push(K::ConsumeMap {
                                    arity,
                                    has_remainder,
                                    span,
                                });
                                cont_stack.append(&mut temp_cont_stack);
                                if let Some(rem) = remainder_node {
                                    cont_stack.push(K::EvalDelayed(rem));
                                }
                            }
                        }
                        kind!("pathmap") => {
                            cont_stack.push(K::ConsumePathMap {
                                arity: collection_node.named_child_count(),
                                has_remainder,
                                span,
                            });
                            cont_stack.push(K::EvalList(collection_node.walk()));
                        }
                        _ => unreachable!("Rholang collections are: list, set, tuple and map"),
                    }
                }

                kind!("send") => {
                    let name_node = get_field(&node, field!("channel"));
                    let send_type_node = get_field(&node, field!("send_type"));
                    let inputs_node = get_field(&node, field!("inputs"));

                    let send_type = match send_type_node.kind_id() {
                        kind!("send_single") => SendType::Single,
                        kind!("send_multiple") => SendType::Multiple,
                        _ => unreachable!("Send type can only be: single or multiple"),
                    };
                    let arity = inputs_node.named_child_count();
                    cont_stack.push(K::ConsumeSend {
                        send_type,
                        arity,
                        span,
                    });
                    if arity > 0 {
                        cont_stack.push(K::EvalList(inputs_node.walk()));
                    }
                    node = name_node;
                    continue 'parse;
                }

                // Cost-accounted Rholang: signed term {% P %}[ s ].
                kind!("signed_term") => {
                    let proc_node = get_field(&node, field!("proc"));
                    let sig_node = get_field(&node, field!("sig"));

                    let mut ops: SmallVec<[SigOp; 4]> = SmallVec::new();
                    let mut sig_procs: Vec<tree_sitter::Node> = Vec::new();
                    flatten_signature(&sig_node, &mut ops, &mut sig_procs);
                    let sig_arity = sig_procs.len();

                    // Body lands at slice[0] (via `node = proc_node`); signature
                    // procs at slice[1..] in source order (pushed reversed so
                    // they pop in source order).
                    cont_stack.push(K::ConsumeSignedTerm {
                        ops,
                        sig_arity,
                        span,
                    });
                    for n in sig_procs.iter().rev() {
                        cont_stack.push(K::EvalDelayed(*n));
                    }
                    node = proc_node;
                    continue 'parse;
                }

                // Cost-accounted Rholang: a bare token stack `s :: … :: ()` is a
                // signed process (Greg `app:concrete` `Stk`; no `purse(...)`
                // wrapper). Located/parallel stacks are ordinary `Par`; ring-fencing
                // is via `new`-bound signatures (binding-sensitive Σ in the normalizer).
                kind!("token_stack") => {
                    let mut all_ops: SmallVec<[SigOp; 8]> = SmallVec::new();
                    let mut layer_descs: SmallVec<[(usize, usize, usize); 2]> = SmallVec::new();
                    let mut layer_procs: Vec<tree_sitter::Node> = Vec::new();

                    // Walk the cons spine `head :: tail`; the terminal `tail` is a
                    // `unit` (no `head` field). The proc node IS the token_stack.
                    let mut cur = node;
                    while let Some(head_node) = cur.child_by_field_id(field!("head")) {
                        let op_start = all_ops.len();
                        let proc_before = layer_procs.len();
                        flatten_signature(&head_node, &mut all_ops, &mut layer_procs);
                        layer_descs.push((
                            op_start,
                            all_ops.len() - op_start,
                            layer_procs.len() - proc_before,
                        ));
                        cur = get_field(&cur, field!("tail"));
                    }

                    cont_stack.push(K::ConsumeTokenStack {
                        all_ops,
                        layer_descs,
                        total_procs: layer_procs.len(),
                        span,
                    });
                    // Pop order → proc_stack slots: [layer_proc0, layer_proc1, …].
                    for n in layer_procs.iter().rev() {
                        cont_stack.push(K::EvalDelayed(*n));
                    }
                    // Fall through to apply_cont (no body to descend into).
                }

                // Greg `app:concrete` PSignedInput: a signed receive continuation
                // `for(...) {% P %}` — unwrap the `{% %}` to its inner process (the
                // K=Par polymorphism marker carries no extra lowering; the body P is
                // normalized as-is, signed terms inside it included).
                kind!("signed_cont") => {
                    node = get_field(&node, field!("proc"));
                    continue 'parse;
                }

                kind!("new") => {
                    fn check_for_duplicate_decls(
                        decls: &[NameDecl],
                    ) -> Option<(SourcePos, SourcePos)> {
                        // Check for duplicates without requiring sorted input
                        // Use O(n^2) comparison since n is typically small
                        for i in 0..decls.len() {
                            for j in (i + 1)..decls.len() {
                                if decls[i].id.name == decls[j].id.name {
                                    let first = decls[i].id.pos;
                                    let second = decls[j].id.pos;
                                    return Some((first, second));
                                }
                            }
                        }
                        None
                    }

                    let decls_node = get_field(&node, field!("decls"));
                    let proc_node = get_field(&node, field!("proc"));

                    let decls = parse_decls(&decls_node, source);
                    // IMPORTANT: Do NOT sort decls - preserve source order for correct variable indexing
                    // The old parser (rust/dev) and Scala implementation expect source order
                    if let Some((first, second)) = check_for_duplicate_decls(&decls) {
                        errors.push(AnnParsingError::new(
                            ParsingError::DuplicateNameDecl { first, second },
                            &decls_node,
                        ));
                    }

                    cont_stack.push(K::ConsumeNew { decls, span });
                    node = proc_node;
                    continue 'parse;
                }

                kind!("contract") => {
                    let name_node = get_field(&node, field!("name"));
                    let proc_node = get_field(&node, field!("proc"));

                    if let Some(formals_node) = node.child_by_field_id(field!("formals")) {
                        cont_stack.push(K::ConsumeContract {
                            arity: formals_node.named_child_count(),
                            has_cont: formals_node.child_by_field_id(field!("cont")).is_some(),
                            span,
                        });
                        cont_stack.push(K::EvalList(formals_node.walk()));
                    } else {
                        cont_stack.push(K::ConsumeContract {
                            arity: 0,
                            has_cont: false,
                            span,
                        });
                    }
                    cont_stack.push(K::EvalDelayed(proc_node));
                    node = name_node;
                    continue 'parse;
                }

                // Agent block sugar (FIP 2025-08-20 Agents + 2026-01-28
                // Private Methods). The visitor classifies each
                // declaration, validates FIP constraints, and queues
                // body/formals evaluation. The Consume step (see
                // build_agent_desugaring) assembles the desugared
                // for + new + match + dispatch tree at parse time, so
                // no AST variant is needed.
                kind!("agent_block") => {
                    let name_node = get_field(&node, field!("name"));
                    let decls_node = get_field(&node, field!("decls"));

                    struct DeclEntry<'tree> {
                        body_node: tree_sitter::Node<'tree>,
                        formals_node: Option<tree_sitter::Node<'tree>>,
                    }
                    let decl_count = decls_node.named_child_count();
                    let mut decls_desc: SmallVec<[AgentDeclDesc<'_>; 4]> =
                        SmallVec::with_capacity(decl_count);
                    let mut decl_entries: SmallVec<[DeclEntry<'_>; 4]> =
                        SmallVec::with_capacity(decl_count);

                    let mut ctor_pos: Option<SourcePos> = None;
                    let mut pub_default_pos: Option<SourcePos> = None;
                    let mut priv_default_pos: Option<SourcePos> = None;
                    let mut has_priv_method = false;

                    for decl_node in decls_node.named_children(&mut decls_node.walk()) {
                        let inner = get_first_child(&decl_node);
                        let inner_pos: SourcePos = inner.start_position().into();
                        let body_node = get_field(&inner, field!("body"));
                        let formals_node = inner.child_by_field_id(field!("formals"));
                        let (arity, has_cont) = match formals_node {
                            Some(n) => (
                                n.named_child_count(),
                                n.child_by_field_id(field!("cont")).is_some(),
                            ),
                            None => (0, false),
                        };
                        let is_private = inner.child_by_field_id(field!("private")).is_some();

                        match inner.kind_id() {
                            kind!("constructor_decl") => {
                                if let Some(first) = ctor_pos {
                                    errors.push(AnnParsingError::new(
                                        ParsingError::DuplicateAgentDecl {
                                            what: "constructor",
                                            first,
                                            second: inner_pos,
                                        },
                                        &inner,
                                    ));
                                } else {
                                    ctor_pos = Some(inner_pos);
                                }
                                decls_desc.push(AgentDeclDesc::Constructor { arity, has_cont });
                            }
                            kind!("method_decl") => {
                                let name_node = get_field(&inner, field!("name"));
                                let method_name = Id {
                                    name: get_node_value(&name_node, source),
                                    pos: name_node.start_position().into(),
                                };
                                if is_private {
                                    has_priv_method = true;
                                }
                                decls_desc.push(AgentDeclDesc::Method {
                                    name: method_name,
                                    arity,
                                    has_cont,
                                    is_private,
                                });
                            }
                            kind!("default_decl") => {
                                let (slot, what) = if is_private {
                                    (&mut priv_default_pos, "private default")
                                } else {
                                    (&mut pub_default_pos, "default")
                                };
                                if let Some(first) = *slot {
                                    errors.push(AnnParsingError::new(
                                        ParsingError::DuplicateAgentDecl {
                                            what,
                                            first,
                                            second: inner_pos,
                                        },
                                        &inner,
                                    ));
                                } else {
                                    *slot = Some(inner_pos);
                                }
                                decls_desc.push(AgentDeclDesc::Default {
                                    arity,
                                    has_cont,
                                    is_private,
                                });
                            }
                            _ => unreachable!("agent_decl is one of three kinds"),
                        }
                        decl_entries.push(DeclEntry {
                            body_node,
                            formals_node,
                        });
                    }

                    // Constraint violations are reported via `errors` --
                    // the loop's `continue 'parse` resets the local
                    // `bad` flag, so setting it here would be dead.
                    // The visitor still queues all decl bodies for
                    // evaluation; consume produces a best-effort tree
                    // (with a Nil ctor fallback) but the final
                    // node_to_ast returns Validated::fail because of
                    // the errors Vec.
                    if ctor_pos.is_none() {
                        errors.push(AnnParsingError::new(
                            ParsingError::MissingAgentDecl {
                                what: "constructor",
                            },
                            &node,
                        ));
                    }
                    if pub_default_pos.is_none() {
                        errors.push(AnnParsingError::new(
                            ParsingError::MissingAgentDecl { what: "default" },
                            &node,
                        ));
                    }
                    if has_priv_method && priv_default_pos.is_none() {
                        errors.push(AnnParsingError::new(
                            ParsingError::MissingAgentDecl {
                                what: "private default",
                            },
                            &node,
                        ));
                    }

                    cont_stack.push(K::ConsumeAgentBlock { span, decls_desc });
                    for entry in decl_entries.into_iter().rev() {
                        if let Some(formals_node) = entry.formals_node {
                            cont_stack.push(K::EvalList(formals_node.walk()));
                        }
                        cont_stack.push(K::EvalDelayed(entry.body_node));
                    }
                    node = name_node;
                    continue 'parse;
                }

                // try/catch sugar (FIP 2026-02-06 File-I/O §"Error
                // syntax"). Desugars at parse time to:
                //
                //   for (@[__ok, ...__rest] <- <source>) {
                //     if (__ok) {
                //       let @<try_pat> <- __rest in { <try_body> }
                //     } else {
                //       let @<catch_pat> <- __rest in { <catch_body> }
                //     }
                //   }
                //
                // The try-line `@<try_pat>` is optional (for methods
                // that return `[true]` alone); when absent, the success
                // branch drops the `let` and runs the try body directly.
                //
                // No `finally` clause: Rholang bodies may be
                // asynchronous, so a keyword-based finally would race
                // the branch. Programmers who want after-both-branches
                // semantics use an explicit `done` channel in each
                // branch and a `for(<- done)` receiver in parallel.
                //
                // Source handling mirrors linear_bind: source can be
                // any of the four `_source` shapes, and its pieces are
                // pushed onto proc_stack in the same order the for-
                // comprehension consumer expects, so reconstruction is
                // via the same SourceDesc / into_name / into_names
                // helpers.
                kind!("try_block") => {
                    let source_node = get_field(&node, field!("source"));
                    let try_pattern_node_opt = node.child_by_field_id(field!("try_pattern"));
                    let try_body_node = get_field(&node, field!("try_body"));
                    let catch_pattern_node = get_field(&node, field!("catch_pattern"));
                    let catch_body_node = get_field(&node, field!("catch_body"));

                    let source_desc = match source_node.kind_id() {
                        kind!("simple_source") => SourceDesc::Simple,
                        kind!("receive_send_source") => SourceDesc::RS,
                        kind!("send_receive_source") => {
                            let inputs_node = get_field(&source_node, field!("inputs"));
                            SourceDesc::SR {
                                arity: inputs_node.named_child_count(),
                            }
                        }
                        kind!("send_method_source") => {
                            let inputs_node = get_field(&source_node, field!("inputs"));
                            SourceDesc::SM {
                                arity: inputs_node.named_child_count(),
                            }
                        }
                        _ => unreachable!("try_block source is one of four _source kinds"),
                    };

                    let has_try_pattern = try_pattern_node_opt.is_some();

                    // Total slice length ConsumeTryBlock will pop:
                    //   source_desc.len() + [try_pat] + try_body +
                    //   catch_pat + catch_body
                    let total_len = source_desc.len() + (has_try_pattern as usize) + 3;

                    // Push evaluations into temp in forward order,
                    // then reverse-append to cont_stack (same pattern
                    // as ConsumeForComprehension).
                    temp_cont_stack.reserve(total_len);

                    // Source: mirror linear_bind's per-kind pattern.
                    match source_desc {
                        SourceDesc::Simple | SourceDesc::RS => {
                            temp_cont_stack.push(K::EvalDelayed(get_first_child(&source_node)));
                        }
                        SourceDesc::SR { .. } => {
                            let inputs = get_field(&source_node, field!("inputs"));
                            temp_cont_stack.push(K::EvalDelayed(get_first_child(&source_node)));
                            temp_cont_stack.push(K::EvalList(inputs.walk()));
                        }
                        SourceDesc::SM { .. } => {
                            let method_node = get_field(&source_node, field!("method"));
                            let inputs = get_field(&source_node, field!("inputs"));
                            let method_name = get_node_value(&method_node, source);
                            let method_lit = AnnProc {
                                proc: ast_builder.alloc_string_literal(method_name),
                                span: method_node.range().into(),
                            };
                            temp_cont_stack.push(K::EvalDelayed(get_first_child(&source_node)));
                            temp_cont_stack.push(K::PushAnnProc(method_lit));
                            temp_cont_stack.push(K::EvalList(inputs.walk()));
                        }
                    }
                    // Try pattern (optional)
                    if let Some(tp) = try_pattern_node_opt {
                        temp_cont_stack.push(K::EvalDelayed(tp));
                    }
                    // Try body
                    temp_cont_stack.push(K::EvalDelayed(try_body_node));
                    // Catch pattern
                    temp_cont_stack.push(K::EvalDelayed(catch_pattern_node));
                    // Catch body
                    temp_cont_stack.push(K::EvalDelayed(catch_body_node));

                    temp_cont_stack.reverse();
                    cont_stack.push(K::ConsumeTryBlock {
                        span,
                        source_desc,
                        has_try_pattern,
                        total_len,
                    });
                    cont_stack.append(&mut temp_cont_stack);
                    // Fall through to apply_cont, which will pull the
                    // first evaluation off cont_stack.
                }

                kind!("ifElse") => {
                    let condition_node = get_field(&node, field!("condition"));
                    let if_true_node = get_field(&node, field!("consequence"));
                    match node.child_by_field_id(field!("alternative")) {
                        Some(alternative_node) => {
                            cont_stack.push(K::ConsumeIfThenElse { span });
                            cont_stack.push(K::EvalDelayed(alternative_node));
                        }
                        None => {
                            cont_stack.push(K::ConsumeIfThen { span });
                        }
                    };
                    cont_stack.push(K::EvalDelayed(if_true_node));
                    node = condition_node;
                    continue 'parse;
                }

                kind!("input") => {
                    let receipts_node = get_field(&node, field!("receipts"));
                    let proc_node = get_field(&node, field!("proc"));

                    let mut rs = SmallVec::with_capacity(receipts_node.named_child_count());
                    temp_cont_stack.reserve(rs.capacity() * 2);

                    let mut total_len = 0;

                    for receipt_node in receipts_node.named_children(&mut receipts_node.walk()) {
                        let mut bs = SmallVec::with_capacity(receipt_node.named_child_count());
                        let mut receipt_len = 0;

                        // The optional `where`-clause guard. The guard's _proc
                        // node is also a named child of the receipt, so we
                        // look it up by field name and skip it when iterating
                        // bind children below.
                        let guard_node = receipt_node.child_by_field_id(field!("guard"));
                        let has_guard = guard_node.is_some();

                        for bind_node in receipt_node.named_children(&mut receipt_node.walk()) {
                            let bind_kind = bind_node.kind_id();

                            // Per-clause signed bind `{% y <- x %}[s]` (Greg
                            // app:concrete SignedBind; Axis-C join). Extract by
                            // field (names?, input, sig); the proc slots are
                            // [source_name, (SR inputs), names…, sig_procs…].
                            if bind_kind == kind!("signed_bind") {
                                let names_node = bind_node.child_by_field_id(field!("names"));
                                let source_node = get_field(&bind_node, field!("input"));
                                let sig_node = get_field(&bind_node, field!("sig"));

                                let (name_count, cont_present) = match names_node {
                                    Some(names) => (
                                        names.named_child_count(),
                                        names.child_by_field_id(field!("cont")).is_some(),
                                    ),
                                    None => (0, false),
                                };

                                let source_desc = match source_node.kind_id() {
                                    kind!("simple_source") => SourceDesc::Simple,
                                    kind!("receive_send_source") => SourceDesc::RS,
                                    kind!("send_receive_source") => SourceDesc::SR {
                                        arity: get_field(&source_node, field!("inputs"))
                                            .named_child_count(),
                                    },
                                    _ => unreachable!(
                                        "Sources in for-comprehensions have three kinds: simple, receive_send and send_receive"
                                    ),
                                };

                                let mut sig_ops: SmallVec<[SigOp; 4]> = SmallVec::new();
                                let mut sig_procs: Vec<tree_sitter::Node> = Vec::new();
                                flatten_signature(&sig_node, &mut sig_ops, &mut sig_procs);

                                let bind_desc = BindDesc::Signed {
                                    name_count,
                                    cont_present,
                                    source: source_desc,
                                    sig_ops,
                                    sig_proc_count: sig_procs.len(),
                                };

                                // Slot order = push order here.
                                match source_desc {
                                    SourceDesc::SR { .. } => {
                                        let inputs = get_field(&source_node, field!("inputs"));
                                        temp_cont_stack
                                            .push(K::EvalDelayed(get_first_child(&source_node)));
                                        temp_cont_stack.push(K::EvalList(inputs.walk()));
                                    }
                                    _ => {
                                        temp_cont_stack
                                            .push(K::EvalDelayed(get_first_child(&source_node)));
                                    }
                                }
                                if let Some(names) = names_node {
                                    temp_cont_stack.push(K::EvalList(names.walk()));
                                }
                                for n in &sig_procs {
                                    temp_cont_stack.push(K::EvalDelayed(*n));
                                }

                                receipt_len += bind_desc.len();
                                bs.push(bind_desc);
                                continue;
                            }

                            if bind_kind != kind!("linear_bind")
                                && bind_kind != kind!("repeated_bind")
                                && bind_kind != kind!("peek_bind")
                            {
                                // Anything else here is the guard's _proc;
                                // it's processed via guard_node above.
                                continue;
                            }

                            let (names_node, source_node) = if bind_node.named_child_count() > 1 {
                                let (ns, s) = get_left_and_right(&bind_node);
                                (Some(ns), s)
                            } else {
                                (None, get_first_child(&bind_node))
                            };
                            // A source the grammar could not parse (a process
                            // where a name is required, say) arrives as an ERROR
                            // node, which is none of the four source kinds. Skip
                            // the bind; query_errors reports the ERROR node once
                            // the parse completes.
                            if source_node.is_error() || source_node.is_missing() {
                                continue;
                            }

                            let (name_count, cont_present) = match names_node {
                                Some(names) => (
                                    names.named_child_count(),
                                    names.child_by_field_id(field!("cont")).is_some(),
                                ),
                                None => (0, false),
                            };

                            let bind_desc = match bind_kind {
                                kind!("linear_bind") => {
                                    let source_desc = match source_node.kind_id() {
                                        kind!("simple_source") => SourceDesc::Simple,
                                        kind!("receive_send_source") => SourceDesc::RS,
                                        kind!("send_receive_source") => {
                                            let inputs_node =
                                                get_field(&source_node, field!("inputs"));
                                            SourceDesc::SR {
                                                arity: inputs_node.named_child_count(),
                                            }
                                        }
                                        kind!("send_method_source") => {
                                            let inputs_node =
                                                get_field(&source_node, field!("inputs"));
                                            SourceDesc::SM {
                                                arity: inputs_node.named_child_count(),
                                            }
                                        }
                                        _ => unreachable!(
                                            "Sources in for-comprehensions have four kinds: simple, receive_send, send_receive, and send_method"
                                        ),
                                    };

                                    BindDesc::Linear {
                                        name_count,
                                        cont_present,
                                        source: source_desc,
                                    }
                                }
                                kind!("repeated_bind") => BindDesc::Repeated {
                                    name_count,
                                    cont_present,
                                },
                                kind!("peek_bind") => BindDesc::Peek {
                                    name_count,
                                    cont_present,
                                },
                                _ => unreachable!("Filtered above"),
                            };

                            match &bind_desc {
                                BindDesc::Linear {
                                    source: SourceDesc::SR { .. },
                                    ..
                                } => {
                                    let inputs = get_field(&source_node, field!("inputs"));
                                    temp_cont_stack
                                        .push(K::EvalDelayed(get_first_child(&source_node)));
                                    temp_cont_stack.push(K::EvalList(inputs.walk()));
                                }
                                BindDesc::Linear {
                                    source: SourceDesc::SM { .. },
                                    ..
                                } => {
                                    // proc_stack target after eval:
                                    //   [channel, method_lit, input0, input1, ...]
                                    // Push order (becomes reversed below):
                                    //   channel last, then method_lit, then inputs.
                                    let method_node = get_field(&source_node, field!("method"));
                                    let inputs = get_field(&source_node, field!("inputs"));
                                    let method_name = get_node_value(&method_node, source);
                                    let method_lit = AnnProc {
                                        proc: ast_builder.alloc_string_literal(method_name),
                                        span: method_node.range().into(),
                                    };
                                    temp_cont_stack
                                        .push(K::EvalDelayed(get_first_child(&source_node)));
                                    temp_cont_stack.push(K::PushAnnProc(method_lit));
                                    temp_cont_stack.push(K::EvalList(inputs.walk()));
                                }
                                BindDesc::Linear { .. } => {
                                    temp_cont_stack
                                        .push(K::EvalDelayed(get_first_child(&source_node)));
                                }
                                _ => {
                                    temp_cont_stack.push(K::EvalDelayed(source_node));
                                }
                            }

                            if let Some(names) = names_node {
                                temp_cont_stack.push(K::EvalList(names.walk()));
                            }

                            receipt_len += bind_desc.len();
                            bs.push(bind_desc);
                        }

                        // Push the guard's eval LAST so that its resulting
                        // AnnProc lands at the end of this receipt's slice in
                        // proc_stack — matching what ReceiptIter expects.
                        if let Some(guard_node) = guard_node {
                            temp_cont_stack.push(K::EvalDelayed(guard_node));
                            receipt_len += 1;
                        }

                        rs.push(ReceiptDesc {
                            parts: bs,
                            len: receipt_len,
                            has_guard,
                        });
                        total_len += receipt_len;
                    }
                    temp_cont_stack.reverse();

                    cont_stack.push(K::ConsumeForComprehension {
                        desc: rs,
                        total_len,
                        span,
                    });
                    cont_stack.append(&mut temp_cont_stack);
                    node = proc_node;
                    continue 'parse;
                }

                kind!("match") => {
                    let expression_node = get_field(&node, field!("expression"));
                    let cases_node = get_field(&node, field!("cases"));

                    // Per-case guard mask: tells ConsumeMatch which cases
                    // pushed a `guard` AnnProc onto proc_stack between their
                    // `pattern` and `proc` AnnProcs.
                    let mut guards_present: SmallVec<[bool; 4]> = SmallVec::new();
                    temp_cont_stack.reserve(3 * cases_node.named_child_count());

                    for case in
                        named_children_of_kind(&cases_node, kind!("case"), &mut cases_node.walk())
                    {
                        let pattern_node = get_field(&case, field!("pattern"));
                        let guard_node = case.child_by_field_id(field!("guard"));
                        let proc_node = get_field(&case, field!("proc"));

                        // Push pattern, optional guard, proc — order on
                        // proc_stack after evaluation (since temp_cont_stack
                        // gets reversed below) will be pattern, [guard,]
                        // proc per case.
                        temp_cont_stack.push(K::EvalDelayed(pattern_node));
                        if let Some(g) = guard_node {
                            temp_cont_stack.push(K::EvalDelayed(g));
                            guards_present.push(true);
                        } else {
                            guards_present.push(false);
                        }
                        temp_cont_stack.push(K::EvalDelayed(proc_node));
                    }
                    temp_cont_stack.reverse();

                    cont_stack.push(K::ConsumeMatch {
                        span,
                        guards_present,
                    });
                    cont_stack.append(&mut temp_cont_stack);

                    node = expression_node;
                    continue 'parse;
                }

                kind!("let") => {
                    fn let_decl_is_malformed(
                        lhs_arity: usize,
                        rhs_arity: usize,
                        lhs_has_cont: bool,
                    ) -> bool {
                        (lhs_has_cont && lhs_arity > rhs_arity) || lhs_arity != rhs_arity
                    }

                    let decls_node = get_field(&node, field!("decls"));
                    let body_node = get_field(&node, field!("proc"));

                    let concurrent = decls_node.kind_id() == kind!("conc_decls");

                    let mut let_decls = SmallVec::with_capacity(decls_node.named_child_count());
                    temp_cont_stack.reserve(2 * let_decls.capacity());

                    let mut total_len = 0;

                    for decl_node in decls_node.named_children(&mut decls_node.walk()) {
                        let (lhs, rhs) = get_left_and_right(&decl_node);

                        let lhs_arity;
                        let rhs_arity;
                        let lhs_has_cont;

                        let is_var_decl = decl_node.kind_id() == kind!("let_var_decl");
                        if is_var_decl {
                            lhs_arity = 1;
                            rhs_arity = 1;
                            lhs_has_cont = false;

                            temp_cont_stack.push(K::EvalDelayed(lhs));
                            temp_cont_stack.push(K::EvalDelayed(rhs));
                        } else {
                            lhs_arity = lhs.named_child_count();
                            rhs_arity = rhs.named_child_count();
                            lhs_has_cont = lhs.child_by_field_id(field!("cont")).is_some();

                            if let_decl_is_malformed(lhs_arity, rhs_arity, lhs_has_cont) {
                                errors.push(AnnParsingError::new(
                                    ParsingError::MalformedLetDecl {
                                        lhs_arity,
                                        rhs_arity,
                                    },
                                    &decl_node,
                                ));
                            }
                            temp_cont_stack.push(K::EvalList(lhs.walk()));
                            temp_cont_stack.push(K::EvalList(rhs.walk()));
                        }

                        let_decls.push(LetDecl {
                            lhs_arity,
                            lhs_has_cont,
                            rhs_arity,
                            is_var_decl,
                        });
                        total_len += lhs_arity + rhs_arity;
                    }
                    temp_cont_stack.reverse();

                    cont_stack.push(K::ConsumeLet {
                        span,
                        concurrent,
                        let_decls,
                        total_len,
                    });
                    cont_stack.append(&mut temp_cont_stack);

                    node = body_node;
                    continue 'parse;
                }

                kind!("bundle") => {
                    let bundle_node = get_field(&node, field!("bundle_type"));

                    let bundle = match bundle_node.kind_id() {
                        kind!("bundle_write") => BundleType::BundleWrite,
                        kind!("bundle_read") => BundleType::BundleRead,
                        kind!("bundle_equiv") => BundleType::BundleEquiv,
                        kind!("bundle_read_write") => BundleType::BundleReadWrite,
                        _ => unreachable!("There are four bundle types in Rholang"),
                    };

                    let proc_node = get_field(&node, field!("proc"));
                    cont_stack.push(K::ConsumeBundle { span, typ: bundle });
                    node = proc_node;
                    continue 'parse;
                }

                kind!("send_sync") => {
                    let name_node = get_field(&node, field!("channel"));
                    let messages_node = get_field(&node, field!("inputs"));
                    let arity = messages_node.named_child_count();
                    let sync_send_cont_node = get_field(&node, field!("cont"));
                    let choice_node = get_first_child(&sync_send_cont_node);
                    match choice_node.kind_id() {
                        kind!("empty_cont") => {
                            cont_stack.push(K::ConsumeSendSync { span, arity });
                        }
                        kind!("non_empty_cont") => {
                            let cont_node = get_first_child(&choice_node);
                            cont_stack.push(K::ConsumeSendSyncWithCont { span, arity });
                            cont_stack.push(K::EvalDelayed(cont_node));
                        }
                        _ => {
                            unreachable!("Continuations of send_sync are either empty or non-empty")
                        }
                    };
                    cont_stack.push(K::EvalList(messages_node.walk()));
                    node = name_node;
                    continue 'parse;
                }

                // `x!y(args)<cont>` desugars to `x!?("y", args)<cont>`
                // at parse time -- the AST carries a Proc::SendSync
                // with the method name prepended as a StringLiteral.
                // Mirrors what send_method_source does for the for-
                // source position; no AST variant for either form.
                kind!("send_method") => {
                    let name_node = get_field(&node, field!("channel"));
                    let method_node = get_field(&node, field!("method"));
                    let messages_node = get_field(&node, field!("inputs"));
                    let original_arity = messages_node.named_child_count();
                    // +1 for the synthesized method-name StringLiteral
                    let synthesized_arity = original_arity + 1;
                    let sync_send_cont_node = get_field(&node, field!("cont"));
                    let choice_node = get_first_child(&sync_send_cont_node);
                    let method_name = get_node_value(&method_node, source);
                    let method_lit = AnnProc {
                        proc: ast_builder.alloc_string_literal(method_name),
                        span: method_node.range().into(),
                    };
                    match choice_node.kind_id() {
                        kind!("empty_cont") => {
                            cont_stack.push(K::ConsumeSendSync {
                                span,
                                arity: synthesized_arity,
                            });
                        }
                        kind!("non_empty_cont") => {
                            let cont_node = get_first_child(&choice_node);
                            cont_stack.push(K::ConsumeSendSyncWithCont {
                                span,
                                arity: synthesized_arity,
                            });
                            cont_stack.push(K::EvalDelayed(cont_node));
                        }
                        _ => unreachable!(
                            "Continuations of send_method are either empty or non-empty"
                        ),
                    };
                    // proc_stack target: [channel, method_lit, input0, ...]
                    // cont_stack is LIFO; top runs first. Execution order:
                    //   1. eval channel (immediate via node = name_node)
                    //   2. PushAnnProc(method_lit) -- the prepended literal
                    //   3. EvalList(inputs) -- one Continue per input
                    //   4. ConsumeSendSync -- (pushed above the match arm)
                    cont_stack.push(K::EvalList(messages_node.walk()));
                    cont_stack.push(K::PushAnnProc(method_lit));
                    node = name_node;
                    continue 'parse;
                }

                kind!("var_ref") => {
                    let (var_ref_kind_node, var_node) = get_left_and_right(&node);

                    let var_ref_kind = match get_node_value(&var_ref_kind_node, source) {
                        "=" => VarRefKind::Proc,
                        "=*" => VarRefKind::Name,
                        _ => unreachable!("var_ref_kind is either '=' or '=*'"),
                    };
                    let var = Id {
                        name: get_node_value(&var_node, source),
                        pos: var_node.start_position().into(),
                    };

                    proc_stack.push(ast_builder.alloc_var_ref(var_ref_kind, var), span);
                }

                kind!("choice") => {
                    unimplemented!("Select is not implemented in this version of Rholang")
                }

                _ => {
                    let text = get_node_value(&node, source);
                    if text == "("
                        && let Some(next_sibling) = node.next_named_sibling()
                    {
                        node = next_sibling;
                        continue 'parse;
                    }

                    unimplemented!("{node}");
                }
            }
        }

        if bad {
            proc_stack.push(ast_builder.bad_const(), node.range().into());
        }
        let step = apply_cont(&mut cont_stack, &mut proc_stack, ast_builder);
        match step {
            Step::Done => {
                if start_node.has_error() {
                    // discover all the errors
                    errors::query_errors(start_node, source, &mut errors);
                }
                if let Some(some_errors) = NEVec::try_from_vec(errors) {
                    return Validated::fail(ParsingFailure {
                        partial_tree: proc_stack.to_proc_partial(),
                        errors: some_errors,
                    });
                }
                let last = proc_stack.into_proc();
                return Validated::Good(last);
            }
            Step::Continue(n) => {
                node = n;
                continue 'parse;
            }
        }
    }
}

fn parse_decls<'a>(from: &tree_sitter::Node, source: &'a str) -> Vec<NameDecl<'a>> {
    let mut result = Vec::with_capacity(from.named_child_count());

    for decl_node in from.named_children(&mut from.walk()) {
        let var_node = get_first_child(&decl_node);
        let id = Id {
            name: get_node_value(&var_node, source),
            pos: var_node.start_position().into(),
        };
        let uri = decl_node
            .child_by_field_id(field!("uri"))
            .map(|uri_literal| get_node_value(&uri_literal, source).into());

        result.push(NameDecl { id, uri });
    }

    result
}

fn parse_sized_int_literal(literal: &str, suffix: char) -> Option<(&str, u32)> {
    let (value, width) = literal.rsplit_once(suffix)?;
    let bits: u32 = width.parse().ok()?;
    if bits < 8 || !bits.is_power_of_two() {
        return None;
    }
    Some((value, bits))
}

fn parse_float_literal(literal: &str) -> Option<(&str, u16)> {
    let (value, width) = literal.rsplit_once('f')?;
    let bits: u16 = width.parse().ok()?;
    match bits {
        32 | 64 | 128 | 256 => Some((value, bits)),
        _ => None,
    }
}

fn parse_fixed_point_literal(literal: &str) -> Option<(&str, u32)> {
    let (value, scale) = literal.rsplit_once('p')?;
    let scale: u32 = scale.parse().ok()?;
    Some((value, scale))
}

fn apply_cont<'tree, 'ast>(
    cont_stack: &mut Vec<K<'tree, 'ast>>,
    proc_stack: &mut ProcStack<'ast>,
    ast_builder: &'ast ASTBuilder<'ast>,
) -> Step<'tree> {
    fn move_cursor_to_named<'a>(
        cursor: &mut tree_sitter::TreeCursor<'a>,
    ) -> Option<tree_sitter::Node<'a>> {
        let mut has_more = if cursor.depth() == 0 {
            cursor.goto_first_child()
        } else {
            cursor.goto_next_sibling()
        };

        while has_more {
            let node = cursor.node();
            if node.is_named() {
                return Some(node);
            }
            has_more = cursor.goto_next_sibling();
        }

        None
    }

    loop {
        let cc = match cont_stack.last_mut() {
            None => return Step::Done,
            Some(k) => k,
        };

        match cc {
            K::EvalDelayed(node) => {
                let next = *node;
                cont_stack.pop();
                return Step::Continue(next);
            }
            K::EvalList(cursor) => {
                if let Some(node) = move_cursor_to_named(cursor) {
                    return Step::Continue(node);
                }
                cont_stack.pop();
            }
            K::PushAnnProc(ann) => {
                let ann = *ann;
                cont_stack.pop();
                proc_stack.push(ann.proc, ann.span);
            }
            _ => {
                //consumes
                unsafe {
                    // SAFETY: We only enter this branch when cont_stack.last_mut() returned
                    // Some(_), which guarantees the stack is non-empty. The pop() cannot fail.
                    let k = cont_stack.pop().unwrap_unchecked();

                    let underflow = !match k {
                        K::ConsumeQuote => proc_stack.mark_quote(),
                        K::ConsumeBinaryExp { op, span } => {
                            proc_stack.replace_top2(|left, right| {
                                ast_builder.alloc_binary_exp(op, left, right).ann(span)
                            })
                        }
                        K::ConsumeBundle { span, typ } => proc_stack
                            .replace_top(|proc| ast_builder.alloc_bundle(typ, proc).ann(span)),
                        K::ConsumeContract {
                            arity,
                            has_cont,
                            span,
                        } => proc_stack.replace_top_slice_with_mask(
                            arity + 2,
                            |name_body_formals, mask| {
                                let name = into_name(name_body_formals[0], mask[0]);
                                let body = name_body_formals[1];
                                let args =
                                    into_names(&name_body_formals[2..], &mask[2..], has_cont);
                                ast_builder.alloc_contract(name, args, body).ann(span)
                            },
                        ),
                        K::ConsumeEval { span } => {
                            proc_stack.replace_top_with_mask(|proc, quoted| {
                                ast_builder.alloc_eval(into_name(proc, quoted)).ann(span)
                            })
                        }
                        K::ConsumeForComprehension {
                            desc,
                            total_len,
                            span,
                        } => proc_stack.replace_top_slice_with_mask(
                            total_len + 1,
                            |body_procs, mask| {
                                let body = body_procs[0];
                                let procs = &body_procs[1..];
                                ast_builder
                                    .alloc_for_with_guards(
                                        ReceiptIter::new(&desc, procs, &mask[1..]),
                                        body,
                                    )
                                    .ann(span)
                            },
                        ),
                        K::ConsumeSignedTerm {
                            ops,
                            sig_arity,
                            span,
                        } => proc_stack.replace_top_slice(sig_arity + 1, |slice| {
                            let body = slice[0];
                            let sig = rebuild_signature(&ops, &slice[1..]);
                            ast_builder.alloc_signed_term(body, sig).ann(span)
                        }),
                        K::ConsumeTokenStack {
                            all_ops,
                            layer_descs,
                            total_procs,
                            span,
                        } => proc_stack.replace_top_slice(total_procs, |slice| {
                            let mut layers: SmallVec<[Signature; 2]> = SmallVec::new();
                            let mut off = 0usize;
                            for (op_start, op_len, proc_count) in &layer_descs {
                                let ops = &all_ops[*op_start..*op_start + *op_len];
                                let procs = &slice[off..off + *proc_count];
                                layers.push(rebuild_signature(ops, procs));
                                off += *proc_count;
                            }
                            ast_builder
                                .alloc_token_stack(TokenStack { layers })
                                .ann(span)
                        }),
                        K::ConsumeIfThen { span } => proc_stack.replace_top2(|cond, if_true| {
                            ast_builder.alloc_if_then(cond, if_true).ann(span)
                        }),
                        K::ConsumeIfThenElse { span } => {
                            proc_stack.replace_top3(|cond, if_true, if_false| {
                                ast_builder
                                    .alloc_if_then_else(cond, if_true, if_false)
                                    .ann(span)
                            })
                        }
                        K::ConsumeLet {
                            span,
                            concurrent,
                            let_decls,
                            total_len,
                        } => proc_stack.replace_top_slice_with_mask(
                            total_len + 1,
                            |body_procs, mask| {
                                let body = body_procs[0];
                                ast_builder
                                    .alloc_let(
                                        LetDeclIter::new(&let_decls, &body_procs[1..], &mask[1..]),
                                        body,
                                        concurrent,
                                    )
                                    .ann(span)
                            },
                        ),
                        K::ConsumeList {
                            arity,
                            has_remainder,
                            span,
                        } => proc_stack.replace_top_slice(arity, |elems| {
                            let list = if has_remainder {
                                assert!(!elems.is_empty());
                                // SAFETY: We have checked above that there is at least one element
                                let (last, init) = elems.split_last().unwrap_unchecked();
                                ast_builder.alloc_list_with_remainder(init, into_remainder(*last))
                            } else {
                                ast_builder.alloc_list(elems)
                            };
                            list.ann(span)
                        }),
                        K::ConsumeMap {
                            arity,
                            has_remainder,
                            span,
                        } => {
                            let n = arity * 2 + if has_remainder { 1 } else { 0 };
                            proc_stack.replace_top_slice(n, |elems| {
                                let map = if has_remainder {
                                    ast_builder.alloc_map_with_remainder(
                                        &elems[1..],
                                        into_remainder(elems[0]),
                                    )
                                } else {
                                    ast_builder.alloc_map(elems)
                                };
                                map.ann(span)
                            })
                        }
                        K::ConsumePathMap {
                            arity,
                            has_remainder,
                            span,
                        } => proc_stack.replace_top_slice(arity, |elems| {
                            let path_map = if has_remainder {
                                assert!(!elems.is_empty());
                                // SAFETY: We have checked above that there is at least one element
                                let (last, init) = elems.split_last().unwrap_unchecked();
                                ast_builder
                                    .alloc_pathmap_with_remainder(init, into_remainder(*last))
                            } else {
                                ast_builder.alloc_pathmap(elems)
                            };
                            path_map.ann(span)
                        }),
                        K::ConsumeMatch {
                            span,
                            guards_present,
                        } => {
                            // Total slice = expression + sum_per_case(2 if no
                            // guard, 3 if guard).
                            let total: usize = 1 + guards_present
                                .iter()
                                .map(|&g| if g { 3 } else { 2 })
                                .sum::<usize>();
                            proc_stack.replace_top_slice(total, |slice| {
                                let expr = slice[0];
                                let mut idx = 1usize;
                                let cases = guards_present
                                    .iter()
                                    .map(|&has_guard| {
                                        let pattern = slice[idx];
                                        idx += 1;
                                        let guard = if has_guard {
                                            let g = slice[idx];
                                            idx += 1;
                                            Some(g)
                                        } else {
                                            None
                                        };
                                        let proc = slice[idx];
                                        idx += 1;
                                        (pattern, guard, proc)
                                    })
                                    .collect::<Vec<_>>();
                                ast_builder.alloc_match_with_guards(expr, cases).ann(span)
                            })
                        }
                        K::ConsumeMethod { span, id, arity } => {
                            proc_stack.replace_top_slice(arity + 1, |recv_args| {
                                let recv = recv_args[0];
                                let args = &recv_args[1..];
                                ast_builder.alloc_method(id, recv, args).ann(span)
                            })
                        }
                        K::ConsumeNew { decls, span } => proc_stack
                            .replace_top(|body| ast_builder.alloc_new(body, decls).ann(span)),
                        K::ConsumePar { span } => proc_stack.replace_top2(|left, right| {
                            ast_builder.alloc_par(left, right).ann(span)
                        }),
                        K::ConsumeSend {
                            send_type,
                            arity,
                            span,
                        } => {
                            proc_stack.replace_top_slice_with_mask(arity + 1, |name_args, mask| {
                                let channel = into_name(name_args[0], mask[0]);
                                let inputs = &name_args[1..];
                                ast_builder.alloc_send(send_type, channel, inputs).ann(span)
                            })
                        }
                        K::ConsumeSendSync { span, arity } => proc_stack
                            .replace_top_slice_with_mask(arity + 1, |name_inputs, mask| {
                                let channel = into_name(name_inputs[0], mask[0]);
                                ast_builder
                                    .alloc_send_sync(channel, &name_inputs[1..])
                                    .ann(span)
                            }),
                        K::ConsumeSendSyncWithCont { span, arity } => {
                            proc_stack.replace_top_slice_with_mask(
                                arity + 2,
                                |name_inputs_cont, mask| {
                                    let channel = into_name(name_inputs_cont[0], mask[0]);
                                    // SAFETY: Because we successfully consumed |arity + 2|
                                    // elements, then the slice.len() is greater or equal 2
                                    let (last, messages) =
                                        name_inputs_cont[1..].split_last().unwrap_unchecked();
                                    let cont = *last;
                                    ast_builder
                                        .alloc_send_sync_with_cont(channel, messages, cont)
                                        .ann(span)
                                },
                            )
                        }
                        K::ConsumeAgentBlock { span, decls_desc } => {
                            let total: usize =
                                1 + decls_desc.iter().map(|d| d.slice_len()).sum::<usize>();
                            proc_stack.replace_top_slice_with_mask(total, |slice, mask| {
                                build_agent_desugaring(ast_builder, &decls_desc, slice, mask, span)
                            })
                        }
                        K::ConsumeTryBlock {
                            span,
                            source_desc,
                            has_try_pattern,
                            total_len,
                        } => proc_stack.replace_top_slice_with_mask(total_len, |slice, mask| {
                            build_try_block_desugaring(
                                ast_builder,
                                source_desc,
                                has_try_pattern,
                                slice,
                                mask,
                                span,
                            )
                        }),
                        K::ConsumeSet {
                            arity,
                            has_remainder,
                            span,
                        } => proc_stack.replace_top_slice(arity, |elems| {
                            let set = if has_remainder {
                                assert!(!elems.is_empty());
                                // SAFETY: We have checked above that there is at least one element
                                let (last, init) = elems.split_last().unwrap_unchecked();
                                ast_builder.alloc_set_with_remainder(init, into_remainder(*last))
                            } else {
                                ast_builder.alloc_set(elems)
                            };
                            set.ann(span)
                        }),
                        K::ConsumeTuple { arity, span } => proc_stack
                            .replace_top_slice(arity, |elems| {
                                ast_builder.alloc_tuple(elems).ann(span)
                            }),
                        K::ConsumeUnaryExp { op, span } => proc_stack
                            .replace_top(|top| ast_builder.alloc_unary_exp(op, top).ann(span)),
                        _ => unreachable!("Eval continuations are handled in another branch"),
                    };

                    if underflow {
                        panic!(
                            "bug: process stack underflow!!!\nProcess stack: {proc_stack:#?}\nContinuation stack: {cont_stack:#?}"
                        );
                    }
                }
            }
        }
    }
}

enum Step<'a> {
    Done,
    Continue(tree_sitter::Node<'a>),
}

type LetDecls = SmallVec<[LetDecl; 1]>;
type ReceiptDescripts = SmallVec<[ReceiptDesc; 1]>;
type BindDescripts = SmallVec<[BindDesc; 1]>;

#[derive(Clone)]
enum K<'tree, 'ast> {
    ConsumeBinaryExp {
        op: BinaryExpOp,
        span: SourceSpan,
    },
    ConsumeBundle {
        span: SourceSpan,
        typ: BundleType,
    },
    ConsumeContract {
        arity: usize,
        has_cont: bool,
        span: SourceSpan,
    },
    ConsumeEval {
        span: SourceSpan,
    },
    ConsumeForComprehension {
        desc: ReceiptDescripts,
        total_len: usize,
        span: SourceSpan,
    },
    // Cost-accounted Rholang. `ops` is the post-order signature spine; the
    // signature's embedded procs sit at proc_stack slice[1..] (slice[0] = body).
    ConsumeSignedTerm {
        ops: SmallVec<[SigOp; 4]>,
        sig_arity: usize,
        span: SourceSpan,
    },
    // `layer_descs[i] = (op_start, op_len, proc_count)` slices `all_ops` and the
    // proc slice for layer `i` of a bare token stack `s :: … :: ()`.
    ConsumeTokenStack {
        all_ops: SmallVec<[SigOp; 8]>,
        layer_descs: SmallVec<[(usize, usize, usize); 2]>,
        total_procs: usize,
        span: SourceSpan,
    },
    ConsumeIfThen {
        span: SourceSpan,
    },
    ConsumeIfThenElse {
        span: SourceSpan,
    },
    ConsumeLet {
        span: SourceSpan,
        concurrent: bool,
        let_decls: LetDecls,
        total_len: usize,
    },
    ConsumeList {
        arity: usize,
        has_remainder: bool,
        span: SourceSpan,
    },
    ConsumeMap {
        arity: usize,
        has_remainder: bool,
        span: SourceSpan,
    },
    ConsumePathMap {
        arity: usize,
        has_remainder: bool,
        span: SourceSpan,
    },
    ConsumeMatch {
        span: SourceSpan,
        // One bool per case in source order: true if that case has a
        // `where` guard sitting between its pattern and proc on
        // proc_stack. Length is the number of cases.
        guards_present: SmallVec<[bool; 4]>,
    },
    ConsumeMethod {
        span: SourceSpan,
        id: Id<'ast>,
        arity: usize,
    },
    ConsumeNew {
        decls: Vec<NameDecl<'ast>>,
        span: SourceSpan,
    },
    ConsumePar {
        span: SourceSpan,
    },
    ConsumeQuote,
    ConsumeSend {
        send_type: SendType,
        arity: usize,
        span: SourceSpan,
    },
    ConsumeSendSync {
        span: SourceSpan,
        arity: usize,
    },
    ConsumeSendSyncWithCont {
        span: SourceSpan,
        arity: usize,
    },
    ConsumeAgentBlock {
        span: SourceSpan,
        decls_desc: SmallVec<[AgentDeclDesc<'ast>; 4]>,
    },
    ConsumeTryBlock {
        span: SourceSpan,
        source_desc: SourceDesc,
        has_try_pattern: bool,
        total_len: usize,
    },
    ConsumeSet {
        arity: usize,
        has_remainder: bool,
        span: SourceSpan,
    },
    ConsumeTuple {
        arity: usize,
        span: SourceSpan,
    },
    ConsumeUnaryExp {
        op: UnaryExpOp,
        span: SourceSpan,
    },
    EvalDelayed(tree_sitter::Node<'tree>),
    EvalList(tree_sitter::TreeCursor<'tree>),
    // Push an already-constructed AnnProc directly onto proc_stack
    // without a tree-sitter parse step. Used for parse-time
    // synthesized literals (e.g., the method-name StringLiteral
    // injected by send_method_source).
    PushAnnProc(AnnProc<'ast>),
}

impl Debug for K<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConsumeBinaryExp { op, span } => f
                .debug_struct("ConsumeBinaryExp")
                .field("op", op)
                .field("span", span)
                .finish(),
            Self::ConsumeBundle { span, typ } => f
                .debug_struct("ConsumeBundle")
                .field("typ", typ)
                .field("span", span)
                .finish(),
            Self::ConsumeContract { arity, span, .. } => f
                .debug_struct("ConsumeContract")
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumeEval { span } => {
                f.debug_struct("ConsumeEval").field("span", span).finish()
            }
            Self::ConsumeForComprehension {
                desc,
                total_len,
                span,
            } => f
                .debug_struct("ConsumeForComprehension")
                .field("desc", desc)
                .field("total_len", total_len)
                .field("span", span)
                .finish(),
            Self::ConsumeSignedTerm {
                ops,
                sig_arity,
                span,
            } => f
                .debug_struct("ConsumeSignedTerm")
                .field("ops", ops)
                .field("sig_arity", sig_arity)
                .field("span", span)
                .finish(),
            Self::ConsumeTokenStack {
                layer_descs,
                total_procs,
                span,
                ..
            } => f
                .debug_struct("ConsumeTokenStack")
                .field("layer_descs", layer_descs)
                .field("total_procs", total_procs)
                .field("span", span)
                .finish(),
            Self::ConsumeIfThen { span } => {
                f.debug_struct("ConsumeIfThen").field("span", span).finish()
            }
            Self::ConsumeIfThenElse { span } => f
                .debug_struct("ConsumeIfThenElse")
                .field("span", span)
                .finish(),
            Self::ConsumeLet {
                span,
                concurrent,
                let_decls,
                total_len,
            } => f
                .debug_struct("ConsumeLet")
                .field("concurrent", concurrent)
                .field("let_decls", let_decls)
                .field("total_len", total_len)
                .field("span", span)
                .finish(),
            Self::ConsumeList { arity, span, .. } => f
                .debug_struct("ConsumeList")
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumeMap { arity, span, .. } => f
                .debug_struct("ConsumeMap")
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumePathMap { arity, span, .. } => f
                .debug_struct("ConsumePathMap")
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumeMatch {
                span,
                guards_present,
            } => f
                .debug_struct("ConsumeMatch")
                .field("guards_present", guards_present)
                .field("span", span)
                .finish(),
            Self::ConsumeMethod { span, id, arity } => f
                .debug_struct("ConsumeMethod")
                .field("id", &id.name)
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumeNew { decls, span } => f
                .debug_struct("ConsumeNew")
                .field("decls", decls)
                .field("span", span)
                .finish(),
            Self::ConsumePar { span } => f.debug_struct("ConsumePar").field("span", span).finish(),
            Self::ConsumeQuote => f.debug_struct("ConsumeQuote").finish(),
            Self::ConsumeSend {
                send_type,
                arity,
                span,
            } => f
                .debug_struct("ConsumeSend")
                .field("send_type", send_type)
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumeSendSync { span, arity } => f
                .debug_struct("ConsumeSendSync")
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumeSendSyncWithCont { span, arity } => f
                .debug_struct("ConsumeSendSyncWithCont")
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumeAgentBlock { span, decls_desc } => f
                .debug_struct("ConsumeAgentBlock")
                .field("decls", decls_desc)
                .field("span", span)
                .finish(),
            Self::ConsumeTryBlock {
                span,
                source_desc,
                has_try_pattern,
                total_len,
            } => f
                .debug_struct("ConsumeTryBlock")
                .field("source_desc", source_desc)
                .field("has_try_pattern", has_try_pattern)
                .field("total_len", total_len)
                .field("span", span)
                .finish(),
            Self::ConsumeSet { arity, span, .. } => f
                .debug_struct("ConsumeSet")
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumeTuple { arity, span } => f
                .debug_struct("ConsumeTuple")
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumeUnaryExp { op, span } => f
                .debug_struct("ConsumeUnaryExp")
                .field("op", op)
                .field("span", span)
                .finish(),
            Self::EvalDelayed(arg0) => f.debug_tuple("EvalDelayed").field(arg0).finish(),
            Self::EvalList(arg0) => f
                .debug_struct("EvalList")
                .field("at", &arg0.node())
                .finish(),
            Self::PushAnnProc(arg0) => f.debug_tuple("PushAnnProc").field(arg0).finish(),
        }
    }
}

struct ProcStack<'a> {
    stack: Vec<AnnProc<'a>>,
    quote_mask: BitVec,
}

impl<'a> ProcStack<'a> {
    const DEFAULT_CAPACITY: usize = 32;

    fn new() -> Self {
        ProcStack {
            stack: Vec::with_capacity(Self::DEFAULT_CAPACITY),
            quote_mask: BitVec::with_capacity(Self::DEFAULT_CAPACITY),
        }
    }

    #[inline(always)]
    fn push(&mut self, proc: &'a Proc<'a>, span: SourceSpan) {
        self.stack.push(proc.ann(span));
        self.quote_mask.push(false);
    }

    fn into_proc(self) -> AnnProc<'a> {
        let stack = self.stack;
        assert!(
            stack.len() == 1,
            "bug: parsing finished prematurely\n.Remaining process stack: {stack:#?}"
        );
        assert!(
            self.quote_mask.last().is_some_and(|q| !q),
            "bug: the last process on the stack is quoted"
        );
        unsafe {
            // SAFETY: We check above that the stack contains exactly one element.
            *stack.last().unwrap_unchecked()
        }
    }

    fn to_proc_partial(&self) -> Option<AnnProc<'a>> {
        self.stack.last().copied()
    }

    #[inline(always)]
    unsafe fn replace_top_unchecked<F>(&mut self, replace: F)
    where
        F: FnOnce(AnnProc<'a>) -> AnnProc<'a>,
    {
        unsafe {
            let top = self.stack.last_mut().unwrap_unchecked();
            *top = replace(*top);
        }
    }

    #[inline]
    fn replace_top<F>(&mut self, replace: F) -> bool
    where
        F: FnOnce(AnnProc<'a>) -> AnnProc<'a>,
    {
        if self.stack.is_empty() {
            return false;
        }
        unsafe {
            self.replace_top_unchecked(replace);
        }
        true
    }

    #[inline(always)]
    unsafe fn replace_top2_unchecked<F>(&mut self, replace: F)
    where
        F: FnOnce(AnnProc<'a>, AnnProc<'a>) -> AnnProc<'a>,
    {
        let stack = &mut self.stack;
        unsafe {
            let top = stack.pop().unwrap_unchecked();
            let top_1 = stack.last_mut().unwrap_unchecked();
            *top_1 = replace(*top_1, top);
        }
        self.quote_mask.pop();
    }

    #[inline]
    fn replace_top2<F>(&mut self, replace: F) -> bool
    where
        F: FnOnce(AnnProc<'a>, AnnProc<'a>) -> AnnProc<'a>,
    {
        if self.stack.len() < 2 {
            return false;
        }
        unsafe {
            self.replace_top2_unchecked(replace);
        }
        true
    }

    #[inline(always)]
    unsafe fn replace_top3_unchecked<F>(&mut self, replace: F)
    where
        F: FnOnce(AnnProc<'a>, AnnProc<'a>, AnnProc<'a>) -> AnnProc<'a>,
    {
        let stack = &mut self.stack;
        unsafe {
            let top = stack.pop().unwrap_unchecked();
            let top_1 = stack.pop().unwrap_unchecked();
            let top_2 = stack.last_mut().unwrap_unchecked();
            *top_2 = replace(*top_2, top_1, top);
        }
        let quote_mask = &mut self.quote_mask;
        quote_mask.pop();
        quote_mask.pop();
    }

    #[inline]
    fn replace_top3<F>(&mut self, replace: F) -> bool
    where
        F: FnOnce(AnnProc<'a>, AnnProc<'a>, AnnProc<'a>) -> AnnProc<'a>,
    {
        if self.stack.len() < 3 {
            return false;
        }
        unsafe {
            self.replace_top3_unchecked(replace);
        }
        true
    }

    fn replace_top_slice_unchecked<F>(&mut self, n: usize, replace: F)
    where
        F: FnOnce(&[AnnProc<'a>]) -> AnnProc<'a>,
    {
        let stack = &mut self.stack;
        let top = stack.len();
        let split = top - n;
        let slice = &stack[split..];
        let result = replace(slice);
        stack.truncate(split);
        stack.push(result);
        let quote_mask = &mut self.quote_mask;
        quote_mask.truncate(split);
        quote_mask.push(false);
    }

    fn replace_top_slice<F>(&mut self, n: usize, replace: F) -> bool
    where
        F: FnOnce(&[AnnProc<'a>]) -> AnnProc<'a>,
    {
        if self.stack.len() < n {
            return false;
        }
        self.replace_top_slice_unchecked(n, replace);
        true
    }

    fn replace_top_slice_with_mask<F>(&mut self, n: usize, replace: F) -> bool
    where
        F: FnOnce(&[AnnProc<'a>], &BitSlice) -> AnnProc<'a>,
    {
        let stack = &mut self.stack;
        if stack.len() < n {
            return false;
        }
        let quote_mask = &mut self.quote_mask;
        let top = stack.len();
        let split = top - n;
        let slice = &stack[split..];
        let result = replace(slice, &quote_mask[split..]);
        stack.truncate(split);
        stack.push(result);
        quote_mask.truncate(split);
        quote_mask.push(false);
        true
    }

    #[inline]
    fn replace_top_with_mask<F>(&mut self, replace: F) -> bool
    where
        F: FnOnce(AnnProc<'a>, bool) -> AnnProc<'a>,
    {
        let stack = &mut self.stack;
        if stack.is_empty() {
            return false;
        }
        unsafe {
            let top = stack.last_mut().unwrap_unchecked();
            let mask = self.quote_mask.last_mut().unwrap_unchecked().replace(false);
            *top = replace(*top, mask);
        }
        true
    }

    #[inline(always)]
    unsafe fn mark_quote_unchecked(&mut self) {
        unsafe {
            self.quote_mask.last_mut().unwrap_unchecked().commit(true);
        }
    }

    #[inline]
    fn mark_quote(&mut self) -> bool {
        if self.stack.is_empty() {
            return false;
        }
        unsafe {
            self.mark_quote_unchecked();
        }
        true
    }
}

impl Debug for ProcStack<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ProcStack")
            .field(&self.stack)
            .field(&self.quote_mask)
            .finish()
    }
}

#[inline]
fn get_first_child<'a>(of: &tree_sitter::Node<'a>) -> tree_sitter::Node<'a> {
    of.named_child(0).unwrap_or_else(|| {
        panic!(
            "{:?} is expected to have a child node < {:?} >",
            of.kind(),
            of.to_sexp()
        )
    })
}

#[inline]
fn get_left_and_right<'a>(
    of: &tree_sitter::Node<'a>,
) -> (tree_sitter::Node<'a>, tree_sitter::Node<'a>) {
    of.named_child(0)
        .and_then(|left| of.named_child(1).map(|right| (left, right)))
        .unwrap_or_else(|| {
            panic!(
                "{:?} is expected to have two child nodes - left and right < {:?} >",
                of.kind(),
                of.to_sexp()
            )
        })
}

#[inline]
fn get_field<'a>(of: &tree_sitter::Node<'a>, id: u16) -> tree_sitter::Node<'a> {
    of.child_by_field_id(id).unwrap_or_else(|| {
        let rholang_language: tree_sitter::Language = rholang_tree_sitter::LANGUAGE.into();
        panic!(
            "{:?} is expected to have a field named {:?} < {:?} >",
            of.kind(),
            rholang_language.field_name_for_id(id),
            of.to_sexp()
        );
    })
}

#[inline]
fn get_node_value<'a>(node: &tree_sitter::Node, source: &'a str) -> &'a str {
    let source_bytes = source.as_bytes();
    unsafe {
        // SAFETY: source code is expected to contain valid utf8 and our grammar does not allow to
        // chop any single character. So, byte ranges of all nodes must start and end on valid UTF-8
        // slice
        str::from_utf8_unchecked(&source_bytes[node.byte_range()])
    }
}

fn named_children_of_kind<'a>(
    node: &tree_sitter::Node<'a>,
    kind: u16,
    cursor: &mut tree_sitter::TreeCursor<'a>,
) -> impl Iterator<Item = tree_sitter::Node<'a>> {
    node.named_children(cursor)
        .filter(move |child| child.kind_id() == kind)
}

#[derive(Debug, Clone, Copy)]
enum SourceDesc {
    Simple,
    RS,
    SR { arity: usize },
    // send_method_source: `name '!' method '(' inputs ')'`. Rewritten
    // at into_bind time into Source::SendReceive with a StringLiteral
    // method-name AnnProc prepended. The proc_stack slice carries the
    // pre-built StringLiteral AnnProc at index 1 (between the channel
    // and the actual inputs).
    SM { arity: usize },
}

impl SourceDesc {
    fn len(&self) -> usize {
        match self {
            SourceDesc::Simple | SourceDesc::RS => 1,
            SourceDesc::SR { arity } => *arity + 1,
            // 1 (channel) + 1 (method-literal AnnProc) + arity (real inputs)
            SourceDesc::SM { arity } => *arity + 2,
        }
    }
}

#[derive(Debug, Clone)]
enum BindDesc {
    Linear {
        name_count: usize,
        cont_present: bool,
        source: SourceDesc,
    },
    Repeated {
        name_count: usize,
        cont_present: bool,
    },
    Peek {
        name_count: usize,
        cont_present: bool,
    },
    // Per-clause signed bind `{% y <- x %}[s]` — a linear bind plus a signature.
    // Proc-slot layout = [source_name, (SR inputs), names…, sig_procs…]; the sig
    // is rebuilt from the trailing `sig_proc_count` procs via `sig_ops`.
    Signed {
        name_count: usize,
        cont_present: bool,
        source: SourceDesc,
        sig_ops: SmallVec<[SigOp; 4]>,
        sig_proc_count: usize,
    },
}

impl BindDesc {
    fn len(&self) -> usize {
        match self {
            BindDesc::Linear {
                name_count, source, ..
            } => name_count + source.len(),
            BindDesc::Signed {
                name_count,
                source,
                sig_proc_count,
                ..
            } => name_count + source.len() + sig_proc_count,
            BindDesc::Repeated { name_count, .. } | BindDesc::Peek { name_count, .. } => {
                name_count + 1
            }
        }
    }

    fn into_bind<'a>(self, procs: &[AnnProc<'a>], mask: &BitSlice) -> Bind<'a> {
        assert_eq!(procs.len(), self.len());
        unsafe {
            // SAFETY: We check above that the slice contains exactly |self.len()| elements which is
            // always > 0 by construction. The mask is also guaranteed to have the exact same length
            let (first, rest) = procs.split_first().unwrap_unchecked();
            let (q0, qs) = mask.split_first().unwrap_unchecked();

            let channel_name = into_name(*first, *q0);

            match self {
                BindDesc::Linear {
                    cont_present,
                    source,
                    ..
                } => {
                    let rhs = match source {
                        SourceDesc::Simple => Source::Simple { name: channel_name },
                        SourceDesc::RS => Source::ReceiveSend { name: channel_name },
                        SourceDesc::SR { arity } => {
                            let inputs = &rest[..arity];
                            Source::SendReceive {
                                name: channel_name,
                                inputs: inputs.to_smallvec(),
                            }
                        }
                        // send_method_source -> Source::SendReceive with
                        // method-name AnnProc already at rest[0] and
                        // actual inputs at rest[1..arity+1].
                        SourceDesc::SM { arity } => {
                            let inputs = &rest[..arity + 1];
                            Source::SendReceive {
                                name: channel_name,
                                inputs: inputs.to_smallvec(),
                            }
                        }
                    };

                    let lhs_start = source.len() - 1;
                    let lhs = into_names(&rest[lhs_start..], &qs[lhs_start..], cont_present);

                    Bind::Linear { lhs, rhs }
                }

                BindDesc::Repeated { cont_present, .. } => Bind::Repeated {
                    lhs: into_names(rest, qs, cont_present),
                    rhs: channel_name,
                },

                BindDesc::Peek { cont_present, .. } => Bind::Peek {
                    lhs: into_names(rest, qs, cont_present),
                    rhs: channel_name,
                },

                BindDesc::Signed {
                    cont_present,
                    source,
                    sig_ops,
                    sig_proc_count,
                    ..
                } => {
                    let rhs = match source {
                        SourceDesc::Simple => Source::Simple { name: channel_name },
                        SourceDesc::RS => Source::ReceiveSend { name: channel_name },
                        SourceDesc::SR { arity } => {
                            let inputs = &rest[..arity];
                            Source::SendReceive {
                                name: channel_name,
                                inputs: inputs.to_smallvec(),
                            }
                        }
                        // send_method_source -> Source::SendReceive with
                        // method-name AnnProc already at rest[0] and
                        // actual inputs at rest[1..arity+1].
                        SourceDesc::SM { arity } => {
                            let inputs = &rest[..arity + 1];
                            Source::SendReceive {
                                name: channel_name,
                                inputs: inputs.to_smallvec(),
                            }
                        }
                    };
                    // rest = [SR inputs…][names…][sig_procs…]; carve the names out
                    // between the source inputs and the trailing signature procs.
                    let lhs_start = source.len() - 1;
                    let names_end = rest.len() - sig_proc_count;
                    let lhs = into_names(
                        &rest[lhs_start..names_end],
                        &qs[lhs_start..names_end],
                        cont_present,
                    );
                    let sig = rebuild_signature(&sig_ops, &rest[names_end..]);
                    Bind::Signed { lhs, rhs, sig }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ReceiptDesc {
    parts: BindDescripts,
    len: usize,
    has_guard: bool,
}

struct BindIter<'slice, 'a, O>
where
    O: Iterator<Item = &'slice BindDesc> + ExactSizeIterator,
{
    iter: O,
    procs: &'slice [AnnProc<'a>],
    mask: &'slice BitSlice,
}

impl<'slice, 'a, O> Iterator for BindIter<'slice, 'a, O>
where
    O: Iterator<Item = &'slice BindDesc> + ExactSizeIterator,
{
    type Item = Bind<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|next| {
            let (this_procs, rest_procs) = self.procs.split_at(next.len());
            let (this_mask, rest_mask) = self.mask.split_at(next.len());

            self.procs = rest_procs;
            self.mask = rest_mask;

            next.clone().into_bind(this_procs, this_mask)
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact = self.iter.len();
        (exact, Some(exact))
    }
}

impl<'slice, 'a, O> ExactSizeIterator for BindIter<'slice, 'a, O>
where
    O: Iterator<Item = &'slice BindDesc> + ExactSizeIterator,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'slice, 'a, O> FusedIterator for BindIter<'slice, 'a, O> where
    O: Iterator<Item = &'slice BindDesc> + ExactSizeIterator
{
}

struct ReceiptIter<'slice, 'a, O>
where
    O: Iterator<Item = &'slice ReceiptDesc> + ExactSizeIterator,
{
    iter: O,
    procs: &'slice [AnnProc<'a>],
    mask: &'slice BitSlice,
}

impl<'slice, 'a, O> ReceiptIter<'slice, 'a, O>
where
    O: Iterator<Item = &'slice ReceiptDesc> + ExactSizeIterator,
{
    fn new(
        receipts: impl IntoIterator<Item = O::Item, IntoIter = O>,
        procs: &'slice [AnnProc<'a>],
        mask: &'slice BitSlice,
    ) -> Self {
        assert_eq!(procs.len(), mask.len());
        ReceiptIter {
            iter: receipts.into_iter(),
            procs,
            mask,
        }
    }
}

impl<'slice, 'a, O> Iterator for ReceiptIter<'slice, 'a, O>
where
    O: Iterator<Item = &'slice ReceiptDesc> + ExactSizeIterator,
{
    type Item = (
        BindIter<'slice, 'a, SliceIter<'slice, BindDesc>>,
        Option<AnnProc<'a>>,
    );

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|next| {
            let (this_procs, rest_procs) = self.procs.split_at(next.len);
            let (this_mask, rest_mask) = self.mask.split_at(next.len);

            self.procs = rest_procs;
            self.mask = rest_mask;

            // The guard proc — if any — sits at the end of this receipt's
            // slice (it was pushed last to temp_cont_stack so it ends up
            // last on proc_stack within this receipt's range).
            let (bind_procs, bind_mask, guard) = if next.has_guard {
                let last = this_procs.len() - 1;
                (
                    &this_procs[..last],
                    &this_mask[..last],
                    Some(this_procs[last]),
                )
            } else {
                (this_procs, this_mask, None)
            };

            (
                BindIter {
                    iter: next.parts.iter(),
                    procs: bind_procs,
                    mask: bind_mask,
                },
                guard,
            )
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact = self.iter.len();
        (exact, Some(exact))
    }
}

impl<'slice, 'a, O> ExactSizeIterator for ReceiptIter<'slice, 'a, O>
where
    O: Iterator<Item = &'slice ReceiptDesc> + ExactSizeIterator,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

impl<'slice, 'a, O> std::iter::FusedIterator for ReceiptIter<'slice, 'a, O> where
    O: Iterator<Item = &'slice ReceiptDesc> + ExactSizeIterator
{
}

#[derive(Debug, Clone, Copy)]
struct LetDecl {
    lhs_arity: usize,
    lhs_has_cont: bool,
    rhs_arity: usize,
    is_var_decl: bool,
}

struct LetDeclIter<'slice, 'a, I>
where
    I: Iterator<Item = &'slice LetDecl> + ExactSizeIterator,
{
    iter: I,
    procs: &'slice [AnnProc<'a>],
    mask: &'slice BitSlice,
}

impl<'slice, 'a, I> LetDeclIter<'slice, 'a, I>
where
    I: Iterator<Item = &'slice LetDecl> + ExactSizeIterator,
{
    fn new(
        decls: impl IntoIterator<Item = I::Item, IntoIter = I>,
        procs: &'slice [AnnProc<'a>],
        mask: &'slice BitSlice,
    ) -> Self {
        LetDeclIter {
            iter: decls.into_iter(),
            procs,
            mask,
        }
    }
}

impl<'slice, 'a, I> Iterator for LetDeclIter<'slice, 'a, I>
where
    I: Iterator<Item = &'slice LetDecl> + ExactSizeIterator,
{
    type Item = LetBinding<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(let_decl) = self.iter.next() {
            let split_point = let_decl.lhs_arity + let_decl.rhs_arity;
            let (this_procs, rest_procs) = self.procs.split_at(split_point);
            let (this_mask, rest_mask) = self.mask.split_at(split_point);

            let item = unsafe {
                // SAFETY: We check above that the slice contains exactly |lhs_arity + rhs_arity|
                // elements, and it is not zero. Therefore, lhs_arity <= slice.len()
                let (lhs, rhs) = this_procs.split_at_unchecked(let_decl.lhs_arity);
                LetBinding {
                    lhs: if let_decl.is_var_decl {
                        Names::single(into_name(lhs[0], true))
                    } else {
                        into_names(lhs, this_mask, let_decl.lhs_has_cont)
                    },
                    rhs: rhs.to_smallvec(),
                }
            };

            self.procs = rest_procs;
            self.mask = rest_mask;

            return Some(item);
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.iter.len(), None)
    }
}

/// Post-order opcode for rebuilding a cost-accounting `Signature` from a flat
/// proc slice. `Ground`/`Hash` each consume one proc; `Compound`/`Transfer`
/// fold the two most-recently-built signatures.
#[derive(Debug, Clone, Copy)]
enum SigOp {
    Ground,
    Hash,
    Compound,
    Transfer,
}

/// Walk a `signature` CST node, appending post-order `SigOp`s and pushing each
/// embedded proc (ground name `_proc_var`; hash body `#(P)`) into `procs` in
/// source order. The spine is shallow — only the hash body re-enters `_proc`.
fn flatten_signature<'a, const N: usize>(
    sig_node: &tree_sitter::Node<'a>,
    ops: &mut SmallVec<[SigOp; N]>,
    procs: &mut Vec<tree_sitter::Node<'a>>,
) {
    // `signature`/`stack_sig` are named wrappers; the stratified `_sig1`/`_sig2`
    // levels are hidden, so a compound/transfer operand is a bare sub-sig node.
    // Match the node directly, recursing through the wrappers.
    match sig_node.kind_id() {
        kind!("signature") | kind!("stack_sig") => {
            flatten_signature(&get_first_child(sig_node), ops, procs);
        }
        kind!("ground_sig") => {
            procs.push(get_field(sig_node, field!("name")));
            ops.push(SigOp::Ground);
        }
        kind!("hash_sig") => {
            procs.push(get_field(sig_node, field!("proc")));
            ops.push(SigOp::Hash);
        }
        kind!("compound_sig") => {
            flatten_signature(&get_field(sig_node, field!("left")), ops, procs);
            flatten_signature(&get_field(sig_node, field!("right")), ops, procs);
            ops.push(SigOp::Compound);
        }
        kind!("transfer_sig") => {
            flatten_signature(&get_field(sig_node, field!("left")), ops, procs);
            flatten_signature(&get_field(sig_node, field!("right")), ops, procs);
            ops.push(SigOp::Transfer);
        }
        _ => unreachable!("signature wraps ground/hash/compound/transfer"),
    }
}

/// Fold `ops` (post-order spine) + `procs` (the signature's embedded procs, in
/// source order) into a `Signature`. Ground sigs are bare names (never quoted).
fn rebuild_signature<'a>(ops: &[SigOp], procs: &[AnnProc<'a>]) -> Signature<'a> {
    let mut sig_stack: SmallVec<[Signature<'a>; 4]> = SmallVec::new();
    let mut p = 0usize;
    for op in ops {
        match op {
            SigOp::Ground => {
                let proc = procs[p];
                p += 1;
                // A well-formed ground sig is a bare name (`_proc_var`). Under
                // error recovery a missing/error node can land here as a `Bad`
                // proc; the overall parse already fails (errors are collected
                // separately), so fall back to a quote instead of panicking.
                let name = if matches!(proc.proc, Proc::ProcVar(_)) {
                    into_name(proc, false)
                } else {
                    Name::Quote(proc)
                };
                sig_stack.push(Signature::Ground(name));
            }
            SigOp::Hash => {
                let body = procs[p];
                p += 1;
                sig_stack.push(Signature::Hash(body));
            }
            SigOp::Compound => {
                let right = sig_stack.pop().expect("compound_sig needs a right operand");
                let left = sig_stack.pop().expect("compound_sig needs a left operand");
                sig_stack.push(Signature::Compound(Box::new(left), Box::new(right)));
            }
            SigOp::Transfer => {
                let right = sig_stack.pop().expect("transfer_sig needs a right operand");
                let left = sig_stack.pop().expect("transfer_sig needs a left operand");
                sig_stack.push(Signature::Transfer(Box::new(left), Box::new(right)));
            }
        }
    }
    sig_stack
        .pop()
        .expect("a signature spine yields exactly one Signature")
}

// process <-> name conversion
#[inline(always)]
fn into_name(ann_proc: AnnProc, quoted: bool) -> Name {
    if quoted {
        Name::Quote(ann_proc)
    } else {
        match ann_proc.proc {
            Proc::ProcVar(var) => Name::NameVar(*var),
            _ => panic!("invalid proc variant for into_name"),
        }
    }
}

#[inline]
fn into_remainder(ann_proc: AnnProc) -> Var {
    ann_proc
        .try_into()
        .expect("invalid remainder (not a proc_var)")
}

fn into_names<'slice, 'a>(
    procs: &'slice [AnnProc<'a>],
    mask: &'slice BitSlice,
    with_remainder: bool,
) -> Names<'a> {
    Names::from_iter(NamesIter::new(procs, mask), with_remainder).unwrap()
}

/// Assembles the agent-block desugaring tree at parse time, per
/// `Agents.md:19-35` + `Private-Methods.md:50-88`. The slice carries
/// the agent name at `slice[0]` followed by per-decl (body, formals)
/// AnnProcs in source order. The decl metadata in `decls_desc` tells
/// us how to route each slice element.
///
/// Output shape (`P` is `ctor_body`):
///
/// ```text
/// for (r, <ctor_formals> <= name) {
///   new this, private in {
///     for (...@args <= this)    { match args { /* pub  */ } } |
///     for (...@args <= private) { match args { /* priv */ } } |  (only if private decls)
///     P |
///     r!(bundle+{*this})
///   }
/// }
/// ```
///
/// Both `for`-comprehensions use `Bind::Repeated` (the persistent
/// `<=` operator) so the constructor channel and dispatch loops
/// stay live in the rspace. `this`, `private`, `return`, `args` are
/// literal names visible to user method bodies; `r` is a literal
/// reserved name (`__r`) -- the FIP freshness requirement is the
/// user's responsibility (matches the existing approach).
fn build_agent_desugaring<'ast>(
    builder: &'ast ASTBuilder<'ast>,
    decls_desc: &[AgentDeclDesc<'ast>],
    slice: &[AnnProc<'ast>],
    mask: &BitSlice,
    span: SourceSpan,
) -> AnnProc<'ast> {
    // Walk the slice in source order, partitioning into ctor / pub /
    // priv slots.
    let name = into_name(slice[0], mask[0]);
    let mut idx: usize = 1;

    let mut ctor: Option<ParsedCtor<'ast>> = None;
    let mut pub_methods: Vec<ParsedMethodLike<'ast>> = Vec::new();
    let mut priv_methods: Vec<ParsedMethodLike<'ast>> = Vec::new();
    let mut pub_default: Option<ParsedDefaultLike<'ast>> = None;
    let mut priv_default: Option<ParsedDefaultLike<'ast>> = None;

    for desc in decls_desc {
        let body = slice[idx];
        idx += 1;
        let (formals_opt, arity) = match desc {
            AgentDeclDesc::Constructor { arity, has_cont }
            | AgentDeclDesc::Method {
                arity, has_cont, ..
            }
            | AgentDeclDesc::Default {
                arity, has_cont, ..
            } => {
                let opt = if *arity > 0 {
                    let f =
                        into_names(&slice[idx..idx + arity], &mask[idx..idx + arity], *has_cont);
                    Some(f)
                } else {
                    None
                };
                (opt, *arity)
            }
        };
        idx += arity;

        match desc {
            AgentDeclDesc::Constructor { .. } => {
                // The visitor pushed a duplicate error already; just
                // keep the first one for desugaring purposes.
                if ctor.is_none() {
                    ctor = Some(ParsedCtor {
                        formals: formals_opt,
                        body,
                    });
                }
            }
            AgentDeclDesc::Method {
                name: method_name,
                is_private,
                ..
            } => {
                let entry = ParsedMethodLike {
                    name: *method_name,
                    formals: formals_opt,
                    body,
                };
                if *is_private {
                    priv_methods.push(entry);
                } else {
                    pub_methods.push(entry);
                }
            }
            AgentDeclDesc::Default { is_private, .. } => {
                let slot = if *is_private {
                    &mut priv_default
                } else {
                    &mut pub_default
                };
                if slot.is_none() {
                    *slot = Some(ParsedDefaultLike { body });
                }
            }
        }
    }

    // If the constructor is missing the visitor pushed an error; we
    // synthesize a Nil body so consume can still emit a tree-shaped
    // AnnProc rather than panic. The caller (node_to_ast) discards the
    // result on `bad`-mode anyway.
    let ctor = ctor.unwrap_or(ParsedCtor {
        formals: None,
        body: AnnProc {
            proc: builder.const_nil(),
            span,
        },
    });

    let has_priv_dispatch = !priv_methods.is_empty() || priv_default.is_some();

    // Fresh-name conventions. See the doc comment above.
    let r_id = id_at(builder.alloc_str("__r"), span.start);
    let args_id = id_at(builder.alloc_str("args"), span.start);
    let this_id = id_at(builder.alloc_str("this"), span.start);
    let private_id = id_at(builder.alloc_str("private"), span.start);
    let return_id = id_at(builder.alloc_str("return"), span.start);

    let r_name = Name::NameVar(Var::Id(r_id));
    let this_name = Name::NameVar(Var::Id(this_id));
    let private_name = Name::NameVar(Var::Id(private_id));

    let pub_dispatch = build_dispatch(
        builder,
        &pub_methods,
        pub_default.as_ref(),
        return_id,
        args_id,
        this_name,
        span,
    );

    let priv_dispatch = if has_priv_dispatch {
        Some(build_dispatch(
            builder,
            &priv_methods,
            priv_default.as_ref(),
            return_id,
            args_id,
            private_name,
            span,
        ))
    } else {
        None
    };

    // r!(bundle+{*this})
    let bundle_eval_this = ann(builder.alloc_eval(this_name), span);
    let bundle_write = ann(
        builder.alloc_bundle(BundleType::BundleWrite, bundle_eval_this),
        span,
    );
    let reply_send = ann(
        builder.alloc_send(SendType::Single, r_name, &[bundle_write]),
        span,
    );

    // Par chain: [priv_dispatch |] pub_dispatch | Pc | reply_send
    let mut combined = pub_dispatch;
    if let Some(pd) = priv_dispatch {
        combined = ann(builder.alloc_par(pd, combined), span);
    }
    combined = ann(builder.alloc_par(combined, ctor.body), span);
    combined = ann(builder.alloc_par(combined, reply_send), span);

    // new this, private in { combined }
    let new_decls = vec![
        NameDecl {
            id: this_id,
            uri: None,
        },
        NameDecl {
            id: private_id,
            uri: None,
        },
    ];
    let new_this_in = ann(builder.alloc_new(combined, new_decls), span);

    // for(r, <ctor_formals> <= name) { new_this_in }
    let outer_lhs = {
        let mut names: SmallVec<[Name<'ast>; 1]> = SmallVec::new();
        names.push(r_name);
        let mut remainder: Option<Var<'ast>> = None;
        if let Some(ctor_formals) = ctor.formals {
            for n in &ctor_formals.names {
                names.push(*n);
            }
            remainder = ctor_formals.remainder;
        }
        Names { names, remainder }
    };
    let outer_bind = Bind::Repeated {
        lhs: outer_lhs,
        rhs: name,
    };
    ann(builder.alloc_for([[outer_bind]], new_this_in), span)
}

/// Collect every identifier name appearing anywhere inside
/// `root`. Used by [`build_try_block_desugaring`] to guarantee
/// the synthesized `__ok` / `__rest` bindings don't shadow a
/// user identifier in the try/catch bodies or patterns.
///
/// Correctness is by conservative over-approximation: bindings
/// AND references are collected in every position (proc vars,
/// name vars, method names, `new` decls, VarRef, remainders of
/// `let` / `for` / `contract` pattern lists, and cost-accounting
/// signatures in signed terms, token stacks, and signed binds).
/// Over-collecting only makes us pick a slightly less pretty
/// fresh name; it never causes an incorrect capture.
fn collect_ids_into<'ast>(root: AnnProc<'ast>, out: &mut BTreeSet<&'ast str>) {
    use crate::ast::{
        Case, Collection, LetBinding, Name, NameDecl, Names, Proc, Source, SyncSendCont,
    };

    // Iterative DFS with a work-queue of AnnProc values (Copy) so
    // borrow-checker lifetimes stay simple: every reference we
    // walk lives for `'ast` via the arena, but the queue itself
    // only holds values.
    let mut stack: SmallVec<[AnnProc<'ast>; 32]> = SmallVec::new();
    stack.push(root);

    while let Some(ann) = stack.pop() {
        match ann.proc {
            Proc::ProcVar(v) => {
                if let Var::Id(id) = v {
                    out.insert(id.name);
                }
            }
            Proc::VarRef { var, .. } => {
                out.insert(var.name);
            }
            Proc::Method {
                receiver,
                name,
                args,
            } => {
                out.insert(name.name);
                stack.push(*receiver);
                for a in args.iter() {
                    stack.push(*a);
                }
            }
            Proc::New { decls, proc: inner } => {
                collect_from_new_decls(decls, out);
                stack.push(*inner);
            }
            Proc::Par { left, right } | Proc::BinaryExp { left, right, .. } => {
                stack.push(*left);
                stack.push(*right);
            }
            Proc::UnaryExp { arg, .. } => {
                stack.push(*arg);
            }
            Proc::Bundle { proc: inner, .. } => {
                stack.push(*inner);
            }
            Proc::IfThenElse {
                condition,
                if_true,
                if_false,
            } => {
                stack.push(*condition);
                stack.push(*if_true);
                if let Some(p) = if_false {
                    stack.push(*p);
                }
            }
            Proc::Send {
                channel, inputs, ..
            } => {
                collect_from_name(*channel, out, &mut stack);
                for i in inputs.iter() {
                    stack.push(*i);
                }
            }
            Proc::SendSync {
                channel,
                inputs,
                cont,
            } => {
                collect_from_name(*channel, out, &mut stack);
                for i in inputs.iter() {
                    stack.push(*i);
                }
                if let SyncSendCont::NonEmpty(p) = cont {
                    stack.push(*p);
                }
            }
            Proc::Eval { name } => {
                collect_from_name(*name, out, &mut stack);
            }
            Proc::Contract {
                name,
                formals,
                body,
            } => {
                collect_from_name(*name, out, &mut stack);
                collect_from_names(formals, out, &mut stack);
                stack.push(*body);
            }
            Proc::Let { bindings, body, .. } => {
                for LetBinding { lhs, rhs } in bindings.iter() {
                    collect_from_names(lhs, out, &mut stack);
                    for p in rhs.iter() {
                        stack.push(*p);
                    }
                }
                stack.push(*body);
            }
            Proc::ForComprehension {
                receipts,
                proc: body,
            } => {
                for r in receipts.iter() {
                    for b in r.binds.iter() {
                        collect_from_names(b.names(), out, &mut stack);
                        match b {
                            Bind::Linear { rhs, .. } => {
                                collect_from_source(rhs, out, &mut stack);
                            }
                            Bind::Repeated { rhs, .. } | Bind::Peek { rhs, .. } => {
                                collect_from_name(*rhs, out, &mut stack);
                            }
                            // Signed bind `{% y <- x %}[s]`: the rendezvous
                            // source is walked like Linear, and the funding
                            // signature is walked so its identifiers count
                            // toward hygiene too.
                            Bind::Signed { rhs, sig, .. } => {
                                collect_from_source(rhs, out, &mut stack);
                                collect_from_signature(sig, out, &mut stack);
                            }
                        }
                    }
                    if let Some(g) = r.guard {
                        stack.push(g);
                    }
                }
                stack.push(*body);
            }
            Proc::Match { expression, cases } => {
                stack.push(*expression);
                for Case {
                    pattern,
                    guard,
                    proc: body,
                } in cases.iter()
                {
                    stack.push(*pattern);
                    if let Some(g) = guard {
                        stack.push(*g);
                    }
                    stack.push(*body);
                }
            }
            Proc::Collection(coll) => match coll {
                Collection::List {
                    elements,
                    remainder,
                }
                | Collection::Set {
                    elements,
                    remainder,
                } => {
                    for e in elements.iter() {
                        stack.push(*e);
                    }
                    if let Some(Var::Id(id)) = remainder {
                        out.insert(id.name);
                    }
                }
                Collection::PathMap { elements, .. } | Collection::Tuple(elements) => {
                    for e in elements.iter() {
                        stack.push(*e);
                    }
                }
                Collection::Map {
                    elements,
                    remainder,
                } => {
                    for (k, v) in elements.iter() {
                        stack.push(*k);
                        stack.push(*v);
                    }
                    if let Some(Var::Id(id)) = remainder {
                        out.insert(id.name);
                    }
                }
            },
            // Cost-accounted forms: walk the embedded process and
            // every funding signature so hygiene sees identifiers
            // inside `{% P %}[s]` terms and token stacks.
            Proc::SignedTerm { proc: inner, sig } => {
                stack.push(*inner);
                collect_from_signature(sig, out, &mut stack);
            }
            Proc::TokenStack { stack: token_stack } => {
                for sig in token_stack.layers.iter() {
                    collect_from_signature(sig, out, &mut stack);
                }
            }
            // Pure leaves without embedded identifiers.
            Proc::Nil
            | Proc::Unit
            | Proc::BoolLiteral(_)
            | Proc::LongLiteral(_)
            | Proc::SignedIntLiteral { .. }
            | Proc::UnsignedIntLiteral { .. }
            | Proc::BigIntLiteral(_)
            | Proc::BigRatLiteral(_)
            | Proc::FloatLiteral { .. }
            | Proc::FixedPointLiteral { .. }
            | Proc::StringLiteral(_)
            | Proc::UriLiteral(_)
            | Proc::SimpleType(_)
            | Proc::Bad => {}
            Proc::Select { .. } => {
                // Select is unimplemented in this Rholang version;
                // matches the same guard in `traverse::PreorderDfsIter`.
                unimplemented!("Select is not implemented in this version of Rholang")
            }
        }
    }

    fn collect_from_name<'ast>(
        name: Name<'ast>,
        out: &mut BTreeSet<&'ast str>,
        stack: &mut SmallVec<[AnnProc<'ast>; 32]>,
    ) {
        match name {
            Name::NameVar(Var::Id(id)) => {
                out.insert(id.name);
            }
            Name::NameVar(Var::Wildcard) => {}
            Name::Quote(inner) => {
                stack.push(inner);
            }
        }
    }

    fn collect_from_names<'ast>(
        names: &Names<'ast>,
        out: &mut BTreeSet<&'ast str>,
        stack: &mut SmallVec<[AnnProc<'ast>; 32]>,
    ) {
        for n in names.names.iter() {
            collect_from_name(*n, out, stack);
        }
        if let Some(Var::Id(id)) = names.remainder {
            out.insert(id.name);
        }
    }

    fn collect_from_new_decls<'ast>(decls: &[NameDecl<'ast>], out: &mut BTreeSet<&'ast str>) {
        for d in decls.iter() {
            out.insert(d.id.name);
        }
    }

    fn collect_from_source<'ast>(
        source: &Source<'ast>,
        out: &mut BTreeSet<&'ast str>,
        stack: &mut SmallVec<[AnnProc<'ast>; 32]>,
    ) {
        match source {
            Source::Simple { name } | Source::ReceiveSend { name } => {
                collect_from_name(*name, out, stack);
            }
            Source::SendReceive { name, inputs } => {
                collect_from_name(*name, out, stack);
                for i in inputs.iter() {
                    stack.push(*i);
                }
            }
        }
    }

    fn collect_from_signature<'ast>(
        sig: &Signature<'ast>,
        out: &mut BTreeSet<&'ast str>,
        stack: &mut SmallVec<[AnnProc<'ast>; 32]>,
    ) {
        match sig {
            Signature::Ground(name) => {
                collect_from_name(*name, out, stack);
            }
            Signature::Hash(inner) => {
                stack.push(*inner);
            }
            Signature::Compound(left, right) | Signature::Transfer(left, right) => {
                collect_from_signature(left, out, stack);
                collect_from_signature(right, out, stack);
            }
        }
    }
}

/// Pick a fresh identifier name based on `base` that isn't
/// present in `avoid`. Tries `base` first, then `base0`, `base1`,
/// ... until it finds a free slot. Inserts the chosen name into
/// `avoid` so a subsequent call to `pick_fresh` (e.g. for
/// `__rest` after `__ok`) doesn't reuse it.
///
/// The returned `&'ast str` lives in the AST arena via
/// `builder.alloc_str`, so it satisfies the same lifetime
/// contract as any other identifier in the produced tree.
fn pick_fresh<'ast>(
    builder: &'ast ASTBuilder<'ast>,
    base: &str,
    pos: SourcePos,
    avoid: &mut BTreeSet<&'ast str>,
) -> Id<'ast> {
    for n in 0usize.. {
        let candidate: String = if n == 0 {
            base.to_string()
        } else {
            format!("{base}{}", n - 1)
        };
        if !avoid.contains(candidate.as_str()) {
            let stored: &'ast str = builder.alloc_str(&candidate);
            avoid.insert(stored);
            return id_at(stored, pos);
        }
    }
    unreachable!("pick_fresh exhausted the entire usize range without finding a free name")
}

/// Assemble the try/catch desugaring per FIP 2026-02-06
/// §"Error syntax". Called from apply_cont at K::ConsumeTryBlock
/// after all pieces have been evaluated onto proc_stack.
///
/// The slice layout, driven by the push order in the visitor case
/// for `kind!("try_block")`, is (bottom to top):
///
///   [0 .. source_desc.len()]   source pieces (name, [method_lit], [inputs...])
///   [+0 or +1]                 try_pattern (if has_try_pattern)
///   [+1]                       try_body
///   [+1]                       catch_pattern
///   [+1]                       catch_body
///
/// The `mask` bit at each index tells whether the corresponding
/// slot came from a `quote` node (needs `Name::Quote`) vs a proc-
/// var (`Name::NameVar`) -- driven by the same
/// `mark_quote` / `into_name` machinery the for-comp path uses.
///
/// The desugared shape is
///   for (@[__ok, ...__rest] <- <source>) {
///     if (__ok) {
///       let @<try_pat> <- __rest in { <try_body> }
///     } else {
///       let @<catch_pat> <- __rest in { <catch_body> }
///     }
///   }
///
/// with the try-branch `let` dropped when `has_try_pattern` is
/// false (bare `try <-` form for methods that return `[true]` alone).
///
/// # Hygiene
///
/// The outer `for` introduces two synthetic bindings (`__ok`,
/// `__rest`) that would silently capture any user reference to
/// those identifiers inside the try/catch bodies or patterns.
/// [`collect_ids_into`] scans all four subtrees for identifier
/// occurrences and [`pick_fresh`] chooses names that don't
/// collide -- falling back to `__ok0`, `__ok1`, ... when the base
/// name is taken. When the user writes nothing named `__ok` /
/// `__rest`, the emitted names are unchanged (so the existing
/// desugared-corpus tests keep passing).
fn build_try_block_desugaring<'ast>(
    builder: &'ast ASTBuilder<'ast>,
    source_desc: SourceDesc,
    has_try_pattern: bool,
    slice: &[AnnProc<'ast>],
    mask: &BitSlice,
    span: SourceSpan,
) -> AnnProc<'ast> {
    let src_len = source_desc.len();

    // Reconstruct Source from the leading source_desc.len() slots.
    // Same shape the for-comprehension consumer builds via
    // BindDesc::to_bind's Linear arm.
    let channel_name = into_name(slice[0], mask[0]);
    let source = match source_desc {
        SourceDesc::Simple => Source::Simple { name: channel_name },
        SourceDesc::RS => Source::ReceiveSend { name: channel_name },
        // Both send-shaped sources collapse to `SendReceive`. The
        // inputs slice is uniformly `slice[1..src_len]` -- every
        // source slot after the channel name at `slice[0]`. For SR
        // that's just the `arity` user-supplied inputs; for SM the
        // visitor's `send_method_source` handler has already
        // prepended the method-name literal at `slice[1]`, so the
        // same slice folds `[method_literal, input_0, ..,
        // input_{arity-1}]` into `SendReceive.inputs` -- exactly
        // what the for-comprehension consumer expects to see for a
        // `chan!method(args)` source. Matches `BindDesc::to_bind`'s
        // linear-SM arm at ~line 2170.
        SourceDesc::SR { .. } | SourceDesc::SM { .. } => Source::SendReceive {
            name: channel_name,
            inputs: slice[1..src_len].to_smallvec(),
        },
    };

    // Slot offsets for the rest of the pieces.
    let try_pat_off = src_len;
    let try_body_off = try_pat_off + has_try_pattern as usize;
    let catch_pat_off = try_body_off + 1;
    let catch_body_off = catch_pat_off + 1;

    let try_pat_ann = has_try_pattern.then(|| slice[try_pat_off]);
    let try_pat_q = has_try_pattern.then(|| mask[try_pat_off]);
    let try_body = slice[try_body_off];
    let catch_pat = into_name(slice[catch_pat_off], mask[catch_pat_off]);
    let catch_body = slice[catch_body_off];

    // Fresh internal names for the outer @[__ok, ...__rest] pattern.
    // These bindings are introduced BY the desugaring; if the user's
    // try/catch bodies or patterns reference an identifier of the
    // same name, our synthesized binding would shadow that reference
    // and change its meaning silently. Hygiene: scan the four
    // subtrees (both bodies, both patterns) for every identifier
    // occurrence, then pick names that don't collide -- starting
    // with the intuitive `__ok`/`__rest` and appending a numeric
    // suffix if needed. Corpus tests continue to pass because their
    // bodies don't mention either name; only user code that
    // deliberately (or accidentally) uses those identifiers triggers
    // renaming.
    let mut avoid: BTreeSet<&'ast str> = BTreeSet::new();
    if let Some(try_pat) = try_pat_ann {
        collect_ids_into(try_pat, &mut avoid);
    }
    collect_ids_into(try_body, &mut avoid);
    collect_ids_into(slice[catch_pat_off], &mut avoid);
    collect_ids_into(catch_body, &mut avoid);
    let ok_id = pick_fresh(builder, "__ok", span.start, &mut avoid);
    let rest_id = pick_fresh(builder, "__rest", span.start, &mut avoid);
    let ok_var = Var::Id(ok_id);
    let rest_var = Var::Id(rest_id);

    // @[__ok, ...__rest]
    let ok_pat_ann = ann(builder.alloc_proc_var(ok_var), span);
    let outer_list = ann(
        builder.alloc_list_with_remainder(&[ok_pat_ann], rest_var),
        span,
    );
    let outer_pattern = Name::Quote(outer_list);

    // __rest as a proc, used as the RHS of the two `let` bindings.
    let rest_rhs = ann(builder.alloc_proc_var(rest_var), span);

    // Try branch: [let @<try_pat> <- __rest in] try_body
    let try_branch = if has_try_pattern {
        let try_pat_name = into_name(try_pat_ann.unwrap(), try_pat_q.unwrap());
        let binding = LetBinding::single(try_pat_name, rest_rhs);
        ann(builder.alloc_let([binding], try_body, false), span)
    } else {
        try_body
    };

    // Catch branch: let @<catch_pat> <- __rest in catch_body
    let catch_binding = LetBinding::single(catch_pat, rest_rhs);
    let catch_branch = ann(builder.alloc_let([catch_binding], catch_body, false), span);

    // if (__ok) { try_branch } else { catch_branch }
    let ok_cond = ann(builder.alloc_proc_var(ok_var), span);
    let if_expr = ann(
        builder.alloc_if_then_else(ok_cond, try_branch, catch_branch),
        span,
    );

    // for (@[__ok, ...__rest] <- <source>) { if_expr }
    let outer_bind = Bind::Linear {
        lhs: Names {
            names: {
                let mut ns: SmallVec<[Name<'ast>; 1]> = SmallVec::new();
                ns.push(outer_pattern);
                ns
            },
            remainder: None,
        },
        rhs: source,
    };
    ann(builder.alloc_for([[outer_bind]], if_expr), span)
}

/// Build one dispatch loop:
///
/// ```text
/// for (...@args_id <= channel) {
///   match args_id {
///     [*return, "methodName", <formals>] => method_body
///     ...
///     _ => default_body (or Nil if missing)
///   }
/// }
/// ```
///
/// `methods` and `default` are assumed to be of the same visibility
/// (the caller partitioned them).
fn build_dispatch<'ast>(
    builder: &'ast ASTBuilder<'ast>,
    methods: &[ParsedMethodLike<'ast>],
    default: Option<&ParsedDefaultLike<'ast>>,
    return_id: Id<'ast>,
    args_id: Id<'ast>,
    channel: Name<'ast>,
    span: SourceSpan,
) -> AnnProc<'ast> {
    let mut cases: Vec<AnnProc<'ast>> = Vec::with_capacity(2 * (methods.len() + 1));

    for m in methods {
        let mut elements: Vec<AnnProc<'ast>> = Vec::new();
        // *return
        elements.push(ann(
            builder.alloc_eval(Name::NameVar(Var::Id(return_id))),
            span,
        ));
        // "methodName"
        elements.push(ann(builder.alloc_string_literal(m.name.name), span));
        // formals as proc-patterns
        let remainder = if let Some(formals) = &m.formals {
            for n in &formals.names {
                elements.push(name_to_proc_pattern(builder, n, span));
            }
            formals.remainder
        } else {
            None
        };
        let pattern_list = ann(
            if let Some(rem) = remainder {
                builder.alloc_list_with_remainder(&elements, rem)
            } else {
                builder.alloc_list(&elements)
            },
            span,
        );
        cases.push(pattern_list);
        cases.push(m.body);
    }

    // Default arm: wildcard => default_body (or Nil if missing).
    let default_body = match default {
        Some(d) => d.body,
        None => ann(builder.const_nil(), span),
    };
    cases.push(ann(builder.alloc_proc_var(Var::Wildcard), span));
    cases.push(default_body);

    let args_eval = ann(builder.alloc_proc_var(Var::Id(args_id)), span);
    let dispatch_match = ann(builder.alloc_match(args_eval, &cases), span);

    // for(...@args <= channel) { match args { ... } }
    let names = Names {
        names: SmallVec::new(),
        remainder: Some(Var::Id(args_id)),
    };
    let bind = Bind::Repeated {
        lhs: names,
        rhs: channel,
    };
    ann(builder.alloc_for([[bind]], dispatch_match), span)
}

// Convert a name pattern (used in for/contract/agent formals) into a
// Proc-position pattern element for use inside a match list pattern.
fn name_to_proc_pattern<'ast>(
    builder: &'ast ASTBuilder<'ast>,
    n: &Name<'ast>,
    span: SourceSpan,
) -> AnnProc<'ast> {
    match n {
        Name::NameVar(var) => ann(builder.alloc_proc_var(*var), span),
        Name::Quote(inner) => *inner,
    }
}

#[inline]
fn ann<'a>(proc: &'a Proc<'a>, span: SourceSpan) -> AnnProc<'a> {
    AnnProc { proc, span }
}

#[inline]
fn id_at(name: &str, pos: SourcePos) -> Id<'_> {
    Id { name, pos }
}

// Parsed-decl staging types shared by build_agent_desugaring and
// build_dispatch. Each captures the data extracted from a single
// declaration (body, formals, and method name where applicable),
// ready for assembly into the desugared tree.
struct ParsedCtor<'a> {
    formals: Option<Names<'a>>,
    body: AnnProc<'a>,
}
struct ParsedMethodLike<'a> {
    name: Id<'a>,
    formals: Option<Names<'a>>,
    body: AnnProc<'a>,
}
struct ParsedDefaultLike<'a> {
    body: AnnProc<'a>,
}

pub struct NamesIter<'slice, 'a> {
    procs: &'slice [AnnProc<'a>],
    mask: &'slice BitSlice,
    front: usize,
    back: usize,
}

impl<'slice, 'a> NamesIter<'slice, 'a> {
    pub fn new(procs: &'slice [AnnProc<'a>], mask: &'slice BitSlice) -> Self {
        assert!(procs.len() <= mask.len());
        let len = procs.len();
        Self {
            procs,
            mask,
            front: 0,
            back: len,
        }
    }
}

impl<'slice, 'a> Iterator for NamesIter<'slice, 'a> {
    type Item = Name<'a>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.front < self.back {
            let i = self.front;
            self.front = i + 1;
            unsafe {
                let proc = self.procs.get_unchecked(i);
                let quoted = *self.mask.get_unchecked(i);
                Some(into_name(*proc, quoted))
            }
        } else {
            None
        }
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.back - self.front;
        (remaining, Some(remaining))
    }
}

impl<'slice, 'a> DoubleEndedIterator for NamesIter<'slice, 'a> {
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.front < self.back {
            self.back -= 1;
            let i = self.back;
            unsafe {
                let proc = self.procs.get_unchecked(i);
                let quoted = *self.mask.get_unchecked(i);
                Some(into_name(*proc, quoted))
            }
        } else {
            None
        }
    }
}

impl<'slice, 'a> ExactSizeIterator for NamesIter<'slice, 'a> {}
impl<'slice, 'a> FusedIterator for NamesIter<'slice, 'a> {}
