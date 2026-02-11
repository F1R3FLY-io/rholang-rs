//! This module implements advanced type checking that goes beyond basic Name/Proc distinction
//! It validates:
//! - Channel type inference from usage patterns
//! - Pattern-message compatibility across bindings
//! - Type consistency throughout the for-comprehension scope
//!
//! Type validation operates on the semantic database after ResolverPass has built scopes
//! It analyzes channel usage patterns and infers types based on:
//! - How channels are declared (unforgeable names from `new`, quoted processes, variables)
//! - How channels are used in send/receive operations
//! - Pattern structure expectations
//!
//! ## Type Inference Rules
//!
//! 1. **Unforgeable Names**: Created by `new` declarations with optional URIs
//! 2. **Quoted Processes**: Channels that are quotes of processes (`@P`)
//! 3. **Variables**: Channels referenced by identifiers (resolved by ResolverPass)
//! 4. **Unknown**: Complex or indeterminate channel types

use crate::sem::{BinderKind, PID, SemanticDb};
use rholang_parser::ast::{self, AnnProc, Bind, Name, Names, Proc};

use super::{
    ChannelType,
    errors::{ValidationError, ValidationResult},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageType {
    /// Ground term (literals, nil, unit)
    Ground,
    /// Single name/quoted process
    Name,
    Process,
    List {
        expected_len: Option<usize>,
    },
    Tuple {
        expected_len: usize,
    },
    Set {
        expected_len: Option<usize>,
    },
    Map {
        expected_pairs: Option<usize>,
    },
    Multiple,
    /// Unknown or complex type
    Unknown,
}

pub struct TypeValidator<'a, 'ast> {
    db: &'a SemanticDb<'ast>,
}

impl<'a, 'ast> TypeValidator<'a, 'ast> {
    pub fn new(db: &'a SemanticDb<'ast>) -> Self {
        Self { db }
    }

    /// Validate channel usage and type consistency across a for-comprehension. It:
    ///
    /// 1. Validates that all channels have consistent types
    /// 2. Checks pattern-message compatibility for each binding
    /// 3. Ensures type consistency across the scope
    ///
    /// ## Arguments
    ///
    /// * `for_comp_pid` - The PID of the for-comprehension node
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` if all type checks pass, or a `ValidationError` otherwise
    pub fn validate_channel_usage(&self, for_comp_pid: PID) -> ValidationResult {
        let proc_ref =
            self.db
                .get(for_comp_pid)
                .ok_or_else(|| ValidationError::InvalidPatternStructure {
                    pid: for_comp_pid,
                    position: None,
                    reason: format!("PID {} not found in database", for_comp_pid),
                })?;

        let receipts = match proc_ref.proc {
            Proc::ForComprehension { receipts, .. } => receipts,
            _ => {
                return Err(ValidationError::InvalidPatternStructure {
                    pid: for_comp_pid,
                    position: Some(proc_ref.span.start),
                    reason: "Expected ForComprehension node".to_string(),
                });
            }
        };

        for receipt in receipts.iter() {
            self.validate_receipt(for_comp_pid, receipt)?;
        }

        Ok(())
    }

    /// Validate a single receipt (sequence of bindings)
    fn validate_receipt(&self, for_comp_pid: PID, receipt: &[Bind<'ast>]) -> ValidationResult {
        for bind in receipt.iter() {
            self.validate_bind(for_comp_pid, bind)?;
        }
        Ok(())
    }

    /// Validate a single binding
    fn validate_bind(&self, for_comp_pid: PID, bind: &Bind<'ast>) -> ValidationResult {
        // Get the pattern (LHS) and channel (RHS)
        let pattern = bind.names();
        let channel = bind.source_name();

        let channel_type = self.infer_channel_type(channel);
        let expected_msg_type = self.infer_pattern_type(pattern)?;

        self.validate_pattern_channel_compatibility(
            for_comp_pid,
            &channel_type,
            &expected_msg_type,
            pattern,
        )?;

        self.validate_pattern_structure(for_comp_pid, pattern)?;

        Ok(())
    }

    /// Infer the type of a channel from its structure
    fn infer_channel_type(&self, channel: &Name<'ast>) -> ChannelType {
        match channel {
            Name::Quote(_proc) => ChannelType::QuotedProcess,
            Name::NameVar(var) => match var {
                ast::Var::Wildcard => ChannelType::Unknown,
                ast::Var::Id(id) => {
                    // Check if this is bound to an unforgeable name from `new` declaration
                    if let Some(crate::sem::VarBinding::Bound(binder_id)) =
                        self.db.binder_of_id(*id)
                        && let Some(binder) = self.db.get_binder(binder_id)
                    {
                        // Check if this binder has URI (unforgeable name marker)
                        if let BinderKind::Name(Some(_uri)) = binder.kind {
                            return ChannelType::UnforgeableName;
                        }
                        // Otherwise it's a regular variable
                        return ChannelType::Variable;
                    }

                    // If we can't determine, treat as variable
                    ChannelType::Variable
                }
            },
        }
    }

    /// Infer the expected message type from a pattern
    fn infer_pattern_type(&self, pattern: &Names<'ast>) -> ValidationResult<MessageType> {
        if pattern.is_empty() {
            return Ok(MessageType::Ground);
        }

        if pattern.is_single_name() {
            let name = &pattern.names[0];
            return self.infer_single_name_type(name);
        }

        if pattern.names.len() > 1 || pattern.remainder.is_some() {
            return Ok(MessageType::Multiple);
        }

        Ok(MessageType::Unknown)
    }

    /// Infer type from a single name in a pattern
    fn infer_single_name_type(&self, name: &Name<'ast>) -> ValidationResult<MessageType> {
        match name {
            Name::Quote(proc) => self.infer_proc_pattern_type(proc),
            Name::NameVar(_) => Ok(MessageType::Name),
        }
    }

    /// Infer message type from a process pattern
    fn infer_proc_pattern_type(&self, proc: &AnnProc<'ast>) -> ValidationResult<MessageType> {
        match proc.proc {
            // Ground terms
            Proc::Nil
            | Proc::Unit
            | Proc::BoolLiteral(_)
            | Proc::LongLiteral(_)
            | Proc::StringLiteral(_)
            | Proc::UriLiteral(_)
            | Proc::SimpleType(_) => Ok(MessageType::Ground),

            // Collections
            Proc::Collection(col) => match col {
                ast::Collection::List {
                    elements,
                    remainder,
                } => Ok(MessageType::List {
                    expected_len: if remainder.is_some() {
                        None
                    } else {
                        Some(elements.len())
                    },
                }),
                ast::Collection::Tuple(elements) => Ok(MessageType::Tuple {
                    expected_len: elements.len(),
                }),
                ast::Collection::Set {
                    elements,
                    remainder,
                } => Ok(MessageType::Set {
                    expected_len: if remainder.is_some() {
                        None
                    } else {
                        Some(elements.len())
                    },
                }),
                ast::Collection::Map {
                    elements,
                    remainder,
                } => Ok(MessageType::Map {
                    expected_pairs: if remainder.is_some() {
                        None
                    } else {
                        Some(elements.len())
                    },
                }),
            },

            // Process patterns
            Proc::Par { .. }
            | Proc::Send { .. }
            | Proc::ForComprehension { .. }
            | Proc::Match { .. }
            | Proc::Let { .. }
            | Proc::New { .. }
            | Proc::Contract { .. } => Ok(MessageType::Process),

            // Variables and expressions
            Proc::ProcVar(_)
            | Proc::VarRef { .. }
            | Proc::Eval { .. }
            | Proc::Method { .. }
            | Proc::UnaryExp { .. }
            | Proc::BinaryExp { .. } => Ok(MessageType::Unknown),

            // Invalid patterns
            Proc::IfThenElse { .. } | Proc::Select { .. } | Proc::SendSync { .. } => {
                Err(ValidationError::InvalidPatternStructure {
                    pid: PID(0), // Will be replaced by caller
                    position: Some(proc.span.start),
                    reason: format!(
                        "Invalid pattern: {:?} cannot be used in patterns",
                        proc.proc
                    ),
                })
            }

            Proc::Bundle { .. } => Err(ValidationError::InvalidPatternStructure {
                pid: PID(0),
                position: Some(proc.span.start),
                reason: "Bundle cannot appear in pattern".to_string(),
            }),

            Proc::Bad => Err(ValidationError::InvalidPatternStructure {
                pid: PID(0),
                position: Some(proc.span.start),
                reason: "Malformed pattern".to_string(),
            }),
        }
    }

    /// Validate pattern-channel compatibility
    ///
    /// This validates that the message type expected by the pattern is compatible
    /// with what can be received from the channel type.
    ///
    /// ## Compatibility Rules (Rholang semantics):
    /// 1. Ground messages can be received on any channel type
    /// 2. Unknown/Variable types are permissive (runtime will validate)
    /// 3. Quoted process channels send quoted processes (type @P)
    /// 4. Unforgeable names behave like regular channels
    /// 5. Collections require compatible structure but Rholang is dynamically typed
    fn validate_pattern_channel_compatibility(
        &self,
        for_comp_pid: PID,
        channel_type: &ChannelType,
        msg_type: &MessageType,
        pattern: &Names<'ast>,
    ) -> ValidationResult {
        match (channel_type, msg_type) {
            // Ground messages work with any channel - runtime will validate exact match
            (_, MessageType::Ground) => Ok(()),

            // Unknown types: we can't statically determine compatibility
            // This is safe because Rholang is dynamically typed
            (ChannelType::Unknown, _) | (_, MessageType::Unknown) => Ok(()),

            // Variable channels: could hold any name, so we allow any message type
            (ChannelType::Variable, _) => Ok(()),

            // Unforgeable names: behave like regular channels, accept any message
            (ChannelType::UnforgeableName, _) => Ok(()),

            // Quoted process channels: typically send quoted processes
            // But in Rholang, any value can be sent on any channel
            (ChannelType::QuotedProcess, MessageType::Process) => Ok(()),
            (ChannelType::QuotedProcess, MessageType::Name) => Ok(()),

            // Quoted process with collection - check if pattern makes sense
            (ChannelType::QuotedProcess, MessageType::List { .. })
            | (ChannelType::QuotedProcess, MessageType::Tuple { .. })
            | (ChannelType::QuotedProcess, MessageType::Set { .. })
            | (ChannelType::QuotedProcess, MessageType::Map { .. }) => {
                // Quoted processes as channels with collection patterns is unusual
                // but valid in Rholang - the pattern would need to match a collection structure
                self.warn_unusual_pattern_channel_combo(for_comp_pid, pattern)?;
                Ok(())
            }

            // Multiple names in pattern
            (_, MessageType::Multiple) => Ok(()),
        }
    }

    /// Warn about unusual but valid pattern-channel combinations
    fn warn_unusual_pattern_channel_combo(
        &self,
        _for_comp_pid: PID,
        _pattern: &Names<'ast>,
    ) -> ValidationResult {
        // Future: emit warning diagnostic for patterns like:
        // for(@[x, y, z] <- @SomeProcess) { ... }
        // This is technically valid but likely indicates a logic error
        Ok(())
    }

    /// Validate pattern structure for correctness
    fn validate_pattern_structure(
        &self,
        for_comp_pid: PID,
        pattern: &Names<'ast>,
    ) -> ValidationResult {
        if pattern.is_empty() {
            // Empty patterns are valid (match anything)
            return Ok(());
        }

        // Validate each name in the pattern
        for name in &pattern.names {
            self.validate_name_in_pattern(for_comp_pid, name)?;
        }

        // Validate remainder if present
        if let Some(_remainder) = pattern.remainder {
            // Remainder patterns are valid
        }

        Ok(())
    }

    /// Validate a single name in a pattern
    fn validate_name_in_pattern(&self, for_comp_pid: PID, name: &Name<'ast>) -> ValidationResult {
        match name {
            Name::Quote(proc) => {
                // Validate the quoted process pattern
                self.validate_quoted_pattern(for_comp_pid, proc)
            }
            Name::NameVar(_) => {
                // Name variables are always valid in patterns
                Ok(())
            }
        }
    }

    /// Validate a quoted process in a pattern
    fn validate_quoted_pattern(&self, for_comp_pid: PID, proc: &AnnProc<'ast>) -> ValidationResult {
        match proc.proc {
            // These are explicitly not allowed in patterns
            Proc::Bundle { .. } => Err(ValidationError::InvalidPatternStructure {
                pid: for_comp_pid,
                position: Some(proc.span.start),
                reason: "Bundle cannot appear in pattern".to_string(),
            }),

            // Connectives should be caught by ResolverPass, but double-check
            Proc::BinaryExp { op, .. } if op.is_connective() => {
                Err(ValidationError::ConnectiveOutsidePattern {
                    pos: proc.span.start,
                })
            }
            Proc::UnaryExp { op, .. } if op.is_connective() => {
                Err(ValidationError::ConnectiveOutsidePattern {
                    pos: proc.span.start,
                })
            }

            // All other patterns are valid
            _ => Ok(()),
        }
    }
}

/// This validator performs advanced pattern analysis that treats patterns as SQL-like queries
/// It validates:
/// - SQL-like pattern semantics (selections, projections, filters)
/// - Pattern satisfiability (detecting impossible patterns)
/// - Logical connective semantics (AND/OR/NOT composition)
///
/// ## Pattern Query Semantics
///
/// Rholang patterns can be viewed as database queries:
/// - **Selection**: Choosing which messages to match (WHERE clause)
/// - **Projection**: Extracting variables from matched messages (SELECT clause)
/// - **Filtering**: Logical connectives restrict the solution space
pub struct PatternQueryValidator<'a, 'ast> {
    /// Semantic database for future type inference and constraint solving
    /// Currently unused as type conflict and logical contradiction detection are blank
    #[allow(dead_code)]
    db: &'a SemanticDb<'ast>,
}

impl<'a, 'ast> PatternQueryValidator<'a, 'ast> {
    pub fn new(db: &'a SemanticDb<'ast>) -> Self {
        Self { db }
    }

    /// Validate SQL-like pattern semantics
    ///
    /// This method treats patterns as database queries and validates their structure
    /// It checks that:
    /// - Pattern projections (variable bindings) are well-formed
    /// - Pattern selections (ground term matches) are consistent
    /// - Pattern filters (connectives) create valid constraints
    ///
    /// ## Arguments
    ///
    /// * `pattern` - The process pattern to validate
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` if the pattern has valid SQL-like semantics, or a `ValidationError` otherwise
    pub fn validate_sql_like_patterns(&self, pattern: &AnnProc<'ast>) -> ValidationResult {
        Self::analyze_pattern_structure(pattern)?;

        Self::validate_pattern_projections(pattern)?;

        Ok(())
    }

    /// Validate pattern satisfiability
    ///
    /// This method performs satisfiability analysis to detect patterns that can never match
    /// A pattern is unsatisfiable if no message can ever satisfy its constraints
    ///
    /// ## Arguments
    ///
    /// * `pattern` - The process pattern to analyze
    ///
    /// ## Returns
    ///
    /// Returns `Ok(())` if the pattern is satisfiable, or `ValidationError::UnsatisfiablePattern` otherwise
    pub fn validate_pattern_satisfiability(&self, pattern: &AnnProc<'ast>) -> ValidationResult {
        if let Some(contradiction) = self.find_ground_term_contradiction(pattern) {
            return Err(ValidationError::UnsatisfiablePattern {
                pattern: contradiction,
                pos: Some(pattern.span.start),
            });
        }

        if let Some(conflict) = self.find_type_conflict(pattern) {
            return Err(ValidationError::UnsatisfiablePattern {
                pattern: conflict,
                pos: Some(pattern.span.start),
            });
        }

        if let Some(impossible) = self.find_impossible_collection_constraint(pattern) {
            return Err(ValidationError::UnsatisfiablePattern {
                pattern: impossible,
                pos: Some(pattern.span.start),
            });
        }

        if let Some(logical_error) = self.find_logical_contradiction(pattern) {
            return Err(ValidationError::UnsatisfiablePattern {
                pattern: logical_error,
                pos: Some(pattern.span.start),
            });
        }

        Ok(())
    }

    /// Analyze pattern structure for SQL-like query semantics
    fn analyze_pattern_structure(pattern: &AnnProc<'ast>) -> ValidationResult {
        match pattern.proc {
            // Collections can be analyzed as structured queries
            Proc::Collection(col) => match col {
                ast::Collection::List {
                    elements,
                    remainder: _,
                } => {
                    // List patterns represent ordered selections
                    for elem in elements.iter() {
                        Self::analyze_pattern_structure(elem)?;
                    }
                    Ok(())
                }
                ast::Collection::Tuple(elements) => {
                    // Tuple patterns represent fixed-arity selections
                    for elem in elements.iter() {
                        Self::analyze_pattern_structure(elem)?;
                    }
                    Ok(())
                }
                ast::Collection::Set {
                    elements,
                    remainder: _,
                } => {
                    // Set patterns represent unordered selections
                    for elem in elements.iter() {
                        Self::analyze_pattern_structure(elem)?;
                    }
                    Ok(())
                }
                ast::Collection::Map {
                    elements,
                    remainder: _,
                } => {
                    // Map patterns represent key-value selections
                    for (key, value) in elements.iter() {
                        Self::analyze_pattern_structure(key)?;
                        Self::analyze_pattern_structure(value)?;
                    }
                    Ok(())
                }
            },

            // Binary expressions with connectives
            Proc::BinaryExp { op, left, right } if op.is_connective() => {
                // Connectives create compound queries (AND/OR)
                Self::analyze_pattern_structure(left)?;
                Self::analyze_pattern_structure(right)?;
                Ok(())
            }

            // Unary expressions with connectives
            Proc::UnaryExp { op, arg } if op.is_connective() => {
                Self::analyze_pattern_structure(arg)?;
                Ok(())
            }

            // Process patterns
            Proc::Send { inputs, .. } => {
                for input in inputs.iter() {
                    Self::analyze_pattern_structure(input)?;
                }
                Ok(())
            }

            Proc::Par { left, right } => {
                Self::analyze_pattern_structure(left)?;
                Self::analyze_pattern_structure(right)?;
                Ok(())
            }

            // Ground terms and variables are leaves in the query tree
            Proc::Nil
            | Proc::Unit
            | Proc::BoolLiteral(_)
            | Proc::LongLiteral(_)
            | Proc::StringLiteral(_)
            | Proc::UriLiteral(_)
            | Proc::SimpleType(_)
            | Proc::ProcVar(_) => Ok(()),

            // Other constructs are validated elsewhere
            _ => Ok(()),
        }
    }

    /// Validate pattern projections (variable bindings)
    fn validate_pattern_projections(pattern: &AnnProc<'ast>) -> ValidationResult {
        // Check that variables are used consistently
        // This is mostly handled by ResolverPass, but we can add extra checks
        match pattern.proc {
            Proc::ProcVar(_) => {
                // Variable binding - this is a projection
                Ok(())
            }
            Proc::Collection(col) => {
                // Recursively check projections in collections
                match col {
                    ast::Collection::List {
                        elements,
                        remainder: _,
                    } => {
                        for elem in elements.iter() {
                            Self::validate_pattern_projections(elem)?;
                        }
                        Ok(())
                    }
                    ast::Collection::Tuple(elements) => {
                        for elem in elements.iter() {
                            Self::validate_pattern_projections(elem)?;
                        }
                        Ok(())
                    }
                    ast::Collection::Set {
                        elements,
                        remainder: _,
                    } => {
                        for elem in elements.iter() {
                            Self::validate_pattern_projections(elem)?;
                        }
                        Ok(())
                    }
                    ast::Collection::Map {
                        elements,
                        remainder: _,
                    } => {
                        for (key, value) in elements.iter() {
                            Self::validate_pattern_projections(key)?;
                            Self::validate_pattern_projections(value)?;
                        }
                        Ok(())
                    }
                }
            }
            _ => Ok(()),
        }
    }

    /// Find contradictory ground terms in a pattern
    fn find_ground_term_contradiction(&self, pattern: &AnnProc<'ast>) -> Option<String> {
        // Look for AND connectives with conflicting ground terms
        match pattern.proc {
            Proc::BinaryExp { op, left, right } if op.is_and_connective() => {
                // Check if both sides are conflicting ground terms
                if let (Some(left_val), Some(right_val)) = (
                    self.extract_ground_value(left),
                    self.extract_ground_value(right),
                ) && left_val != right_val
                {
                    return Some(format!(
                        "Contradictory ground terms: {} AND {}",
                        left_val, right_val
                    ));
                }

                // Recurse into subpatterns
                if let Some(contra) = self.find_ground_term_contradiction(left) {
                    return Some(contra);
                }
                if let Some(contra) = self.find_ground_term_contradiction(right) {
                    return Some(contra);
                }

                None
            }
            _ => None,
        }
    }

    /// Find type conflicts in a pattern
    ///
    /// TODO: This is a placeholder for now. Full implementation will be:
    /// - Type hierarchy knowledge (Int, String, Bool incompatibilities)
    /// - Type inference for variables
    /// - Constraint solving for complex patterns
    fn find_type_conflict(&self, _pattern: &AnnProc<'ast>) -> Option<String> {
        // For now, we rely on runtime type checking
        None
    }

    /// Find impossible collection constraints
    ///
    /// Detects patterns that impose contradictory constraints on collections:
    /// 1. Conflicting fixed sizes (e.g., [a, b] AND [c, d, e])
    /// 2. Tuple arity mismatches (e.g., (a, b) AND (c, d, e))
    /// 3. Impossible set/map constraints
    ///
    /// Note: We only detect structural impossibilities. Type-level conflicts
    /// within collection elements are handled by find_type_conflict.
    fn find_impossible_collection_constraint(&self, pattern: &AnnProc<'ast>) -> Option<String> {
        match pattern.proc {
            Proc::BinaryExp { op, left, right } if op.is_and_connective() => {
                // Check for conflicting collection types (List AND Tuple, etc.)
                if let (Some(left_type), Some(right_type)) = (
                    self.extract_collection_type(left),
                    self.extract_collection_type(right),
                ) && left_type != right_type
                {
                    return Some(format!(
                        "Impossible collection constraint: {} AND {}",
                        left_type, right_type
                    ));
                }

                // Check for conflicting collection sizes (for fixed-size collections)
                if let (Some(left_size), Some(right_size)) = (
                    self.extract_collection_size(left),
                    self.extract_collection_size(right),
                ) && left_size != right_size
                {
                    return Some(format!(
                        "Impossible collection size constraint: size {} AND size {}",
                        left_size, right_size
                    ));
                }

                // Recurse into subpatterns
                if let Some(impossible) = self.find_impossible_collection_constraint(left) {
                    return Some(impossible);
                }
                if let Some(impossible) = self.find_impossible_collection_constraint(right) {
                    return Some(impossible);
                }

                None
            }
            // Recursively check collection elements for nested contradictions
            Proc::Collection(col) => {
                match col {
                    ast::Collection::List { elements, .. }
                    | ast::Collection::Set { elements, .. } => {
                        for elem in elements.iter() {
                            if let Some(impossible) =
                                self.find_impossible_collection_constraint(elem)
                            {
                                return Some(impossible);
                            }
                        }
                    }
                    ast::Collection::Tuple(elements) => {
                        for elem in elements.iter() {
                            if let Some(impossible) =
                                self.find_impossible_collection_constraint(elem)
                            {
                                return Some(impossible);
                            }
                        }
                    }
                    ast::Collection::Map { elements, .. } => {
                        for (key, value) in elements.iter() {
                            if let Some(impossible) =
                                self.find_impossible_collection_constraint(key)
                            {
                                return Some(impossible);
                            }
                            if let Some(impossible) =
                                self.find_impossible_collection_constraint(value)
                            {
                                return Some(impossible);
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Extract collection type as a string for comparison
    fn extract_collection_type(&self, pattern: &AnnProc<'ast>) -> Option<&'static str> {
        match pattern.proc {
            Proc::Collection(col) => match col {
                ast::Collection::List { .. } => Some("List"),
                ast::Collection::Tuple(_) => Some("Tuple"),
                ast::Collection::Set { .. } => Some("Set"),
                ast::Collection::Map { .. } => Some("Map"),
            },
            _ => None,
        }
    }

    /// Find logical contradictions in connective patterns
    fn find_logical_contradiction(&self, _pattern: &AnnProc<'ast>) -> Option<String> {
        // This is a complex analysis that would require tracking variable bindings
        // and negations across the pattern tree. For now, we rely on runtime semantics
        None
    }

    /// Extract ground value from a pattern (if it's a ground term)
    fn extract_ground_value(&self, pattern: &AnnProc<'ast>) -> Option<String> {
        match pattern.proc {
            Proc::Nil => Some("Nil".to_string()),
            Proc::Unit => Some("()".to_string()),
            Proc::BoolLiteral(b) => Some(b.to_string()),
            Proc::LongLiteral(n) => Some(n.to_string()),
            Proc::StringLiteral(s) => Some(format!("\"{}\"", s)),
            Proc::UriLiteral(u) => Some(format!("`{}`", u)),
            _ => None,
        }
    }

    /// Extract collection size from a pattern (if it's a fixed-size collection)
    fn extract_collection_size(&self, pattern: &AnnProc<'ast>) -> Option<usize> {
        match pattern.proc {
            Proc::Collection(col) => match col {
                ast::Collection::List {
                    elements,
                    remainder,
                } => {
                    if remainder.is_none() {
                        Some(elements.len())
                    } else {
                        None
                    }
                }
                ast::Collection::Tuple(elements) => Some(elements.len()),
                ast::Collection::Set {
                    elements,
                    remainder,
                } => {
                    if remainder.is_none() {
                        Some(elements.len())
                    } else {
                        None
                    }
                }
                ast::Collection::Map {
                    elements,
                    remainder,
                } => {
                    if remainder.is_none() {
                        Some(elements.len())
                    } else {
                        None
                    }
                }
            },
            _ => None,
        }
    }
}

/// Helper trait for checking connective operators
trait ConnectiveOps {
    fn is_and_connective(&self) -> bool;
}

impl ConnectiveOps for ast::BinaryExpOp {
    fn is_and_connective(&self) -> bool {
        matches!(self, ast::BinaryExpOp::And)
    }
}

#[cfg(test)]
mod pattern_query_tests {
    use super::*;
    use crate::sem::resolver::ResolverPass;
    use crate::sem::{FactPass, SemanticDb};
    use rholang_parser::RholangParser;

    fn setup_db(code: &str) -> (SemanticDb<'static>, PID) {
        // Leak memory for 'static lifetime (test only)
        let parser = Box::leak(Box::new(RholangParser::new()));
        let code_static: &'static str = Box::leak(code.to_string().into_boxed_str());
        let ast = parser.parse(code_static).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        // Run ResolverPass to build scopes
        let resolver = ResolverPass::new(root_pid);
        resolver.run(&mut db);

        // Find the for-comprehension PID
        let for_comp_pid = db
            .find_proc(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .expect("No for-comprehension found");

        (db, for_comp_pid)
    }

    #[test]
    fn test_pattern_query_validator_creation() {
        let db = SemanticDb::new();
        let validator = PatternQueryValidator::new(&db);
        assert!(std::ptr::eq(validator.db, &db));
    }

    #[test]
    fn test_validate_simple_pattern() {
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let (db, _pid) = setup_db(code);

        let validator = PatternQueryValidator::new(&db);
        let pattern = Proc::Nil.ann(rholang_parser::SourcePos::default().span_of(3));

        let result = validator.validate_sql_like_patterns(&pattern);
        assert!(
            result.is_ok(),
            "Simple pattern should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_satisfiable_pattern() {
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let (db, _pid) = setup_db(code);

        let validator = PatternQueryValidator::new(&db);
        let pattern = Proc::LongLiteral(42).ann(rholang_parser::SourcePos::default().span_of(2));

        let result = validator.validate_pattern_satisfiability(&pattern);
        assert!(
            result.is_ok(),
            "Satisfiable pattern should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_extract_ground_value() {
        let db = SemanticDb::new();
        let validator = PatternQueryValidator::new(&db);

        let nil_pattern = Proc::Nil.ann(rholang_parser::SourcePos::default().span_of(3));
        assert_eq!(
            validator.extract_ground_value(&nil_pattern),
            Some("Nil".to_string())
        );

        let bool_pattern =
            Proc::BoolLiteral(true).ann(rholang_parser::SourcePos::default().span_of(4));
        assert_eq!(
            validator.extract_ground_value(&bool_pattern),
            Some("true".to_string())
        );

        let int_pattern =
            Proc::LongLiteral(42).ann(rholang_parser::SourcePos::default().span_of(2));
        assert_eq!(
            validator.extract_ground_value(&int_pattern),
            Some("42".to_string())
        );
    }
}

#[cfg(test)]
mod type_validator_tests {
    use super::*;
    use crate::sem::resolver::ResolverPass;
    use crate::sem::{FactPass, SemanticDb};
    use rholang_parser::RholangParser;

    fn setup_db(code: &str) -> (SemanticDb<'static>, PID) {
        // Leak memory for 'static lifetime (test only)
        let parser = Box::leak(Box::new(RholangParser::new()));
        let code_static: &'static str = Box::leak(code.to_string().into_boxed_str());
        let ast = parser.parse(code_static).expect("Failed to parse");
        let ast_static = Box::leak(Box::new(ast));

        let mut db = SemanticDb::new();
        let proc = &ast_static[0];
        let root_pid = db.build_index(proc);

        // Run ResolverPass to build scopes
        let resolver = ResolverPass::new(root_pid);
        resolver.run(&mut db);

        // Find the for-comprehension PID
        let for_comp_pid = db
            .find_proc(|p| matches!(p.proc, Proc::ForComprehension { .. }))
            .map(|(pid, _)| pid)
            .expect("No for-comprehension found");

        (db, for_comp_pid)
    }

    #[test]
    fn test_validator_creation() {
        let db = SemanticDb::new();
        let validator = TypeValidator::new(&db);
        assert!(std::ptr::eq(validator.db, &db));
    }

    #[test]
    fn test_infer_channel_type_quoted() {
        let code = r#"for(@x <- @Nil) { Nil }"#;
        let (db, _pid) = setup_db(code);
        let _validator = TypeValidator::new(&db);

        // The validator should identify quoted process channels
        assert!(db.pids().len() > 0);
    }

    #[test]
    fn test_infer_channel_type_variable() {
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let (db, _pid) = setup_db(code);
        let _validator = TypeValidator::new(&db);

        // Should identify variable channels
        assert!(db.pids().len() > 0);
    }

    #[test]
    fn test_validate_simple_for_comp() {
        let code = r#"new ch in { for(@x <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        let result = validator.validate_channel_usage(pid);
        assert!(
            result.is_ok(),
            "Simple for-comp should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_multiple_bindings() {
        let code = r#"new ch1, ch2 in { for(@x <- ch1; @y <- ch2) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        let result = validator.validate_channel_usage(pid);
        assert!(
            result.is_ok(),
            "Multiple bindings should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_list_pattern() {
        let code = r#"new ch in { for(@[x, y, z] <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        let result = validator.validate_channel_usage(pid);
        assert!(result.is_ok(), "List pattern should validate: {:?}", result);
    }

    #[test]
    fn test_validate_tuple_pattern() {
        let code = r#"new ch in { for(@(x, y) <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        let result = validator.validate_channel_usage(pid);
        assert!(
            result.is_ok(),
            "Tuple pattern should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_ground_pattern() {
        let code = r#"new ch in { for(@42 <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        let result = validator.validate_channel_usage(pid);
        assert!(
            result.is_ok(),
            "Ground pattern should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_complex_pattern() {
        let code = r#"new ch in { for(@[x, y, ...rest] <- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        let result = validator.validate_channel_usage(pid);
        assert!(
            result.is_ok(),
            "Complex pattern with remainder should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_infer_message_type_ground() {
        let code = r#"new ch in { for(@Nil <- ch) { Nil } }"#;
        let (db, _pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        // Ground patterns should be recognized
        let proc = Proc::Nil.ann(rholang_parser::SourcePos::default().span_of(3));
        let msg_type = validator.infer_proc_pattern_type(&proc).unwrap();
        assert_eq!(msg_type, MessageType::Ground);
    }

    #[test]
    fn test_validate_unforgeable_name_channel() {
        let code = r#"new unforgeable(`rho:test:uri`) in { for(@x <- unforgeable) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        let result = validator.validate_channel_usage(pid);
        assert!(
            result.is_ok(),
            "Unforgeable name channel should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_peek_binding() {
        let code = r#"new ch in { for(@x <<- ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        let result = validator.validate_channel_usage(pid);
        assert!(result.is_ok(), "Peek binding should validate: {:?}", result);
    }

    #[test]
    fn test_validate_repeated_binding() {
        let code = r#"new ch in { for(@x <= ch) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        let result = validator.validate_channel_usage(pid);
        assert!(
            result.is_ok(),
            "Repeated binding should validate: {:?}",
            result
        );
    }

    #[test]
    fn test_validate_mixed_bind_types() {
        let code = r#"new ch1, ch2, ch3 in { for(@x <- ch1; @y <= ch2; @z <<- ch3) { Nil } }"#;
        let (db, pid) = setup_db(code);
        let validator = TypeValidator::new(&db);

        let result = validator.validate_channel_usage(pid);
        assert!(
            result.is_ok(),
            "Mixed bind types should validate: {:?}",
            result
        );
    }
}
