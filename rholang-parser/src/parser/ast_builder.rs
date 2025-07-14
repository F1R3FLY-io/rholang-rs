use smallvec::ToSmallVec;
use typed_arena::Arena;

use crate::ast::{
    AnnName, AnnProc, BinaryExpOp, Bind, BundleType, Case, Collection, Id, KeyValuePair,
    LetBinding, NameDecl, Names, Proc, Receipt, SendType, SimpleType, SyncSendCont, UnaryExpOp,
    Var, VarRefKind,
};

pub(super) struct ASTBuilder<'ast> {
    arena: Arena<Proc<'ast>>,
    // useful quasi-constants
    pub(super) NIL: Proc<'ast>,
    pub(super) TRUE: Proc<'ast>,
    pub(super) FALSE: Proc<'ast>,
    pub(super) WILD: Proc<'ast>,
    pub(super) BAD: Proc<'ast>,
}

impl<'ast> ASTBuilder<'ast> {
    pub(super) fn new() -> ASTBuilder<'ast> {
        Self::with_capacity(64)
    }

    pub(super) fn with_capacity(capacity: usize) -> ASTBuilder<'ast> {
        ASTBuilder {
            arena: Arena::with_capacity(capacity),
            NIL: Proc::Nil,
            TRUE: Proc::BoolLiteral(true),
            FALSE: Proc::BoolLiteral(false),
            WILD: Proc::ProcVar(Var::Wildcard),
            BAD: Proc::Bad,
        }
    }

    pub(super) fn alloc_string_literal(&'ast self, value: &'ast str) -> &'ast Proc<'ast> {
        self.arena
            .alloc(Proc::StringLiteral(value.trim_matches(|c| c == '"')))
    }

    pub(super) fn alloc_long_literal(&'ast self, value: i64) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::LongLiteral(value))
    }

    pub(super) fn alloc_uri_literal(&'ast self, value: &'ast str) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::UriLiteral(value.into()))
    }

    pub(super) fn alloc_simple_type(&'ast self, value: SimpleType) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::SimpleType(value))
    }

    pub(super) fn alloc_list(&'ast self, procs: &[AnnProc<'ast>]) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Collection(Collection::List {
            elements: procs.to_vec(),
            remainder: None,
        }))
    }

    pub(super) fn alloc_list_with_remainder(
        &'ast self,
        procs: &[AnnProc<'ast>],
        remainder: Var<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Collection(Collection::List {
            elements: procs.to_vec(),
            remainder: Some(remainder),
        }))
    }

    pub(super) fn alloc_set(&'ast self, procs: &[AnnProc<'ast>]) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Collection(Collection::Set {
            elements: procs.to_vec(),
            remainder: None,
        }))
    }

    pub(super) fn alloc_set_with_remainder(
        &'ast self,
        procs: &[AnnProc<'ast>],
        remainder: Var<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Collection(Collection::Set {
            elements: procs.to_vec(),
            remainder: Some(remainder),
        }))
    }

    pub(super) fn alloc_tuple(&'ast self, procs: &[AnnProc<'ast>]) -> &'ast Proc<'ast> {
        self.arena
            .alloc(Proc::Collection(Collection::Tuple(procs.to_vec())))
    }

    fn to_key_value(slice: &[AnnProc<'ast>]) -> Vec<KeyValuePair<'ast>> {
        slice.chunks_exact(2).map(|kv| (kv[0], kv[1])).collect()
    }

    pub(super) fn alloc_map(&'ast self, pairs: &[AnnProc<'ast>]) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Collection(Collection::Map {
            elements: Self::to_key_value(pairs),
            remainder: None,
        }))
    }

    pub(super) fn alloc_map_with_remainder(
        &'ast self,
        pairs: &[AnnProc<'ast>],
        remainder: Var<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Collection(Collection::Map {
            elements: Self::to_key_value(pairs),
            remainder: Some(remainder),
        }))
    }

    pub(super) fn alloc_var(&'ast self, id: Id<'ast>) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::ProcVar(Var::Id(id)))
    }

    pub(super) fn alloc_par(
        &'ast self,
        left: AnnProc<'ast>,
        right: AnnProc<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Par { left, right })
    }

    pub(super) fn alloc_if_then(
        &'ast self,
        condition: AnnProc<'ast>,
        if_true: AnnProc<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::IfThenElse {
            condition,
            if_true,
            if_false: None,
        })
    }

    pub(super) fn alloc_if_then_else(
        &'ast self,
        condition: AnnProc<'ast>,
        if_true: AnnProc<'ast>,
        if_false: AnnProc<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::IfThenElse {
            condition,
            if_true,
            if_false: Some(if_false),
        })
    }

    pub(super) fn alloc_send(
        &'ast self,
        send_type: SendType,
        channel: AnnName<'ast>,
        inputs: &[AnnProc<'ast>],
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Send {
            channel,
            send_type,
            inputs: inputs.to_smallvec(),
        })
    }

    pub(super) fn alloc_for<'x, Rs>(&'ast self, rs: Rs, proc: AnnProc<'ast>) -> &'ast Proc<'ast>
    where
        Rs: IntoIterator<Item = &'x [Bind<'ast>]>,
        'ast: 'x,
    {
        let receipts = rs
            .into_iter()
            .map(|bs| Receipt { binds: bs.into() })
            .collect();
        self.arena.alloc(Proc::ForComprehension { receipts, proc })
    }

    pub(super) fn alloc_match(
        &'ast self,
        expression: AnnProc<'ast>,
        cases: &[AnnProc<'ast>],
    ) -> &'ast Proc<'ast> {
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

    pub(super) fn alloc_bundle(
        &'ast self,
        bundle_type: BundleType,
        proc: AnnProc<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Bundle { bundle_type, proc })
    }

    pub(super) fn alloc_let<Ls>(
        &'ast self,
        bindings: Ls,
        body: AnnProc<'ast>,
        concurrent: bool,
    ) -> &'ast Proc<'ast>
    where
        Ls: IntoIterator<Item = LetBinding<'ast>>,
    {
        self.arena.alloc(Proc::Let {
            bindings: bindings.into_iter().collect(),
            body,
            concurrent,
        })
    }

    pub(super) fn alloc_new(
        &'ast self,
        proc: AnnProc<'ast>,
        decls: Vec<NameDecl<'ast>>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::New { decls, proc })
    }

    pub(super) fn alloc_contract(
        &'ast self,
        name: AnnName<'ast>,
        formals: Names<'ast>,
        body: AnnProc<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Contract {
            name,
            formals,
            body,
        })
    }

    pub(super) fn alloc_send_sync(
        &'ast self,
        channel: AnnName<'ast>,
        messages: &[AnnProc<'ast>],
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::SendSync {
            channel,
            messages: messages.to_smallvec(),
            cont: SyncSendCont::Empty,
        })
    }

    pub(super) fn alloc_send_sync_with_cont(
        &'ast self,
        channel: AnnName<'ast>,
        messages: &[AnnProc<'ast>],
        cont: AnnProc<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::SendSync {
            channel,
            messages: messages.to_smallvec(),
            cont: SyncSendCont::NonEmpty(cont),
        })
    }

    pub(super) fn alloc_eval(&'ast self, name: AnnName<'ast>) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Eval { name })
    }

    pub(super) fn alloc_quote(&'ast self, proc: &'ast Proc<'ast>) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Quote { proc })
    }

    pub(super) fn alloc_method(
        &'ast self,
        name: Id<'ast>,
        receiver: AnnProc<'ast>,
        args: &[AnnProc<'ast>],
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::Method {
            receiver,
            name,
            args: args.to_smallvec(),
        })
    }

    pub(super) fn alloc_binary_exp(
        &'ast self,
        op: BinaryExpOp,
        left: AnnProc<'ast>,
        right: AnnProc<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::BinaryExp { op, left, right })
    }

    pub(super) fn alloc_unary_exp(
        &'ast self,
        op: UnaryExpOp,
        arg: &'ast Proc<'ast>,
    ) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::UnaryExp { op, arg })
    }

    pub(super) fn alloc_var_ref(&'ast self, kind: VarRefKind, var: Id<'ast>) -> &'ast Proc<'ast> {
        self.arena.alloc(Proc::VarRef { kind, var })
    }
}
