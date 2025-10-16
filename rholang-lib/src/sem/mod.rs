use std::{borrow::Cow, collections::BTreeMap, fmt::Display, iter::FusedIterator};

use as_any::AsAny;
use bitvec::prelude::*;
use by_address::ByAddress;
use fixedbitset::FixedBitSet as BitSet;
use indexmap::IndexMap;
use intmap::{IntKey, IntMap};
use rholang_parser::{SourcePos, SourceSpan, ast};

pub mod db;
pub mod diagnostics;
mod elaborator;
mod interner;
pub mod pipeline;
mod resolver;

/// A generic semantic analysis pass.
///
/// This is the root trait for all analysis passes — both fact-producing and diagnostic ones.
/// It exists primarily to provide introspection and polymorphic storage within the pipeline.
pub trait Pass: AsAny {
    /// A human-readable name for debugging/logging.
    fn name(&self) -> Cow<'static, str>;
}

/// A *fact pass* that computes or mutates semantic information.
///
/// These passes populate the [`SemanticDb`] with inferred or resolved facts
/// (such as symbol bindings, types, captures, etc.).
/// They are **executed sequentially** to guarantee deterministic mutation order.
pub trait FactPass: Pass {
    /// Executes the pass, mutating the [`SemanticDb`].
    fn run(&self, db: &mut SemanticDb);
}

/// A *diagnostic pass* that inspects the results of fact passes.
///
/// Diagnostic passes **must not mutate** the [`SemanticDb`]. They analyze
/// the collected facts and emit warnings, infos, or errors.
///
/// Diagnostic passes may run **in parallel** since they only read from the database.
pub trait DiagnosticPass: Pass + Send + Sync {
    /// Executes the pass asynchronously.  
    /// Only read access to `db` is permitted.
    fn run(&self, db: &SemanticDb) -> Vec<Diagnostic>;
}

pub use elaborator::ForCompElaborationPass;
pub use resolver::ResolverPass;

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
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(u32);

impl Symbol {
    pub const DUMMY: Symbol = Symbol(u32::MAX);
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Symbol occurence in the source code (used to mark variables)
#[derive(Copy, Clone, Debug)]
pub struct SymbolOccurence {
    pub position: SourcePos,
    pub symbol: Symbol,
}

impl PartialEq for SymbolOccurence {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
    }
}

impl Eq for SymbolOccurence {}

impl PartialOrd for SymbolOccurence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SymbolOccurence {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.position.cmp(&other.position)
    }
}

