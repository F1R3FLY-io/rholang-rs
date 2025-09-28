use std::{collections::BTreeMap, fmt::Display, iter::FusedIterator, u32};

use bitvec::prelude::*;
use by_address::ByAddress;
use fixedbitset::FixedBitSet as BitSet;
use indexmap::IndexMap;
use intmap::{IntKey, IntMap};
use rholang_parser::{SourcePos, ast};

pub mod db;
mod interner;
mod resolver;

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

impl From<Binder> for SymbolOccurence {
    fn from(value: Binder) -> Self {
        Self {
            symbol: value.name,
            position: value.source_position,
        }
    }
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
    free: BitVec,           // Track binders that are free (only in patterns)
    captures: BitSet,       // Tracks which *outer binders* are captured from enclosing scopes.
}

impl ScopeInfo {
    fn valid_range(num: usize) -> u32 {
        num.try_into()
            .expect("didn't expect more than 4 billions of binders")
    }

    pub fn new(binder_start: BinderId, num_binders: usize) -> Self {
        Self::from_parts(
            binder_start,
            bitvec![0; num_binders],
            BitSet::with_capacity(binder_start.0 as usize),
        )
    }

    pub fn ground(binder_start: BinderId) -> Self {
        Self::from_parts(binder_start, BitVec::EMPTY, BitSet::new())
    }

    pub fn free_var(binder_start: BinderId) -> Self {
        Self::from_parts(binder_start, bitvec![1], BitSet::new())
    }

    pub fn var_ref(binder_start: BinderId, ref_binder: BinderId) -> ScopeInfo {
        let captures = BitSet::with_capacity(binder_start.0 as usize);
        let mut res = Self::from_parts(binder_start, BitVec::EMPTY, captures);
        res.mark_captured(ref_binder);

        res
    }

    pub fn from_parts(binder_start: BinderId, free: BitVec, captures: BitSet) -> Self {
        let num_binders = free.len();
        Self::valid_range(binder_start.0 as usize + num_binders);
        Self {
            binder_start,
            num_binders: num_binders as u32,
            uses: bitvec![0; num_binders],
            free,
            captures,
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
        self.captures.is_clear() && self.free.not_any()
    }

    pub fn num_captures(&self) -> usize {
        self.captures.count_ones(..)
    }

    #[inline]
    pub fn mark_used(&mut self, bid: BinderId) {
        unsafe {
            self.mark_used_unchecked(self.checked_idx_inside(bid));
        }
    }

    #[inline(always)]
    pub unsafe fn mark_used_unchecked(&mut self, idx: usize) {
        unsafe {
            self.uses.set_unchecked(idx, true);
        }
    }

    pub fn set_uses(&mut self, uses: BitVec) {
        assert_eq!(self.num_binders(), uses.len());
        self.uses = uses;
    }

    #[inline]
    pub fn mark_captured(&mut self, bid: BinderId) {
        assert!(bid < self.binder_start, "binder {bid} too far!");
        if self.captures.len() <= bid.0 as usize {
            self.captures.grow(self.binder_start.0 as usize);
        }
        unsafe {
            // SAFETY: only accepts ids < binder_start. The line above preallocates if empty
            self.captures.set_unchecked(bid.0 as usize, true);
        }
    }

    pub fn captures(&self) -> impl DoubleEndedIterator<Item = BinderId> + ExactSizeIterator {
        // SAFETY: fixed bit size will never exceed self.binder_start
        let iter = self.captures.ones().map(|i| BinderId(i as u32));
        WithLen::new(iter, self.num_captures())
    }

    pub fn num_free(&self) -> usize {
        self.free.count_ones()
    }

    pub fn free(&self) -> impl Iterator<Item = BinderId> + ExactSizeIterator {
        self.free
            .iter_ones()
            .map(|i| self.binder_start + (i as u32))
    }
}

/// Describes occurence of a symbol
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VarBinding {
    /// resolved variable
    Bound(BinderId),
    /// unresolved variable in a pattern; `index` points to its parent binders
    Free { index: usize },
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

    var_to_binder: BTreeMap<SymbolOccurence, VarBinding>, // var -> where it is bound
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
    ShadowedVar { original: SymbolOccurence },
    UnusedVariable,
    PatternIsExpression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    UnboundVariable,
    DuplicateVarDef { original: SymbolOccurence },
    NameInProcPosition(BinderId, Symbol),
    ProcInNamePosition(BinderId, Symbol),
    ConnectiveOutsidePattern,
    BundleInsidePattern,
    BadCode,
}

impl ErrorKind {
    pub fn kind_mismatch(binder: BinderId, sym: Symbol, expects_name: bool) -> Self {
        if expects_name {
            ErrorKind::ProcInNamePosition(binder, sym)
        } else {
            ErrorKind::NameInProcPosition(binder, sym)
        }
    }
}

const SEED0: u64 = 0x0FED_CBA9_8765_4321;
const SEED1: u64 = 0x0BAD_F00D_F00D_BAAD;
const SEED2: u64 = 0xCAFEBABE_DEADC0DE;
const SEED3: u64 = 0x1234_5678_9ABC_DEF0;

fn stable_hasher() -> ahash::RandomState {
    ahash::RandomState::with_seeds(SEED0, SEED1, SEED2, SEED3)
}

struct WithLen<I> {
    iter: I,
    len: usize,
}

impl<I> WithLen<I> {
    pub fn new(iter: I, len: usize) -> Self {
        Self { iter, len }
    }
}

impl<I> Iterator for WithLen<I>
where
    I: Iterator,
{
    type Item = I::Item;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next();
        if item.is_some() {
            self.len -= 1;
        }
        item
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<I> DoubleEndedIterator for WithLen<I>
where
    I: DoubleEndedIterator,
{
    #[inline(always)]
    fn next_back(&mut self) -> Option<Self::Item> {
        let item = self.iter.next_back();
        if item.is_some() {
            self.len -= 1;
        }
        item
    }
}

impl<I> ExactSizeIterator for WithLen<I> where I: Iterator {}

impl<I> FusedIterator for WithLen<I> where I: FusedIterator {}
