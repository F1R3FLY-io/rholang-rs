use std::{
    fmt::{Display, Write},
    ops::Deref,
};

use smallvec::SmallVec;

use crate::{SourcePos, SourceSpan};

pub type ProcList<'a> = SmallVec<[AnnProc<'a>; 1]>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Proc<'ast> {
    Nil,
    BoolLiteral(bool),
    LongLiteral(i64),
    StringLiteral(&'ast str),
    UriLiteral(Uri<'ast>),

    SimpleType(SimpleType),
    Collection(Collection<'ast>),

    ProcVar(Var<'ast>),

    Par {
        left: AnnProc<'ast>,
        right: AnnProc<'ast>,
    },

    IfThenElse {
        condition: AnnProc<'ast>,
        if_true: AnnProc<'ast>,
        if_false: Option<AnnProc<'ast>>,
    },

    Send {
        channel: AnnName<'ast>,
        send_type: SendType,
        inputs: ProcList<'ast>,
    },

    ForComprehension {
        receipts: Receipts<'ast>,
        proc: AnnProc<'ast>,
    },

    Match {
        expression: AnnProc<'ast>,
        cases: Vec<Case<'ast>>,
    },

    Select {
        branches: Vec<Branch<'ast>>,
    },

    Bundle {
        bundle_type: BundleType,
        proc: AnnProc<'ast>,
    },

    Let {
        bindings: SmallVec<[LetBinding<'ast>; 1]>,
        body: AnnProc<'ast>,
        concurrent: bool,
    },

    New {
        decls: Vec<NameDecl<'ast>>,
        proc: AnnProc<'ast>,
    },

    Contract {
        name: AnnName<'ast>,
        formals: Names<'ast>,
        body: AnnProc<'ast>,
    },

    SendSync {
        channel: AnnName<'ast>,
        messages: ProcList<'ast>,
        cont: SyncSendCont<'ast>,
    },

    // expressions
    Eval {
        name: AnnName<'ast>,
    },
    Quote {
        proc: AnnProc<'ast>,
    },
    Method {
        receiver: AnnProc<'ast>,
        name: Id<'ast>,
        args: ProcList<'ast>,
    },

    UnaryExp {
        op: UnaryExpOp,
        arg: &'ast Proc<'ast>,
    },
    BinaryExp {
        op: BinaryExpOp,
        left: AnnProc<'ast>,
        right: AnnProc<'ast>,
    },

    // VarRef
    VarRef {
        kind: VarRefKind,
        var: Id<'ast>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AnnProc<'ast> {
    pub proc: &'ast Proc<'ast>,
    pub span: SourceSpan,
}

// process variables and names

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Id<'ast> {
    pub name: &'ast str,
    pub pos: SourcePos,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Var<'ast> {
    Id(Id<'ast>),
    Wildcard,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Name<'ast> {
    ProcVar(Var<'ast>),
    Quote(&'ast Proc<'ast>),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AnnName<'ast> {
    pub name: Name<'ast>,
    pub span: SourceSpan,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Names<'ast> {
    pub names: SmallVec<[AnnName<'ast>; 1]>,
    pub remainder: Option<Var<'ast>>,
}

impl Clone for Names<'_> {
    fn clone(&self) -> Self {
        let mut dest_names = SmallVec::with_capacity(self.names.len());
        dest_names.extend_from_slice(&self.names);

        Names {
            names: dest_names,
            remainder: self.remainder,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        // Inspired by `impl Clone for Vec`.
        let source_len = source.names.len();
        // drop anything that will not be overwritten
        self.names.truncate(source_len);
        let len = self.names.len();

        // len <= source_len due to the truncate above, so the
        // slices here are always in-bounds.
        let (init, tail) = source.names.split_at(len);

        // reuse the contained values' allocations/resources.

        self.names.copy_from_slice(init);
        self.names.extend_from_slice(tail);
        self.remainder.clone_from(&source.remainder);
    }
}

// expressions

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnaryExpOp {
    Not,
    Neg,
    Negation,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BinaryExpOp {
    Or,
    And,
    Matches,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    Concat,
    Diff,
    Add,
    Sub,
    Interpolation,
    Mult,
    Div,
    Mod,
    Disjunction,
    Conjunction,
}

// for-comprehensions

pub type Receipts<'a> = SmallVec<[Receipt<'a>; 1]>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Receipt<'a> {
    pub binds: SmallVec<[Bind<'a>; 1]>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Bind<'ast> {
    Linear {
        lhs: Names<'ast>,
        rhs: Source<'ast>,
    },
    Repeated {
        lhs: Names<'ast>,
        rhs: AnnName<'ast>,
    },
    Peek {
        lhs: Names<'ast>,
        rhs: AnnName<'ast>,
    },
}

// source definitions

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Source<'ast> {
    Simple {
        name: AnnName<'ast>,
    },
    ReceiveSend {
        name: AnnName<'ast>,
    },
    SendReceive {
        name: AnnName<'ast>,
        inputs: ProcList<'ast>,
    },
}

// case in match expression

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Case<'ast> {
    pub pattern: AnnProc<'ast>,
    pub proc: AnnProc<'ast>,
}

// branch in select expression

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SelectPattern<'ast> {
    pub lhs: Names<'ast>,
    pub rhs: Source<'ast>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Branch<'ast> {
    pub patterns: Vec<SelectPattern<'ast>>,
    pub proc: AnnProc<'ast>,
}

// ground terms and expressions

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Uri<'a>(&'a str);

impl Deref for Uri<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> From<&'a str> for Uri<'a> {
    fn from(value: &'a str) -> Self {
        Uri(value.trim_matches(|c| c == '`'))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SimpleType {
    Bool,
    Int,
    String,
    Uri,
    ByteArray,
}

// collections

pub type KeyValuePair<'ast> = (AnnProc<'ast>, AnnProc<'ast>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Collection<'ast> {
    List {
        elements: Vec<AnnProc<'ast>>,
        remainder: Option<Var<'ast>>,
    },

    Tuple(Vec<AnnProc<'ast>>),

    Set {
        elements: Vec<AnnProc<'ast>>,
        remainder: Option<Var<'ast>>,
    },

    Map {
        elements: Vec<KeyValuePair<'ast>>,
        remainder: Option<Var<'ast>>,
    },
}

// sends

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SendType {
    Single,
    Multiple,
}

// bundles

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum BundleType {
    BundleEquiv,
    BundleWrite,
    BundleRead,
    BundleReadWrite,
}

// let declarations

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LetBinding<'ast> {
    Single {
        lhs: AnnName<'ast>,
        rhs: AnnProc<'ast>,
    },
    Multiple {
        lhs: Var<'ast>,
        rhs: Vec<AnnProc<'ast>>,
    },
}

// new name declaration

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct NameDecl<'ast> {
    pub id: Id<'ast>,
    pub uri: Option<Uri<'ast>>,
}

// synchronous send continuations

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SyncSendCont<'ast> {
    Empty,
    NonEmpty(AnnProc<'ast>),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum VarRefKind {
    Proc,
    Name,
}

// display implementations

impl Display for Var<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Var::Id(id) => Display::fmt(id, f),
            Var::Wildcard => f.write_char('_'),
        }
    }
}

impl Display for Id<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('\'')?;
        f.write_str(self.name)?;
        f.write_char('\'')?;
        Ok(())
    }
}

impl Display for Uri<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('`')?;
        f.write_str(self.0)?;
        f.write_char('`')?;
        Ok(())
    }
}

impl Display for SimpleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Display for NameDecl<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.id, f)?;
        if let Some(uri) = &self.uri {
            f.write_char('(')?;
            Display::fmt(uri, f)?;
            f.write_char(')')?;
        }

        Ok(())
    }
}

// useful constants

pub const NIL: Proc = Proc::Nil;
pub const GTRUE: Proc = Proc::BoolLiteral(true);
pub const GFALSE: Proc = Proc::BoolLiteral(false);
pub const WILD: Proc = Proc::ProcVar(Var::Wildcard);
pub const NAME_WILD: Name = Name::ProcVar(Var::Wildcard);
pub const TYPE_URI: Proc = Proc::SimpleType(SimpleType::Uri);
pub const TYPE_STRING: Proc = Proc::SimpleType(SimpleType::String);
pub const TYPE_INT: Proc = Proc::SimpleType(SimpleType::Int);
pub const TYPE_BOOL: Proc = Proc::SimpleType(SimpleType::Bool);
pub const TYPE_BYTEA: Proc = Proc::SimpleType(SimpleType::ByteArray);
