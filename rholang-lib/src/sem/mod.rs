use std::{collections::BTreeMap, fmt::Display};

use bitvec::prelude::*;
use by_address::ByAddress;
use indexmap::IndexMap;
use intmap::{IntKey, IntMap};
use rholang_parser::{SourcePos, ast};

pub mod db;
mod interner;

pub type ProcRef<'a> = &'a ast::AnnProc<'a>;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PID(u32);

impl Display for PID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl IntKey for PID {
    type Int = u32;

    const PRIME: Self::Int = 222_367;

    fn into_int(self) -> Self::Int {
        self.0
    }
}

/// Interned strings
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Symbol(u32);

/// Symbol occurence in the source code (used to mark variables)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SymbolOccurence {
    pub symbol: Symbol,
    pub position: SourcePos,
}

/// ID of a binder (variable or name)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinderId(u32);

impl BinderId {
    #[inline(always)]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(BinderId)
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        BinderId(self.0.saturating_sub(rhs.0))
    }
}

impl std::ops::Add for BinderId {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Add<u32> for BinderId {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl std::ops::Sub for BinderId {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Display for BinderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Binder {
    pub name: Symbol,
    pub kind: BinderKind,
    pub scope: PID,
    pub index: usize,
    pub source_position: SourcePos,
}

/// Distinguishes between name-valued and proc-valued binders
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BinderKind {
    Name(Option<Symbol>),
    Proc,
}

/// Metadata about a scope introduced by a new / let / match arm.
/// Compact: tracks just offsets and bitvecs.
#[derive(Clone, Debug)]
pub struct ScopeInfo {
    binder_start: BinderId, // The first binder introduced in this scope.
    num_binders: u32,       // Number of binders introduced by this scope.
    uses: BitVec,           // Tracks which binders of *this scope* are actually used.
    captures: BitVec,       // Tracks which *outer binders* are captured from enclosing scopes.
}

impl ScopeInfo {
    pub fn new(binder_start: BinderId, num_binders: usize, total_binders: usize) -> Self {
        Self {
            binder_start,
            num_binders: num_binders
                .try_into()
                .expect("didn't expect more than 4 billions of binders"),
            uses: bitvec![0; num_binders],
            captures: bitvec![0; total_binders],
        }
    }

    pub fn num_binders(&self) -> usize {
        self.num_binders as usize
    }

    #[inline(always)]
    fn binder_end(&self) -> BinderId {
        self.binder_start + self.num_binders
    }

    #[inline(always)]
    fn checked_idx_inside(&self, bid: BinderId) -> usize {
        bid.checked_sub(self.binder_start)
            .expect("binder outside scope")
            .0 as usize
    }

    #[inline(always)]
    fn checked_idx_outside(&self, bid: BinderId) -> usize {
        let offset_from_end = self
            .binder_start
            .checked_sub(bid)
            .expect("binder too far")
            .0 as usize;
        self.captures.len() - offset_from_end
    }

    #[inline(always)]
    pub fn contains(&self, bid: BinderId) -> bool {
        self.binder_start <= bid && bid < self.binder_end()
    }

    #[inline(always)]
    pub fn binder_range(&self) -> impl DoubleEndedIterator<Item = BinderId> + ExactSizeIterator {
        let start = self.binder_start.0;
        (start..(start + self.num_binders)).map(BinderId)
    }

    #[inline(always)]
    pub fn as_range(&self) -> std::ops::Range<usize> {
        let start = self.binder_start.0 as usize;
        let end = start + self.num_binders as usize;
        start..end
    }

    pub fn is_top_level(&self) -> bool {
        self.captures.is_empty()
    }

    pub fn is_ground(&self) -> bool {
        self.captures.not_any()
    }

    pub fn is_constant(&self) -> bool {
        self.uses.not_any()
    }

    pub fn num_captures(&self) -> usize {
        self.captures.count_ones()
    }

    #[inline]
    pub fn mark_used(&mut self, bid: BinderId) {
        let idx = self.checked_idx_inside(bid);
        self.uses.set(idx, true);
    }

    #[inline]
    pub fn mark_captured(&mut self, bid: BinderId) {
        let idx = self.checked_idx_outside(bid);
        self.captures.set(idx, true);
    }
}

pub struct SemanticDb<'a> {
    rev: IndexMap<ByAddress<ProcRef<'a>>, PID, ahash::RandomState>, // ref <-> PID
    interner: interner::Interner,                                   // name <-> Symbol

    diagnostics: Vec<Diagnostic>,
    has_errors: bool,

    next_binder: u32,
    binder_is_name: BitVec,                // fast BinderId -> name or proc
    binders: Vec<Binder>,                  // semantic info about each binding
    proc_to_scope: IntMap<PID, ScopeInfo>, // PID -> semantic info about the scope

    var_to_binder: BTreeMap<SymbolOccurence, BinderId>, // var -> where it is bound
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diagnostic {
    pub pid: PID,
    pub kind: DiagnosticKind,
    pub exact_position: Option<SourcePos>,
}

impl Diagnostic {
    pub fn error(pid: PID, kind: ErrorKind, pos: Option<SourcePos>) -> Self {
        Self {
            pid,
            kind: DiagnosticKind::Error(kind),
            exact_position: pos,
        }
    }

    pub fn warning(pid: PID, kind: WarningKind, pos: Option<SourcePos>) -> Self {
        Self {
            pid,
            kind: DiagnosticKind::Warning(kind),
            exact_position: pos,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    Info(InfoKind),
    Warning(WarningKind),
    Error(ErrorKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoKind {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningKind {
    ShadowedVar {
        symbol: Symbol,
        old_position: SourcePos,
    },
    UnusedVariable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    UnboundVariable,
    DuplicateVarDef {
        symbol: Symbol,
        old_position: SourcePos,
    },
    NameInProcPosition(BinderId),
    RemainderOutsidePattern,
    ConnectiveOutsidePattern,
    WildcardOutsidePattern,
    BadCode,
}

const SEED0: u64 = 0x0FED_CBA9_8765_4321;
const SEED1: u64 = 0x0BAD_F00D_F00D_BAAD;
const SEED2: u64 = 0xCAFEBABE_DEADC0DE;
const SEED3: u64 = 0x1234_5678_9ABC_DEF0;

fn stable_hasher() -> ahash::RandomState {
    ahash::RandomState::with_seeds(SEED0, SEED1, SEED2, SEED3)
}
