//! Utility functions for scope building and variable tracking
//!
//! This module provides reusable helper functions to reduce code duplication
//! in the scope building process (Phase 3.1).

use crate::sem::{BinderKind, PID, Symbol, SymbolOccurence};
use rholang_parser::SourcePos;
use std::collections::HashSet;

use super::errors::ElaborationError;
use super::patterns::PatternVariable;

/// Check if a symbol is duplicate and return an error if so
///
/// This helper consolidates the duplicate checking logic used throughout
/// scope building to ensure consistent error reporting.
///
/// # Arguments
///
/// * `symbol` - The symbol to check
/// * `position` - Source position of this occurrence
/// * `seen_symbols` - Set of previously seen symbols
/// * `all_variables` - List of all pattern variables collected so far
/// * `for_comp_pid` - The PID of the for-comprehension (for error reporting)
///
/// # Returns
///
/// Returns `Ok(())` if no duplicate, or `Err(ElaborationError::DuplicateVarDef)` if duplicate found
pub fn check_duplicate_symbol(
    symbol: Symbol,
    position: SourcePos,
    seen_symbols: &HashSet<Symbol>,
    all_variables: &[PatternVariable],
    for_comp_pid: PID,
) -> Result<(), ElaborationError> {
    if seen_symbols.contains(&symbol) {
        // Find the original occurrence
        let original = all_variables
            .iter()
            .find(|v| v.symbol == symbol)
            .expect("Duplicate must have original");

        return Err(ElaborationError::DuplicateVarDef {
            pid: for_comp_pid,
            original: SymbolOccurence {
                symbol: original.symbol,
                position: original.position,
            },
            duplicate: SymbolOccurence { symbol, position },
        });
    }

    Ok(())
}

/// Add a pattern variable after checking for duplicates
///
/// This combines duplicate checking and variable insertion into a single operation.
///
/// # Arguments
///
/// * `symbol` - The symbol to add
/// * `position` - Source position of this occurrence
/// * `kind` - The kind of binder (Name or Proc)
/// * `seen_symbols` - Mutable set of previously seen symbols
/// * `all_variables` - Mutable list to append the variable to
/// * `for_comp_pid` - The PID of the for-comprehension (for error reporting)
///
/// # Returns
///
/// Returns `Ok(())` if successfully added, or `Err(ElaborationError::DuplicateVarDef)` if duplicate
pub fn add_pattern_variable(
    symbol: Symbol,
    position: SourcePos,
    kind: BinderKind,
    seen_symbols: &mut HashSet<Symbol>,
    all_variables: &mut Vec<PatternVariable>,
    for_comp_pid: PID,
) -> Result<(), ElaborationError> {
    check_duplicate_symbol(symbol, position, seen_symbols, all_variables, for_comp_pid)?;

    seen_symbols.insert(symbol);
    all_variables.push(PatternVariable::new(symbol, position, kind));

    Ok(())
}

/// Add a remainder pattern variable after checking for duplicates
///
/// Similar to `add_pattern_variable` but specifically for remainder variables.
///
/// # Arguments
///
/// * `symbol` - The symbol to add
/// * `position` - Source position of this occurrence
/// * `kind` - The kind of binder (usually Proc for remainder)
/// * `seen_symbols` - Mutable set of previously seen symbols
/// * `all_variables` - Mutable list to append the variable to
/// * `for_comp_pid` - The PID of the for-comprehension (for error reporting)
///
/// # Returns
///
/// Returns `Ok(())` if successfully added, or `Err(ElaborationError::DuplicateVarDef)` if duplicate
pub fn add_remainder_variable(
    symbol: Symbol,
    position: SourcePos,
    kind: BinderKind,
    seen_symbols: &mut HashSet<Symbol>,
    all_variables: &mut Vec<PatternVariable>,
    for_comp_pid: PID,
) -> Result<(), ElaborationError> {
    check_duplicate_symbol(symbol, position, seen_symbols, all_variables, for_comp_pid)?;

    seen_symbols.insert(symbol);
    all_variables.push(PatternVariable::new(symbol, position, kind).with_remainder(true));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::{BinderKind, PID, Symbol};
    use rholang_parser::SourcePos;

    #[test]
    fn test_check_duplicate_symbol_no_duplicate() {
        let symbol = Symbol(1);
        let seen = HashSet::new();
        let vars = vec![];
        let pid = PID(0);

        let result = check_duplicate_symbol(symbol, SourcePos::default(), &seen, &vars, pid);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_duplicate_symbol_with_duplicate() {
        let symbol = Symbol(1);
        let mut seen = HashSet::new();
        seen.insert(symbol);

        let vars = vec![PatternVariable::new(
            symbol,
            SourcePos::default(),
            BinderKind::Proc,
        )];
        let pid = PID(0);

        let result = check_duplicate_symbol(symbol, SourcePos::at_col(5), &seen, &vars, pid);
        assert!(result.is_err());

        if let Err(ElaborationError::DuplicateVarDef {
            original,
            duplicate,
            ..
        }) = result
        {
            assert_eq!(original.symbol, symbol);
            assert_eq!(duplicate.symbol, symbol);
            assert_ne!(original.position, duplicate.position);
        } else {
            panic!("Expected DuplicateVarDef error");
        }
    }

    #[test]
    fn test_add_pattern_variable() {
        let symbol = Symbol(1);
        let mut seen = HashSet::new();
        let mut vars = vec![];
        let pid = PID(0);

        let result = add_pattern_variable(
            symbol,
            SourcePos::default(),
            BinderKind::Proc,
            &mut seen,
            &mut vars,
            pid,
        );

        assert!(result.is_ok());
        assert!(seen.contains(&symbol));
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].symbol, symbol);
        assert!(!vars[0].is_remainder);
    }

    #[test]
    fn test_add_remainder_variable() {
        let symbol = Symbol(1);
        let mut seen = HashSet::new();
        let mut vars = vec![];
        let pid = PID(0);

        let result = add_remainder_variable(
            symbol,
            SourcePos::default(),
            BinderKind::Proc,
            &mut seen,
            &mut vars,
            pid,
        );

        assert!(result.is_ok());
        assert!(seen.contains(&symbol));
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].symbol, symbol);
        assert!(vars[0].is_remainder);
    }

    #[test]
    fn test_add_pattern_variable_duplicate_error() {
        let symbol = Symbol(1);
        let mut seen = HashSet::new();
        let mut vars = vec![];
        let pid = PID(0);

        // First addition should succeed
        let result1 = add_pattern_variable(
            symbol,
            SourcePos::default(),
            BinderKind::Proc,
            &mut seen,
            &mut vars,
            pid,
        );
        assert!(result1.is_ok());

        // Second addition should fail
        let result2 = add_pattern_variable(
            symbol,
            SourcePos::at_col(5),
            BinderKind::Proc,
            &mut seen,
            &mut vars,
            pid,
        );
        assert!(result2.is_err());
    }
}
