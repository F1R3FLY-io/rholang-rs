use std::{
    fmt::{Debug, Display, Write},
    ops::Deref,
};

use smallvec::{SmallVec, smallvec};

use crate::{SourcePos, SourceSpan, traverse::*};

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

    pub fn is_trivially_ground(&self) -> bool {
        match self {
            Proc::Nil
            | Proc::Unit
            | Proc::BoolLiteral(_)
            | Proc::LongLiteral(_)
            | Proc::StringLiteral(_)
            | Proc::UriLiteral(_)
            | Proc::SimpleType(_)
            | Proc::ProcVar(Var::Wildcard)
            | Proc::Bad => true,
            Proc::Collection(col) if col.is_empty() => true,
            _ => false,
        }
    }

    pub fn is_ident(&self, expected: &str) -> bool {
        match self {
            Proc::ProcVar(var) => var.is_ident(expected),
            _ => false,
        }
    }

    pub fn as_var(&self) -> Option<Var<'a>> {
        match self {
            Proc::ProcVar(var) => Some(*var),
            _ => None,
        }
    }
}

impl<'a> From<Var<'a>> for Proc<'a> {
    fn from(value: Var<'a>) -> Self {
        Proc::ProcVar(value)
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

    pub fn iter_dfs_event(&'a self) -> impl Iterator<Item = DfsEvent<'a>> {
        DfsEventIter::<32>::new(self)
    }

    pub fn iter_dfs_event_and_names(&'a self) -> impl Iterator<Item = DfsEventExt<'a>> {
        NameAwareDfsEventIter::<32>::new(self)
    }

    pub fn is_trivially_ground(&self) -> bool {
        self.proc.is_trivially_ground()
    }

    pub fn is_ident(&self, expected: &str) -> bool {
        self.proc.is_ident(expected)
    }

    pub fn as_var(&self) -> Option<Var<'a>> {
        self.proc.as_var()
    }

    pub fn iter_proc_vars(&'a self) -> impl Iterator<Item = Var<'a>> {
        PreorderDfsIter::<4>::new(self).filter_map(|ann_proc| ann_proc.as_var())
    }

    pub fn iter_vars(&'a self) -> impl Iterator<Item = Var<'a>> {
        NameAwareDfsEventIter::<4>::new(self).filter_map(|ev| match ev {
            DfsEventExt::Enter(ann_proc) => ann_proc.as_var(),
            DfsEventExt::Name(name) => name.as_var(),
            DfsEventExt::Exit(_) => None,
        })
    }

    pub fn iter_names_direct(&'a self) -> impl Iterator<Item = &'a Name<'a>> {
        NameAwareDfsEventIter::<4>::new(self)
            .skip(1) // skip Enter(self)
            .take_while(|ev| ev.as_proc().is_none()) // stop before entering any sub-process
            .filter_map(|ev| ev.as_name())
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

impl<'a> Var<'a> {
    pub fn get_position(self) -> Option<SourcePos> {
        match self {
            Var::Wildcard => None,
            Var::Id(id) => Some(id.pos),
        }
    }

    pub fn is_ident(self, expected: &str) -> bool {
        match self {
            Var::Wildcard => expected == "_",
            Var::Id(id) => id.name == expected,
        }
    }

    pub fn as_ident(self) -> &'a str {
        match self {
            Var::Wildcard => "_",
            Var::Id(id) => id.name,
        }
    }
}

impl<'a> TryFrom<&Proc<'a>> for Var<'a> {
    type Error = String;

    fn try_from(value: &Proc<'a>) -> Result<Self, Self::Error> {
        value
            .as_var()
            .ok_or_else(|| format!("attempt to convert {{ {value:?} }} to a var"))
    }
}

impl<'a> TryFrom<AnnProc<'a>> for Var<'a> {
    type Error = String;

    fn try_from(value: AnnProc<'a>) -> Result<Self, Self::Error> {
        value.proc.try_into()
    }
}

impl<'a> TryFrom<&AnnProc<'a>> for Var<'a> {
    type Error = String;

    fn try_from(value: &AnnProc<'a>) -> Result<Self, Self::Error> {
        value.proc.try_into()
    }
}

impl<'a> TryFrom<Name<'a>> for Var<'a> {
    type Error = String;

    fn try_from(value: Name<'a>) -> Result<Self, Self::Error> {
        value
            .as_var()
            .ok_or_else(|| format!("attempt to convert {{ {value:?} }} to a var"))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Name<'ast> {
    NameVar(Var<'ast>),
    Quote(AnnProc<'ast>),
}

impl<'a> Name<'a> {
    pub fn is_ident(&self, expected: &str) -> bool {
        match self {
            Name::NameVar(var) => var.is_ident(expected),
            Name::Quote(ann_proc) => ann_proc.is_ident(expected),
        }
    }

    pub fn is_trivially_ground(&self) -> bool {
        match self {
            Name::NameVar(Var::Wildcard) => true,
            Name::Quote(quoted) => quoted.is_trivially_ground(),
            _ => false,
        }
    }

    pub fn as_quote(&'a self) -> Option<&'a AnnProc<'a>> {
        match self {
            Name::Quote(quoted) => Some(quoted),
            _ => None,
        }
    }

    pub fn as_var(&self) -> Option<Var<'a>> {
        match self {
            Name::NameVar(var) => Some(*var),
            Name::Quote(_) => None,
        }
    }

    pub fn iter_into(&'a self) -> impl Iterator<Item = DfsEventExt<'a>> {
        match self {
            Name::NameVar(_) => NameAwareDfsEventIter::<4>::single(self),
            Name::Quote(quoted) => NameAwareDfsEventIter::<4>::new(quoted),
        }
    }
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

pub enum NamesKind<'a> {
    Empty,
    SingleName(&'a Name<'a>),
    SingleRemainder(Var<'a>),
    Multiple,
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
    pub fn from_iter<I>(iterable: I, with_remainder: bool) -> Result<Names<'a>, String>
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

    pub fn single(name: Name<'a>) -> Self {
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

    pub fn is_single_name(&self) -> bool {
        self.names.len() == 1 && self.remainder.is_none()
    }

    pub fn kind(&'a self) -> NamesKind<'a> {
        if self.is_empty() {
            NamesKind::Empty
        } else if self.only_remainder() {
            NamesKind::SingleRemainder(self.remainder.unwrap())
        } else if self.is_single_name() {
            NamesKind::SingleName(&self.names[0])
        } else {
            NamesKind::Multiple
        }
    }
}

// expressions

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnaryExpOp {
    Not,
    Neg,
    Negation,
}
impl UnaryExpOp {
    pub fn is_connective(self) -> bool {
        self == UnaryExpOp::Negation
    }
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

impl BinaryExpOp {
    pub fn is_connective(self) -> bool {
        matches!(self, BinaryExpOp::Conjunction | BinaryExpOp::Disjunction)
    }
}

// for-comprehensions

pub type Receipts<'a> = SmallVec<[Receipt<'a>; 1]>;
pub type Receipt<'a> = SmallVec<[Bind<'a>; 1]>;

pub fn source_names<'a>(
    receipt: &'a [Bind<'a>],
) -> impl DoubleEndedIterator<Item = &'a Name<'a>> + ExactSizeIterator {
    receipt.iter().map(|bind| bind.source_name())
}

pub fn inputs<'a>(receipt: &'a [Bind<'a>]) -> impl DoubleEndedIterator<Item = &'a AnnProc<'a>> {
    receipt.iter().filter_map(|bind| bind.input()).flatten()
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Bind<'ast> {
    Linear { lhs: Names<'ast>, rhs: Source<'ast> },
    Repeated { lhs: Names<'ast>, rhs: Name<'ast> },
    Peek { lhs: Names<'ast>, rhs: Name<'ast> },
}

impl<'a> Bind<'a> {
    pub fn source_name(&self) -> &Name<'a> {
        match self {
            Bind::Linear { lhs: _, rhs } => match rhs {
                Source::Simple { name }
                | Source::ReceiveSend { name }
                | Source::SendReceive { name, .. } => name,
            },
            Bind::Repeated { lhs: _, rhs } | Bind::Peek { lhs: _, rhs } => rhs,
        }
    }

    pub fn input(&self) -> Option<&[AnnProc<'a>]> {
        match self {
            Bind::Linear { lhs: _, rhs } => match rhs {
                Source::Simple { .. } | Source::ReceiveSend { .. } => None,
                Source::SendReceive { name: _, inputs } => Some(inputs),
            },
            Bind::Repeated { .. } | Bind::Peek { .. } => None,
        }
    }

    pub fn names(&self) -> &Names<'a> {
        match self {
            Bind::Linear { lhs, rhs: _ }
            | Bind::Repeated { lhs, rhs: _ }
            | Bind::Peek { lhs, rhs: _ } => lhs,
        }
    }

    pub fn names_iter(&self) -> std::slice::Iter<'_, Name<'a>> {
        self.names().names.iter()
    }
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
        Uri(super::trim_byte(value, b'`'))
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

impl<'a> Collection<'a> {
    pub fn remainder(&self) -> Option<Var<'a>> {
        match self {
            Collection::List { remainder, .. }
            | Collection::Set { remainder, .. }
            | Collection::Map { remainder, .. } => *remainder,
            Collection::Tuple(_) => None,
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Collection::List {
                elements,
                remainder,
            }
            | Collection::Set {
                elements,
                remainder,
            } => elements.is_empty() && remainder.is_none(),
            Collection::Map {
                elements,
                remainder,
            } => elements.is_empty() && remainder.is_none(),
            Collection::Tuple(_) => false,
        }
    }
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
pub struct LetBinding<'ast> {
    pub lhs: Names<'ast>,
    pub rhs: ProcList<'ast>,
}

impl<'a> LetBinding<'a> {
    pub fn single(lhs: Name<'a>, rhs: AnnProc<'a>) -> Self {
        Self {
            lhs: Names::single(lhs),
            rhs: smallvec![rhs],
        }
    }
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
        f.write_char(':')?;
        Display::fmt(&self.pos, f)
    }
}

impl Display for Uri<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('`')?;
        f.write_str(self.0)?;
        f.write_char('`')
    }
}

impl Display for SimpleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Display for NameDecl<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('\'')?;
        f.write_str(self.id.name)?;
        f.write_char('\'')?;
        if let Some(uri) = &self.uri {
            f.write_char('(')?;
            Display::fmt(uri, f)?;
            f.write_char(')')?;
        }

        f.write_char(':')?;
        Display::fmt(&self.id.pos, f)
    }
}
