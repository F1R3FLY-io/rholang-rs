use smallvec::ToSmallVec;
use typed_arena::Arena;

use crate::ast::{
    AnnName, AnnProc, BinaryExpOp, Bind, BundleType, Case, Collection, Id, KeyValuePair,
    LetBinding, NameDecl, Names, Proc, SendType, SimpleType, SyncSendCont, UnaryExpOp, Var,
    VarRefKind,
};

pub struct ASTBuilder<'ast> {
    arena: Arena<Proc<'ast>>,
    // useful quasi-constants
    nil: Proc<'ast>,
    r#true: Proc<'ast>,
    r#false: Proc<'ast>,
    wild: Proc<'ast>,
    unit: Proc<'ast>,
    bad: Proc<'ast>,
    empty_list: Proc<'ast>,
    empty_map: Proc<'ast>,
    zero: Proc<'ast>,
    one: Proc<'ast>,
}

impl<'ast> ASTBuilder<'ast> {
    pub(super) fn new() -> Self {
        Self::with_capacity(64)
    }

    pub(super) fn with_capacity(capacity: usize) -> Self {
        ASTBuilder {
            arena: Arena::with_capacity(capacity),
            nil: Proc::Nil,
            r#true: Proc::BoolLiteral(true),
            r#false: Proc::BoolLiteral(false),
            wild: Proc::ProcVar(Var::Wildcard),
            unit: Proc::Unit,
            empty_list: Proc::Collection(Collection::List {
                elements: Vec::new(),
                remainder: None,
            }),
            empty_map: Proc::Collection(Collection::Map {
                elements: Vec::new(),
                remainder: None,
            }),
            zero: Proc::LongLiteral(0),
            one: Proc::LongLiteral(1),
            bad: Proc::Bad,
        }
    }