impl From<Binder> for SymbolOccurence {
    fn from(value: Binder) -> Self {
        Self {
            symbol: value.name,
            position: value.source_position,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BoundOccurence {
    pub occurence: SymbolOccurence,
    pub binding: VarBinding,
}

/// ID of a binder (variable or name)
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BinderId(u32);

impl BinderId {
    pub const MAX: BinderId = BinderId(u32::MAX);

    #[inline(always)]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(BinderId)
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    span: SourceSpan,
}

impl ScopeInfo {
    pub fn new(binder_start: BinderId, num_binders: usize, span: SourceSpan) -> Self {
        Self::from_parts(
            binder_start,
            bitvec![0; num_binders],
            BitSet::with_capacity(binder_start.0 as usize),
            span,
        )
    }

    pub fn ground(binder_start: BinderId, span: SourceSpan) -> Self {
        Self::from_parts(binder_start, BitVec::EMPTY, BitSet::new(), span)
    }

    pub fn free_var(binder_start: BinderId, span: SourceSpan) -> Self {
        Self::from_parts(binder_start, bitvec![1], BitSet::new(), span)
    }

    pub fn var_ref(binder_start: BinderId, ref_binder: BinderId, span: SourceSpan) -> ScopeInfo {
        let captures = BitSet::with_capacity(binder_start.0 as usize);
        let mut res = Self::from_parts(binder_start, BitVec::EMPTY, captures, span);
        res.mark_captured(ref_binder);

        res
    }

    pub fn from_parts(
        binder_start: BinderId,
        free: BitVec,
        captures: BitSet,
        span: SourceSpan,
    ) -> Self {
        let num_binders = free.len();
        if num_binders > u32::MAX as usize
            || binder_start.0 as usize + num_binders > u32::MAX as usize
        {
            panic!("didn't expect more than 4 billions of binders")
        }

        Self {
            binder_start,
            num_binders: num_binders as u32,
            uses: bitvec![0; num_binders],
            free,
            captures,
            span,
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
    pub fn all_used(&self) -> bool {
        self.uses.all()
    }

    pub fn unused(&self) -> Unused<'_> {
        Unused::new(&self.uses, self.binder_start.0)
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

    pub fn captures(&self) -> Captures<'_> {
        Captures::new(&self.captures)
    }

    pub fn num_free(&self) -> usize {
        self.free.count_ones()
    }

    pub fn free(&self) -> Free<'_> {
        Free::new(&self.free, self.binder_start.0)
    }

    pub fn absorb(&mut self, rhs: ScopeInfo) {
        assert_eq!(
            self.binder_end(),
            rhs.binder_start,
            "scopes not contiguous: left {}..{}, right {}..{}",
            self.binder_start,
            self.binder_end(),
            rhs.binder_start,
            rhs.binder_end()
        );

        // Merge captures first
        self.captures.union_with(&rhs.captures);

        // Convert captures from rhs that fall into self's binder range into uses
        let binder_range = self.as_range();
        let overlap_start = binder_range.start;
        let overlaps = self
            .captures
            .maximum() // that’s a clever tradeoff since it compiles down to a SIMD scan of the last non-empty block.
            .is_some_and(|last| overlap_start <= last);
        if overlaps {
            let overlap_end = binder_range.end.min(self.captures.len());

            // REMARK: Since FixedBitSet stores its bits as a Vec<Block>, we can operate
            // block-by-block instead of bit-by-bit. This can reduce iteration overhead
            // significantly, especially if scopes become wide.
            //
            // Idea for rewrite if this ever becomes a performance problem
            // - Compute the block range that overlaps with binder_range.
            // - For each overlapping block:
            // - - Mask the part of the block that belongs to the overlap.
            // - - Convert those set bits into uses (shifted indices).
            // - - Clear them in captures.
            for i in self.captures.ones().rev() {
                // Otherwise if there are any set bits at or beyond binder_range.end, they’ll also be converted into uses.
                if i >= overlap_end {
                    continue;
                }
                if i < overlap_start {
                    break;
                }
                self.uses.set(i - overlap_start, true);
            }
            self.captures.set_range(overlap_start..overlap_end, false);
        }

        // Merge other metadata
        if self.num_binders == 0 {
            self.free = rhs.free;
            self.uses = rhs.uses;
            self.num_binders = rhs.num_binders;
        } else if rhs.num_binders() != 0 {
            self.free.extend_from_bitslice(&rhs.free);
            self.uses.extend_from_bitslice(&rhs.uses);
            self.num_binders += rhs.num_binders;
        }
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
    UnusedVariable(BinderId, Symbol),
    TopLevelPatternExpr { span: SourceSpan },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    UnboundVariable,
    DuplicateVarDef { original: SymbolOccurence },
    NameInProcPosition(BinderId, Symbol),
    ProcInNamePosition(BinderId, Symbol),
    ConnectiveOutsidePattern,
    BundleInsidePattern,
    FreeVariable(SymbolOccurence),
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

pub struct Free<'a> {
    inner: bitvec::slice::IterOnes<'a, usize, Lsb0>,
    binder_start: u32,
}

impl<'a> Free<'a> {
    pub fn new(free: &'a BitVec, binder_start: u32) -> Self {
        Self {
            inner: free.iter_ones(),
            binder_start,
        }
    }

    pub fn empty() -> Self {
        Self {
            inner: bitvec::slice::IterOnes::default(),
            binder_start: u32::MAX,
        }
    }
}

impl Default for Free<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a> Iterator for Free<'a> {
    type Item = BinderId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|i| BinderId(self.binder_start + i as u32))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Free<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|i| BinderId(self.binder_start + i as u32))
    }
}

impl<'a> ExactSizeIterator for Free<'a> {}
impl<'a> FusedIterator for Free<'a> {}

pub struct Captures<'a> {
    inner: fixedbitset::Ones<'a>,
    current_len: usize,
}

impl<'a> Captures<'a> {
    pub fn new(bitmap: &'a BitSet) -> Self {
        Self {
            inner: bitmap.ones(),
            current_len: bitmap.count_ones(..),
        }
    }
}

impl<'a> Iterator for Captures<'a> {
    type Item = BinderId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let id = self.inner.next().map(|i| BinderId(i as u32)); // SAFETY: fixed bit size is bounded
        if id.is_some() {
            self.current_len -= 1;
        }
        id
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.current_len, Some(self.current_len))
    }
}

impl<'a> DoubleEndedIterator for Captures<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let id = self.inner.next_back().map(|i| BinderId(i as u32)); // SAFETY: fixed bit size is bounded
        if id.is_some() {
            self.current_len -= 1;
        }
        id
    }
}

impl<'a> ExactSizeIterator for Captures<'a> {}
impl<'a> FusedIterator for Captures<'a> {}

pub struct Unused<'a> {
    inner: bitvec::slice::IterZeros<'a, usize, Lsb0>,
    binder_start: u32,
}

impl<'a> Unused<'a> {
    pub fn new(used: &'a BitVec, binder_start: u32) -> Self {
        Self {
            inner: used.iter_zeros(),
            binder_start,
        }
    }

    pub fn empty() -> Self {
        Self {
            inner: bitvec::slice::IterZeros::default(),
            binder_start: u32::MAX,
        }
    }
}

impl Default for Unused<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<'a> Iterator for Unused<'a> {
    type Item = BinderId;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|i| BinderId(self.binder_start + i as u32))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Unused<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|i| BinderId(self.binder_start + i as u32))
    }
}

impl<'a> ExactSizeIterator for Unused<'a> {}
impl<'a> FusedIterator for Unused<'a> {}

#[cfg(test)]
mod tests {
    #[macro_export]
    macro_rules! match_proc {
        ($value:expr, $pat:pat => $body:expr) => {{
            if let $pat = $value {
                $body
            } else {
                panic!("unexpected AST structure: {:#?}", $value);
            }
        }};
    }
}
