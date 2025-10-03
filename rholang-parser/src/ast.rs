use std::{
    fmt::{Display, Write},
    ops::Deref,
};

use smallvec::{SmallVec, smallvec};

use crate::{SourcePos, SourceSpan, traverse::PreorderDfsIter};

pub type ProcList<'a> = SmallVec<[AnnProc<'a>; 1]>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Proc<'ast> {
    Nil,
    Unit,
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
        channel: Name<'ast>,
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
        bindings: LetBindings<'ast>,
        body: AnnProc<'ast>,
        concurrent: bool,
    },

    New {
        decls: Vec<NameDecl<'ast>>,
        proc: AnnProc<'ast>,
    },

    Contract {
        name: Name<'ast>,
        formals: Names<'ast>,
        body: AnnProc<'ast>,
    },

    SendSync {
        channel: Name<'ast>,
        inputs: ProcList<'ast>,
        cont: SyncSendCont<'ast>,
    },

    // expressions
    Eval {
        name: Name<'ast>,
    },
    Method {
        receiver: AnnProc<'ast>,
        name: Id<'ast>,
        args: ProcList<'ast>,
    },

    UnaryExp {
        op: UnaryExpOp,
        arg: AnnProc<'ast>,
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

    Bad, // bad process usually represents a parsing error
}

impl<'a> Proc<'a> {
    pub fn ann(&'a self, span: SourceSpan) -> AnnProc<'a> {
        AnnProc { proc: self, span }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AnnProc<'ast> {
    pub proc: &'ast Proc<'ast>,
    pub span: SourceSpan,
}

impl<'a> AnnProc<'a> {
    pub fn iter_preorder_dfs(&'a self) -> impl Iterator<Item = &'a Self> {
        PreorderDfsIter::<16>::new(self)
    }
}

// process variables and names

#[derive(Debug, Clone, Copy)]
pub struct Id<'ast> {
    pub name: &'ast str,
    pub pos: SourcePos,
}

impl PartialEq for Id<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Id<'_> {}

impl Ord for Id<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(other.name)
    }
}

impl PartialOrd for Id<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Var<'ast> {
    Wildcard,
    Id(Id<'ast>),
}

impl<'a> TryFrom<&Proc<'a>> for Var<'a> {
    type Error = String;

    fn try_from(value: &Proc<'a>) -> Result<Self, Self::Error> {
        match value {
            Proc::ProcVar(var) => Ok(*var),
            other => Err(format!("attempt to convert {{ {other:?} }} to a var")),
        }
    }
}

impl<'a> TryFrom<AnnProc<'a>> for Var<'a> {
    type Error = String;

    fn try_from(value: AnnProc<'a>) -> Result<Self, Self::Error> {
        value.proc.try_into()
    }
}

impl<'a> TryFrom<Name<'a>> for Var<'a> {
    type Error = String;

    fn try_from(value: Name<'a>) -> Result<Self, Self::Error> {
        match value {
            Name::NameVar(var) => Ok(var),
            other => Err(format!("attempt to convert {{ {other:?} }} to a var")),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Name<'ast> {
    NameVar(Var<'ast>),
    Quote(AnnProc<'ast>),
}

impl<'a> From<Id<'a>> for Name<'a> {
    fn from(value: Id<'a>) -> Self {
        Name::NameVar(Var::Id(value))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Names<'ast> {
    pub names: SmallVec<[Name<'ast>; 1]>,
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

impl<'a> Names<'a> {
    pub(super) fn from_iter<I>(iterable: I, with_remainder: bool) -> Result<Names<'a>, String>
    where
        I: IntoIterator<Item = Name<'a>, IntoIter: DoubleEndedIterator>,
    {
        let mut iter = iterable.into_iter();
        let remainder = if with_remainder {
            match iter.next_back() {
                None => return Err("attempt to build 'x, y ...@z' out of zero names".to_string()),
                Some(last) => Some(last.try_into()?),
            }
        } else {
            None
        };

        Ok(Names {
            names: iter.collect(),
            remainder,
        })
    }

    #[allow(dead_code)]
    pub(super) fn single(name: Name<'a>) -> Self {
        Names {
            names: smallvec![name],
            remainder: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.remainder.is_none()
    }

    pub fn only_remainder(&self) -> bool {
        self.names.is_empty() && self.remainder.is_some()
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
pub type Receipt<'a> = SmallVec<[Bind<'a>; 1]>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Bind<'ast> {
    Linear { lhs: Names<'ast>, rhs: Source<'ast> },
    Repeated { lhs: Names<'ast>, rhs: Name<'ast> },
    Peek { lhs: Names<'ast>, rhs: Name<'ast> },
}

// source definitions

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Source<'ast> {
    Simple {
        name: Name<'ast>,
    },
    ReceiveSend {
        name: Name<'ast>,
    },
    SendReceive {
        name: Name<'ast>,
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

    PathMap {
        elements: Vec<AnnProc<'ast>>,
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

pub type LetBindings<'a> = SmallVec<[LetBinding<'a>; 1]>;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LetBinding<'ast> {
    Single {
        lhs: Name<'ast>,
        rhs: AnnProc<'ast>,
    },
    Multiple {
        lhs: Var<'ast>,
        rhs: Vec<AnnProc<'ast>>,
    },
}

// new name declaration

#[derive(Debug, Clone, Copy)]
pub struct NameDecl<'ast> {
    pub id: Id<'ast>,
    pub uri: Option<Uri<'ast>>,
}

impl PartialEq for NameDecl<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for NameDecl<'_> {}

impl Ord for NameDecl<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for NameDecl<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
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
