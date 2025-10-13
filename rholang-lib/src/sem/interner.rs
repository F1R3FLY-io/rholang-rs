use std::ops::Index;

use indexmap::IndexSet;

use super::Symbol;
use parking_lot::RwLock;

pub(super) struct Interner {
    rev: RwLock<IndexSet<String, ahash::RandomState>>,
}

const DEFAULT_INTERNER_CAPACITY: usize = 32;

impl Interner {
    pub(super) fn new() -> Self {
        Self {
            rev: RwLock::new(IndexSet::with_capacity_and_hasher(
                DEFAULT_INTERNER_CAPACITY,
                super::stable_hasher(),
            )),
        }
    }

    pub(super) fn intern(&self, name: &str) -> Symbol {
        // First try immutable borrow to check for existing symbol
        if let Some(sym) = self.rev.read().get_index_of(name) {
            Symbol(sym as u32) // SAFETY: we never allow length of the symbol table to exceed |u32|
        } else {
            // Only borrow mutably if we need to insert
            let mut rev = self.rev.write();
            let new_sym = rev
                .len()
                .try_into()
                .expect("too many symbols in the symbol table");
            rev.insert(name.to_string());
            Symbol(new_sym)
        }
    }

    pub(super) fn resolve(&self, sym: Symbol) -> Option<&str> {
        let rev = self.rev.read();
        rev.get_index(sym.0 as usize).map(|s| unsafe {
            // SAFETY: safe because `intern` only holds a mutable borrow when inserting,
            // and here we only call `resolve` after `intern` has returned.
            // There is never an active mutable borrow while using this reference.
            str::from_utf8_unchecked(std::slice::from_raw_parts(s.as_ptr(), s.len()))
        })
    }

    pub(super) fn resolve_owned(&self, sym: Symbol) -> Option<String> {
        // No need to use `String.as_ptr()` trickery
        self.rev.read().get_index(sym.0 as usize).cloned()
    }
}

impl Index<Symbol> for Interner {
    type Output = str;

    fn index(&self, sym: Symbol) -> &Self::Output {
        self.resolve(sym).expect("symbol not interned")
    }
}
