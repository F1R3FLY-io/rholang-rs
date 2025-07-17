use std::{
    fmt::{Display, Write},
    ops::Deref,
};

use smallvec::{SmallVec, ToSmallVec};

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
        proc: &'ast Proc<'ast>,
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

    Bad, // bad process usually represents a parsing error
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AnnProc<'ast> {
    pub proc: &'ast Proc<'ast>,
    pub span: SourceSpan,
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Name<'ast> {
    ProcVar(Var<'ast>),
    Quote(&'ast Proc<'ast>),
}

impl<'a> TryFrom<&Proc<'a>> for Name<'a> {
    type Error = String;

    fn try_from(value: &Proc<'a>) -> Result<Self, Self::Error> {
        match value {
            Proc::ProcVar(var) => Ok(Name::ProcVar(*var)),
            Proc::Quote { proc } => Ok(Name::Quote(*proc)),
            other => Err(format!("{other:?} is not a name")),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AnnName<'ast> {
    pub name: Name<'ast>,
    pub span: SourceSpan,
}

impl<'a> TryFrom<AnnProc<'a>> for AnnName<'a> {
    type Error = String;

    fn try_from(value: AnnProc<'a>) -> Result<Self, Self::Error> {
        value.proc.try_into().map(|name| AnnName {
            name,
            span: value.span,
        })
    }
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

impl<'a> Names<'a> {
    pub(super) fn from_slice(
        slice: &[AnnProc<'a>],
        with_remainder: bool,
    ) -> Result<Names<'a>, String> {
        fn to_names<'b>(procs: &[AnnProc<'b>]) -> Result<SmallVec<[AnnName<'b>; 1]>, String> {
            procs.iter().map(|p| (*p).try_into()).collect()
        }
        //this method is optimized for small input (<= 2 names) because it collects directly into SmallVec's inline buffer
        //Consider allocating to an intermediate Vec if output is deemed to be large
        if with_remainder {
            match slice.split_last() {
                None => Err("attempt to build 'x, y ...@z' out of zero names".to_string()),
                Some((_, init)) if init.is_empty() => {
                    Err("attempt to build 'x, y ...@z' out of one name".to_string())
                }
                Some((last, init)) => {
                    let names = to_names(init)?;
                    let remainder = (*last).try_into()?;
                    Ok(Names {
                        names,
                        remainder: Some(remainder),
                    })
                }
            }
        } else {
            if slice.is_empty() {
                Err("attempt to build empty names".to_string())
            } else {
                let names = to_names(slice)?;
                Ok(Names {
                    names,
                    remainder: None,
                })
            }
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