    pub fn const_nil(&self) -> &Proc<'ast> {
        &self.nil
    }

    pub(super) fn const_true(&self) -> &Proc<'ast> {
        &self.r#true
    }

    pub(super) fn const_false(&self) -> &Proc<'ast> {
        &self.r#false
    }

    pub fn const_wild(&self) -> &Proc<'ast> {
        &self.wild
    }

    pub(super) fn const_unit(&self) -> &Proc<'ast> {
        &self.unit
    }

    pub(super) fn const_empty_list(&self) -> &Proc<'ast> {
        &self.empty_list
    }

    pub(super) fn const_empty_map(&self) -> &Proc<'ast> {
        &self.empty_map
    }

    pub(super) fn bad_const(&self) -> &Proc<'ast> {
        &self.bad
    }

    pub fn alloc_string_literal(&self, value: &'ast str) -> &Proc<'ast> {
        self.arena
            .alloc(Proc::StringLiteral(value.trim_matches(|c| c == '"')))
    }

    pub fn alloc_long_literal(&self, value: i64) -> &Proc<'ast> {
        match value {
            0 => &self.zero,
            1 => &self.one,
            other => self.arena.alloc(Proc::LongLiteral(other)),
        }
    }

    pub(super) fn alloc_uri_literal(&self, value: &'ast str) -> &Proc<'ast> {
        self.arena.alloc(Proc::UriLiteral(value.into()))
    }

    pub(super) fn alloc_simple_type(&self, value: SimpleType) -> &Proc<'ast> {
        self.arena.alloc(Proc::SimpleType(value))
    }

    pub fn alloc_list(&self, procs: &[AnnProc<'ast>]) -> &Proc<'ast> {
        if procs.is_empty() {
            return self.const_empty_list();
        }
        self.arena.alloc(Proc::Collection(Collection::List {
            elements: procs.to_vec(),
            remainder: None,
        }))
    }

    pub fn alloc_list_with_remainder(
        &self,
        procs: &[AnnProc<'ast>],
        remainder: Var<'ast>,
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::Collection(Collection::List {
            elements: procs.to_vec(),
            remainder: Some(remainder),
        }))
    }

    pub fn alloc_set(&self, procs: &[AnnProc<'ast>]) -> &Proc<'ast> {
        self.arena.alloc(Proc::Collection(Collection::Set {
            elements: procs.to_vec(),
            remainder: None,
        }))
    }

    pub fn alloc_set_with_remainder(
        &self,
        procs: &[AnnProc<'ast>],
        remainder: Var<'ast>,
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::Collection(Collection::Set {
            elements: procs.to_vec(),
            remainder: Some(remainder),
        }))
    }

    pub(super) fn alloc_tuple(&self, procs: &[AnnProc<'ast>]) -> &Proc<'ast> {
        self.arena
            .alloc(Proc::Collection(Collection::Tuple(procs.to_vec())))
    }

    fn to_key_value(slice: &[AnnProc<'ast>]) -> Vec<KeyValuePair<'ast>> {
        slice.chunks_exact(2).map(|kv| (kv[0], kv[1])).collect()
    }

    pub fn alloc_map(&self, pairs: &[AnnProc<'ast>]) -> &Proc<'ast> {
        if pairs.is_empty() {
            return self.const_empty_map();
        }
        self.arena.alloc(Proc::Collection(Collection::Map {
            elements: Self::to_key_value(pairs),
            remainder: None,
        }))
    }

    pub fn alloc_map_with_remainder(
        &self,
        pairs: &[AnnProc<'ast>],
        remainder: Var<'ast>,
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::Collection(Collection::Map {
            elements: Self::to_key_value(pairs),
            remainder: Some(remainder),
        }))
    }

    pub fn alloc_var(&self, id: Id<'ast>) -> &Proc<'ast> {
        self.arena.alloc(Proc::ProcVar(Var::Id(id)))
    }

    pub fn alloc_par(&self, left: AnnProc<'ast>, right: AnnProc<'ast>) -> &Proc<'ast> {
        self.arena.alloc(Proc::Par { left, right })
    }

    pub(super) fn alloc_if_then(
        &self,
        condition: AnnProc<'ast>,
        if_true: AnnProc<'ast>,
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::IfThenElse {
            condition,
            if_true,
            if_false: None,
        })
    }

    pub(super) fn alloc_if_then_else(
        &self,
        condition: AnnProc<'ast>,
        if_true: AnnProc<'ast>,
        if_false: AnnProc<'ast>,
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::IfThenElse {
            condition,
            if_true,
            if_false: Some(if_false),
        })
    }

    pub fn alloc_send(
        &self,
        send_type: SendType,
        channel: AnnName<'ast>,
        inputs: &[AnnProc<'ast>],
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::Send {
            channel,
            send_type,
            inputs: inputs.to_smallvec(),
        })
    }

    pub fn alloc_for<Rs, Bs>(&self, receipts: Rs, proc: AnnProc<'ast>) -> &Proc<'ast>
    where
        Rs: IntoIterator<Item = Bs>,
        Bs: IntoIterator<Item = Bind<'ast>>,
    {
        self.arena.alloc(Proc::ForComprehension {
            receipts: receipts
                .into_iter()
                .map(|bs| bs.into_iter().collect())
                .collect(),
            proc,
        })
    }

    pub fn alloc_match(
        &self,
        expression: AnnProc<'ast>,
        cases: &[AnnProc<'ast>],
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::Match {
            expression,
            cases: cases
                .chunks_exact(2)
                .map(|pair| Case {
                    pattern: pair[0],
                    proc: pair[1],
                })
                .collect(),
        })
    }

    pub(super) fn alloc_bundle(&self, bundle_type: BundleType, proc: AnnProc<'ast>) -> &Proc<'ast> {
        self.arena.alloc(Proc::Bundle { bundle_type, proc })
    }

    pub fn alloc_let<Ls>(
        &self,
        bindings: Ls,
        body: AnnProc<'ast>,
        concurrent: bool,
    ) -> &Proc<'ast>
    where
        Ls: IntoIterator<Item = LetBinding<'ast>>,
    {
        self.arena.alloc(Proc::Let {
            bindings: bindings.into_iter().collect(),
            body,
            concurrent,
        })
    }

    pub fn alloc_new(&self, proc: AnnProc<'ast>, decls: Vec<NameDecl<'ast>>) -> &Proc<'ast> {
        self.arena.alloc(Proc::New { decls, proc })
    }

    pub fn alloc_contract(
        &self,
        name: AnnName<'ast>,
        formals: Names<'ast>,
        body: AnnProc<'ast>,
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::Contract {
            name,
            formals,
            body,
        })
    }

    pub(super) fn alloc_send_sync(
        &self,
        channel: AnnName<'ast>,
        messages: &[AnnProc<'ast>],
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::SendSync {
            channel,
            messages: messages.to_smallvec(),
            cont: SyncSendCont::Empty,
        })
    }

    pub(super) fn alloc_send_sync_with_cont(
        &self,
        channel: AnnName<'ast>,
        messages: &[AnnProc<'ast>],
        cont: AnnProc<'ast>,
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::SendSync {
            channel,
            messages: messages.to_smallvec(),
            cont: SyncSendCont::NonEmpty(cont),
        })
    }

    pub fn alloc_eval(&self, name: AnnName<'ast>) -> &Proc<'ast> {
        self.arena.alloc(Proc::Eval { name })
    }

    pub(super) fn alloc_quote(&self, proc: &'ast Proc<'ast>) -> &Proc<'ast> {
        self.arena.alloc(Proc::Quote { proc })
    }

    pub(super) fn alloc_method(
        &self,
        name: Id<'ast>,
        receiver: AnnProc<'ast>,
        args: &[AnnProc<'ast>],
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::Method {
            receiver,
            name,
            args: args.to_smallvec(),
        })
    }

    pub fn alloc_binary_exp(
        &self,
        op: BinaryExpOp,
        left: AnnProc<'ast>,
        right: AnnProc<'ast>,
    ) -> &Proc<'ast> {
        self.arena.alloc(Proc::BinaryExp { op, left, right })
    }

    pub(super) fn alloc_unary_exp(&self, op: UnaryExpOp, arg: &'ast Proc<'ast>) -> &Proc<'ast> {
        self.arena.alloc(Proc::UnaryExp { op, arg })
    }

    pub(super) fn alloc_var_ref(&self, kind: VarRefKind, var: Id<'ast>) -> &Proc<'ast> {
        self.arena.alloc(Proc::VarRef { kind, var })
    }
}
