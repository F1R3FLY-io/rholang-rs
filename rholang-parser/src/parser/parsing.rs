use bitvec::slice::BitSlice;
use bitvec::slice::Iter as MaskIter;
use bitvec::vec::BitVec;
use nonempty_collections::NEVec;
use rholang_tree_sitter_proc_macro::{field, kind};
use smallvec::{SmallVec, ToSmallVec};
use std::fmt::Debug;
use std::iter::{FusedIterator, Zip};
use std::slice::Iter as SliceIter;
use validated::Validated;

use crate::SourcePos;
use crate::ast::{Name, Var};
use crate::parser::errors::{self, ParsingFailure};
use crate::{
    SourceSpan,
    ast::{
        AnnProc, BinaryExpOp, Bind, BundleType, Id, LetBinding, NameDecl, Names, Proc, SendType,
        SimpleType, Source, UnaryExpOp, VarRefKind,
    },
    parser::{
        ast_builder::ASTBuilder,
        errors::{AnnParsingError, ParsingError},
    },
};

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
                | kind!("conjunction")
                | kind!("pathmap_subtract")
                | kind!("pathmap_restrict") => {
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
                            kind!("conjunction") => BinaryExpOp::Conjunction,
                            kind!("pathmap_subtract") => BinaryExpOp::PathmapSubtract,
                            _ => BinaryExpOp::PathmapRestrict,
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

                kind!("pathmap_drop") => {
                    let count_node = get_field(&node, field!("count"));
                    let pathmap_node = get_field(&node, field!("pathmap"));
                    
                    let count_value = get_node_value(&count_node, source);
                    match count_value.parse::<i64>() {
                        Ok(count) => {
                            cont_stack.push(K::ConsumePathmapDrop { count, span });
                            node = pathmap_node;
                            continue 'parse;
                        }
                        Err(_) => {
                            errors.push(AnnParsingError::new(ParsingError::NumberOutOfRange, &count_node));
                            bad = true;
                        }
                    }
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
                            let mut temp_cont_stack =
                                Vec::with_capacity(collection_node.named_child_count() * 2);
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
                            let arity = collection_node.named_child_count();
                            if arity == 0 {
                                proc_stack.push(ast_builder.const_empty_pathmap(), span);
                            } else {
                                cont_stack.push(K::ConsumePathmap {
                                    arity,
                                    has_remainder,
                                    span,
                                });
                                cont_stack.push(K::EvalList(collection_node.walk()));
                            }
                        }
                        _ => unreachable!("Rholang collections are: list, set, tuple, map and pathmap"),
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

                kind!("new") => {
                    fn check_for_duplicate_decls(
                        decls: &[NameDecl],
                    ) -> Option<(SourcePos, SourcePos)> {
                        decls.windows(2).find_map(|w| {
                            if w[0] == w[1] {
                                let mut first = w[0].id.pos;
                                let mut second = w[1].id.pos;
                                if second < first {
                                    std::mem::swap(&mut first, &mut second);
                                }
                                Some((first, second))
                            } else {
                                None
                            }
                        })
                    }

                    let decls_node = get_field(&node, field!("decls"));
                    let proc_node = get_field(&node, field!("proc"));

                    let mut decls = parse_decls(&decls_node, source);
                    decls.sort_unstable();
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
                            has_cont: formals_node.child_by_field_name("cont").is_some(),
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
                    let mut temp_cont_stack = Vec::with_capacity(rs.capacity() * 2);

                    let mut total_len = 0;

                    for receipt_node in receipts_node.named_children(&mut receipts_node.walk()) {
                        let mut bs = SmallVec::with_capacity(receipt_node.named_child_count());
                        let mut receipt_len = 0;

                        for bind_node in receipt_node.named_children(&mut receipt_node.walk()) {
                            let (names_node, source_node) = if bind_node.named_child_count() > 1 {
                                let (ns, s) = get_left_and_right(&bind_node);
                                (Some(ns), s)
                            } else {
                                (None, get_first_child(&bind_node))
                            };
                            let (name_count, cont_present) = match names_node {
                                Some(names) => (
                                    names.named_child_count(),
                                    names.child_by_field_id(field!("cont")).is_some(),
                                ),
                                None => (0, false),
                            };

                            let bind_desc = match bind_node.kind_id() {
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
                                        _ => unreachable!(
                                            "Sources in for-comprehensions have three kinds: simple, receive_send and send_receive"
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
                                _ => unreachable!(
                                    "There are only three types of binds in for-comprehensions: linear, repeated and peek"
                                ),
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

                            bs.push(bind_desc);
                            receipt_len += bind_desc.len();
                        }
                        rs.push(ReceiptDesc {
                            parts: bs,
                            len: receipt_len,
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

                    let mut temp_cont_stack =
                        Vec::with_capacity(2 * cases_node.named_child_count());
                    let arity = eval_named_pairs(
                        &cases_node,
                        kind!("case"),
                        field!("pattern"),
                        field!("proc"),
                        &mut temp_cont_stack,
                    );

                    cont_stack.push(K::ConsumeMatch { span, arity });
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
                    let mut temp_cont_stack = Vec::with_capacity(2 * let_decls.capacity());

                    let mut total_len = 0;

                    for decl_node in decls_node.named_children(&mut decls_node.walk()) {
                        let (lhs, rhs) = get_left_and_right(&decl_node);
                        let lhs_arity = lhs.named_child_count();
                        let rhs_arity = rhs.named_child_count();
                        let lhs_has_cont = lhs.child_by_field_id(field!("cont")).is_some();

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
                        let_decls.push(LetDecl {
                            lhs_arity,
                            lhs_has_cont,
                            rhs_arity,
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

                kind!("var_ref") => {
                    let (var_ref_kind_node, var_node) = get_left_and_right(&node);

                    let kind = get_first_child(&var_ref_kind_node).kind();

                    let var_ref_kind = match kind {
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

                _ => unimplemented!(),
            }
        }

        if bad {
            proc_stack.push(ast_builder.bad_const(), node.range().into());
        }
        loop {
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
                    let last = proc_stack.to_proc();
                    return Validated::Good(last);
                }
                Step::Continue(n) => {
                    node = n;
                    continue 'parse;
                }
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

fn apply_cont<'tree, 'ast>(
    cont_stack: &mut Vec<K<'tree, 'ast>>,
    proc_stack: &mut ProcStack<'ast>,
    ast_builder: &'ast ASTBuilder<'ast>,
) -> Step<'tree> {
    fn move_cursor_to_named(cursor: &mut tree_sitter::TreeCursor) -> bool {
        let mut has_more = if cursor.depth() == 0 {
            cursor.goto_first_child()
        } else {
            cursor.goto_next_sibling()
        };
        while has_more && !cursor.node().is_named() {
            has_more = cursor.goto_next_sibling();
        }
        has_more
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
                if move_cursor_to_named(cursor) {
                    return Step::Continue(cursor.node());
                }
                cont_stack.pop();
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
                                    .alloc_for(ReceiptIter::new(&desc, procs, &mask[1..]), body)
                                    .ann(span)
                            },
                        ),
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
                        K::ConsumeMatch { span, arity } => {
                            proc_stack.replace_top_slice(arity * 2 + 1, |expr_cases| {
                                let expr = expr_cases[0];
                                let cases = &expr_cases[1..];
                                ast_builder.alloc_match(expr, cases).ann(span)
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
                        K::ConsumePathmap {
                            arity,
                            has_remainder,
                            span,
                        } => proc_stack.replace_top_slice(arity, |elems| {
                            let pathmap = if has_remainder {
                                assert!(!elems.is_empty());
                                // SAFETY: We have checked above that there is at least one element
                                let (last, init) = elems.split_last().unwrap_unchecked();
                                ast_builder.alloc_pathmap_with_remainder(init, into_remainder(*last))
                            } else {
                                ast_builder.alloc_pathmap(elems)
                            };
                            pathmap.ann(span)
                        }),
                        K::ConsumePathmapDrop { count, span } => proc_stack
                            .replace_top(|pathmap| ast_builder.alloc_pathmap_drop(count, pathmap).ann(span)),
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
    ConsumeMatch {
        span: SourceSpan,
        arity: usize,
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
    ConsumePathmap {
        arity: usize,
        has_remainder: bool,
        span: SourceSpan,
    },
    ConsumePathmapDrop {
        count: i64,
        span: SourceSpan,
    },
    EvalDelayed(tree_sitter::Node<'tree>),
    EvalList(tree_sitter::TreeCursor<'tree>),
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
            Self::ConsumeMatch { span, arity } => f
                .debug_struct("ConsumeMatch")
                .field("arity", arity)
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
            Self::ConsumePathmap { arity, span, .. } => f
                .debug_struct("ConsumePathmap")
                .field("arity", arity)
                .field("span", span)
                .finish(),
            Self::ConsumePathmapDrop { count, span } => f
                .debug_struct("ConsumePathmapDrop")
                .field("count", count)
                .field("span", span)
                .finish(),
            Self::EvalDelayed(arg0) => f.debug_tuple("EvalDelayed").field(arg0).finish(),
            Self::EvalList(arg0) => f
                .debug_struct("EvalList")
                .field("at", &arg0.node())
                .finish(),
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

    fn to_proc(self) -> AnnProc<'a> {
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
}

impl SourceDesc {
    fn len(&self) -> usize {
        match self {
            SourceDesc::Simple | SourceDesc::RS => 1,
            SourceDesc::SR { arity } => *arity + 1,
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
}

impl BindDesc {
    fn len(&self) -> usize {
        match self {
            BindDesc::Linear {
                name_count, source, ..
            } => name_count + source.len(),
            BindDesc::Repeated { name_count, .. } | BindDesc::Peek { name_count, .. } => {
                name_count + 1
            }
        }
    }

    fn to_bind<'a>(self, procs: &[AnnProc<'a>], mask: &BitSlice) -> Bind<'a> {
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
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ReceiptDesc {
    parts: BindDescripts,
    len: usize,
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

            next.to_bind(this_procs, this_mask)
        })
    }

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
    type Item = BindIter<'slice, 'a, SliceIter<'slice, BindDesc>>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|next| {
            let (this_procs, rest_procs) = self.procs.split_at(next.len);
            let (this_mask, rest_mask) = self.mask.split_at(next.len);

            self.procs = rest_procs;
            self.mask = rest_mask;

            BindIter {
                iter: next.parts.iter(),
                procs: this_procs,
                mask: this_mask,
            }
        })
    }

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
}

type MaskIterSpec<'a> = MaskIter<'a, usize, bitvec::order::Lsb0>;
struct LetBindingIter<'slice, 'a> {
    iter: Zip<
        Zip<SliceIter<'slice, AnnProc<'a>>, MaskIterSpec<'slice>>,
        SliceIter<'slice, AnnProc<'a>>,
    >,
    tail: Option<(Var<'a>, &'slice [AnnProc<'a>])>,
}

impl<'slice, 'a> LetBindingIter<'slice, 'a> {
    fn new(decl: &LetDecl, slice: &'slice [AnnProc<'a>], mask: &'slice BitSlice) -> Self {
        assert!(!slice.is_empty() && slice.len() == decl.lhs_arity + decl.rhs_arity);
        assert_eq!(slice.len(), mask.len());
        unsafe {
            // SAFETY: We check above that the slice contains exactly |lhs_arity + rhs_arity|
            // elements, and it is not zero. Therefore, lhs_arity <= slice.len()
            let (lhs, rhs) = slice.split_at_unchecked(decl.lhs_arity);
            if decl.lhs_has_cont && rhs.len() > lhs.len() {
                // SAFETY: If lhs has a continuation then it's arity is at least 1
                let (rem, init) = lhs.split_last().unwrap_unchecked();
                LetBindingIter {
                    iter: init.iter().zip(mask.iter()).zip(rhs.iter()),
                    tail: Some((into_remainder(*rem), &rhs[(lhs.len() - 1)..])),
                }
            } else {
                LetBindingIter {
                    iter: lhs.iter().zip(mask.iter()).zip(rhs.iter()),
                    tail: None,
                }
            }
        }
    }
}

impl<'a> Iterator for LetBindingIter<'_, 'a> {
    type Item = LetBinding<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|((l, q), r)| LetBinding::Single {
                lhs: into_name(*l, *q),
                rhs: *r,
            })
            .or_else(|| {
                self.tail.map(|(lhs, rhs)| LetBinding::Multiple {
                    lhs,
                    rhs: rhs.to_vec(),
                })
            })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.tail.is_none() {
            self.iter.size_hint()
        } else {
            let (min, max) = self.iter.size_hint();
            (min + 1, max.map(|hint| hint + 1))
        }
    }
}

struct LetDeclIter<'slice, 'a, O>
where
    O: Iterator<Item = &'slice LetDecl> + ExactSizeIterator,
{
    outer: O,
    procs: &'slice [AnnProc<'a>],
    mask: &'slice BitSlice,
    current_inner: Option<LetBindingIter<'slice, 'a>>,
}

impl<'slice, 'a, O> LetDeclIter<'slice, 'a, O>
where
    O: Iterator<Item = &'slice LetDecl> + ExactSizeIterator,
{
    fn new(
        decls: impl IntoIterator<Item = O::Item, IntoIter = O>,
        procs: &'slice [AnnProc<'a>],
        mask: &'slice BitSlice,
    ) -> Self {
        LetDeclIter {
            outer: decls.into_iter(),
            procs,
            mask,
            current_inner: None,
        }
    }
}

impl<'slice, 'a, O> Iterator for LetDeclIter<'slice, 'a, O>
where
    O: Iterator<Item = &'slice LetDecl> + ExactSizeIterator,
{
    type Item = LetBinding<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(inner) = &mut self.current_inner {
            if let Some(item) = inner.next() {
                return Some(item);
            }
        }
        // inner exhausted → refill from outer
        loop {
            match self.outer.next() {
                Some(let_decl) => {
                    let split_point = let_decl.lhs_arity + let_decl.rhs_arity;
                    let (this_procs, rest_procs) = self.procs.split_at(split_point);
                    let (this_mask, rest_mask) = self.mask.split_at(split_point);

                    let mut new_inner = LetBindingIter::new(let_decl, this_procs, this_mask);

                    self.procs = rest_procs;
                    self.mask = rest_mask;

                    if let Some(item) = new_inner.next() {
                        self.current_inner = Some(new_inner);
                        return Some(item);
                    }
                }
                None => return None,
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.outer.len(), None)
    }
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

pub struct NamesIter<'slice, 'a> {
    procs: &'slice [AnnProc<'a>],
    mask: &'slice BitSlice,
    front: usize,
    back: usize,
}

impl<'slice, 'a> NamesIter<'slice, 'a> {
    #[inline]
    pub fn new(procs: &'slice [AnnProc<'a>], mask: &'slice BitSlice) -> Self {
        assert_eq!(procs.len(), mask.len());
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
