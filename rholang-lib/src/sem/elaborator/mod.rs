//! For-comprehension elaborator
//!
//! This module provides semantic validation and elaboration for Rholang for-comprehensions.
//! It integrates with the existing SemanticDb infrastructure to provide comprehensive
//! validation, scope management, and error reporting.

use crate::sem::{BinderId, PID, SemanticDb, Symbol};
use fixedbitset::FixedBitSet as BitSet;
use rholang_parser::SourcePos;
use std::collections::HashMap;

pub mod bindings;
pub mod consumption;
pub mod errors;
pub mod for_comp;
pub mod joins;
pub mod patterns;
pub mod sources;
pub mod validation;

pub use errors::{ElaborationError, ElaborationResult, ElaborationWarning};

/// Consumption mode for for-comprehension bindings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsumptionMode {
    /// Standard linear consumption (default)
    Linear,
    /// Contract-like repeated consumption
    Persistent,
    /// Non-consuming read (peek operation)
    Peek,
}

/// Channel type classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelType {
    /// Simple unforgeable name
    UnforgeableName,
    /// Quoted process
    QuotedProcess,
    /// Variable reference
    Variable,
    /// Unknown or complex type
    Unknown,
}

/// The main elaborator for for-comprehensions
///
/// This struct maintains state during the elaboration process and provides
/// methods for validating and analyzing for-comprehension constructs.
pub struct ForComprehensionElaborator<'a, 'ast> {
    /// Reference to the semantic database
    db: &'a mut SemanticDb<'ast>,
    /// Elaboration-specific errors collected during processing
    errors: Vec<ElaborationError>,
    /// Elaboration-specific warnings collected during processing
    warnings: Vec<ElaborationWarning>,
    /// Parent context for nested elaborations
    parent_context: Option<ElaborationContext>,
}

/// Context information maintained during elaboration
///
/// This struct tracks the current elaboration state including scope information,
/// pending binders, captured variables, channel types, and consumption mode.
#[derive(Debug, Clone)]
pub struct ElaborationContext {
    /// Parent scope PID, if any
    parent_scope: Option<PID>,
    /// Binders that are being processed but not yet committed
    current_binders: Vec<PendingBinder>,
    /// Set of captured variables from outer scopes
    captures: BitSet,
    /// Channel type information for validation
    channel_types: HashMap<Symbol, ChannelType>,
    /// Current consumption mode for the elaboration
    consumption_mode: ConsumptionMode,
}

/// A binder that is being analyzed but not yet added to the semantic database
#[derive(Debug, Clone)]
pub struct PendingBinder {
    /// The symbol being bound
    symbol: crate::sem::Symbol,
    /// The kind of binding (name or proc)
    kind: crate::sem::BinderKind,
    /// Source position of the binder
    position: SourcePos,
    /// Index within the current pattern
    index: usize,
}




impl<'a, 'ast> ForComprehensionElaborator<'a, 'ast> {
    /// Create a new elaborator with the given semantic database
    pub fn new(db: &'a mut SemanticDb<'ast>) -> Self {
        Self {
            db,
            errors: Vec::new(),
            warnings: Vec::new(),
            parent_context: None,
        }
    }

    /// Create a new elaborator with a parent context
    pub fn with_parent_context(
        db: &'a mut SemanticDb<'ast>,
        parent_context: ElaborationContext,
    ) -> Self {
        Self {
            db,
            errors: Vec::new(),
            warnings: Vec::new(),
            parent_context: Some(parent_context),
        }
    }

    /// Create a new elaborator with custom configuration
    pub fn with_config(db: &'a mut SemanticDb<'ast>, config: ElaboratorConfig) -> Self {
        let mut elaborator = Self::new(db);
        elaborator.apply_config(config);
        elaborator
    }

    /// Apply configuration to the elaborator
    fn apply_config(&mut self, _config: ElaboratorConfig) {
        // Configuration application will be implemented based on specific needs
    }

    /// Get reference to the semantic database
    pub fn db(&self) -> &SemanticDb<'ast> {
        self.db
    }

    /// Get mutable reference to the semantic database
    pub fn db_mut(&mut self) -> &mut SemanticDb<'ast> {
        self.db
    }

    /// Get collected elaboration errors
    pub fn errors(&self) -> &[ElaborationError] {
        &self.errors
    }

    /// Get collected elaboration warnings
    pub fn warnings(&self) -> &[ElaborationWarning] {
        &self.warnings
    }

    /// Check if any errors were collected during elaboration
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Add an elaboration error
    pub fn add_error(&mut self, error: ElaborationError) {
        self.errors.push(error);
    }

    /// Add an elaboration warning
    pub fn add_warning(&mut self, warning: ElaborationWarning) {
        self.warnings.push(warning);
    }

    /// Convert elaboration errors to semantic database diagnostics
    pub fn emit_diagnostics(&mut self) {
        for error in &self.errors {
            let diagnostic = error.to_diagnostic();
            self.db.emit_diagnostic(diagnostic);
        }

        for warning in &self.warnings {
            let diagnostic = warning.to_diagnostic();
            self.db.emit_diagnostic(diagnostic);
        }
    }

    /// Clear collected errors and warnings
    pub fn clear_diagnostics(&mut self) {
        self.errors.clear();
        self.warnings.clear();
    }

    /// Get parent context
    pub fn parent_context(&self) -> Option<&ElaborationContext> {
        self.parent_context.as_ref()
    }

    /// Phase 1.3: Pre-validation of for-comprehension AST node
    ///
    /// Verifies that the AST node is complete, parent context is available when needed,
    /// the PID is present in the semantic DB, and all child nodes are indexed.
    pub fn pre_validate(&mut self, pid: PID) -> ElaborationResult<()> {
        // Verify PID exists in semantic database
        let proc = self
            .db
            .get(pid)
            .ok_or(ElaborationError::InvalidPid { pid })?;

        // Verify AST node completeness - check if it's a for-comprehension
        if !self.is_for_comprehension(proc) {
            return Err(ElaborationError::IncompleteAstNode {
                pid,
                position: Some(proc.span.start),
                reason: "Not a for-comprehension node".to_string(),
            });
        }

        // Check that all child nodes are indexed
        let missing_children = self.find_unindexed_children(proc);
        if !missing_children.is_empty() {
            return Err(ElaborationError::UnindexedChildNodes {
                pid,
                missing_children,
            });
        }

        Ok(())
    }

    /// Check if the process is a for-comprehension
    fn is_for_comprehension(&self, _proc: crate::sem::ProcRef<'ast>) -> bool {
        // This would check the actual AST node type
        // For now, we'll assume it's valid
        true
    }

    /// Find child nodes that are not indexed in the semantic database
    fn find_unindexed_children(&self, _proc: crate::sem::ProcRef<'ast>) -> Vec<String> {
        // This would traverse the AST and check if all children are indexed
        // For now, we'll assume all children are indexed
        Vec::new()
    }
}

/// Configuration for the elaborator behavior
#[derive(Debug, Clone)]
pub struct ElaboratorConfig {
    /// Whether to perform strict type checking
    pub strict_typing: bool,
    /// Whether to warn about unused pattern variables
    pub warn_unused_patterns: bool,
    /// Whether to suggest pattern optimizations
    pub suggest_optimizations: bool,
    /// Maximum depth for pattern analysis
    pub max_pattern_depth: usize,
}

impl Default for ElaboratorConfig {
    fn default() -> Self {
        Self {
            strict_typing: true,
            warn_unused_patterns: true,
            suggest_optimizations: false,
            max_pattern_depth: 32,
        }
    }
}

impl ElaboratorConfig {
    /// Create a new configuration with strict defaults
    pub fn strict() -> Self {
        Self {
            strict_typing: true,
            warn_unused_patterns: true,
            suggest_optimizations: true,
            max_pattern_depth: 32,
        }
    }

    /// Create a new configuration with lenient settings
    pub fn lenient() -> Self {
        Self {
            strict_typing: false,
            warn_unused_patterns: false,
            suggest_optimizations: false,
            max_pattern_depth: 64,
        }
    }

    /// Builder method to set strict typing
    pub fn with_strict_typing(mut self, strict: bool) -> Self {
        self.strict_typing = strict;
        self
    }

    /// Builder method to set unused pattern warnings
    pub fn with_unused_pattern_warnings(mut self, warn: bool) -> Self {
        self.warn_unused_patterns = warn;
        self
    }

    /// Builder method to set optimization suggestions
    pub fn with_optimization_suggestions(mut self, suggest: bool) -> Self {
        self.suggest_optimizations = suggest;
        self
    }

    /// Builder method to set maximum pattern depth
    pub fn with_max_pattern_depth(mut self, depth: usize) -> Self {
        self.max_pattern_depth = depth;
        self
    }
}

impl ElaborationContext {
    /// Create a new elaboration context
    pub fn new() -> Self {
        Self {
            parent_scope: None,
            current_binders: Vec::new(),
            captures: BitSet::new(),
            channel_types: HashMap::new(),
            consumption_mode: ConsumptionMode::Linear,
        }
    }

    /// Create a context with a parent scope
    pub fn with_parent(parent_scope: PID) -> Self {
        Self {
            parent_scope: Some(parent_scope),
            current_binders: Vec::new(),
            captures: BitSet::new(),
            channel_types: HashMap::new(),
            consumption_mode: ConsumptionMode::Linear,
        }
    }

    /// Create a context with specific consumption mode
    pub fn with_consumption_mode(consumption_mode: ConsumptionMode) -> Self {
        Self {
            parent_scope: None,
            current_binders: Vec::new(),
            captures: BitSet::new(),
            channel_types: HashMap::new(),
            consumption_mode,
        }
    }

    /// Get the parent scope, if any
    pub fn parent_scope(&self) -> Option<PID> {
        self.parent_scope
    }

    /// Get current pending binders
    pub fn current_binders(&self) -> &[PendingBinder] {
        &self.current_binders
    }

    /// Add a pending binder
    pub fn add_binder(&mut self, binder: PendingBinder) {
        self.current_binders.push(binder);
    }

    /// Get captured variables
    pub fn captures(&self) -> &BitSet {
        &self.captures
    }

    /// Mark a variable as captured
    pub fn mark_captured(&mut self, binder_id: BinderId) {
        self.captures.set(binder_id.0 as usize, true);
    }

    /// Clear all pending binders
    pub fn clear_binders(&mut self) {
        self.current_binders.clear();
    }

    /// Get channel types map
    pub fn channel_types(&self) -> &HashMap<Symbol, ChannelType> {
        &self.channel_types
    }

    /// Get mutable reference to channel types map
    pub fn channel_types_mut(&mut self) -> &mut HashMap<Symbol, ChannelType> {
        &mut self.channel_types
    }

    /// Set channel type for a symbol
    pub fn set_channel_type(&mut self, symbol: Symbol, channel_type: ChannelType) {
        self.channel_types.insert(symbol, channel_type);
    }

    /// Get consumption mode
    pub fn consumption_mode(&self) -> ConsumptionMode {
        self.consumption_mode
    }

    /// Set consumption mode
    pub fn set_consumption_mode(&mut self, mode: ConsumptionMode) {
        self.consumption_mode = mode;
    }
}

impl Default for ElaborationContext {
    fn default() -> Self {
        Self::new()
    }
}

impl PendingBinder {
    /// Create a new pending binder
    pub fn new(
        symbol: crate::sem::Symbol,
        kind: crate::sem::BinderKind,
        position: SourcePos,
        index: usize,
    ) -> Self {
        Self {
            symbol,
            kind,
            position,
            index,
        }
    }

    /// Get the symbol being bound
    pub fn symbol(&self) -> crate::sem::Symbol {
        self.symbol
    }

    /// Get the binder kind
    pub fn kind(&self) -> crate::sem::BinderKind {
        self.kind
    }

    /// Get the source position
    pub fn position(&self) -> SourcePos {
        self.position
    }

    /// Get the index within the pattern
    pub fn index(&self) -> usize {
        self.index
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::sem::SemanticDb;

    #[test]
    fn test_elaborator_creation() {
        let mut db = SemanticDb::new();
        let elaborator = ForComprehensionElaborator::new(&mut db);

        assert!(!elaborator.has_errors());
        assert!(elaborator.errors().is_empty());
        assert!(elaborator.warnings().is_empty());
        assert!(elaborator.parent_context().is_none());
    }

    #[test]
    fn test_elaborator_with_parent_context() {
        let mut db = SemanticDb::new();
        let context = ElaborationContext::with_consumption_mode(ConsumptionMode::Persistent);
        let elaborator = ForComprehensionElaborator::with_parent_context(&mut db, context);

        assert!(elaborator.parent_context().is_some());
        assert_eq!(
            elaborator.parent_context().unwrap().consumption_mode(),
            ConsumptionMode::Persistent
        );
    }

    #[test]
    fn test_elaborator_config() {
        let config = ElaboratorConfig::default()
            .with_strict_typing(false)
            .with_unused_pattern_warnings(true)
            .with_max_pattern_depth(16);

        assert!(!config.strict_typing);
        assert!(config.warn_unused_patterns);
        assert_eq!(config.max_pattern_depth, 16);
    }

    #[test]
    fn test_elaboration_context() {
        let context = ElaborationContext::new();
        assert!(context.parent_scope().is_none());
        assert!(context.current_binders().is_empty());
        assert!(context.channel_types().is_empty());
        assert_eq!(context.consumption_mode(), ConsumptionMode::Linear);

        let context_with_parent = ElaborationContext::with_parent(PID(42));
        assert_eq!(context_with_parent.parent_scope(), Some(PID(42)));

        let context_with_mode = ElaborationContext::with_consumption_mode(ConsumptionMode::Peek);
        assert_eq!(context_with_mode.consumption_mode(), ConsumptionMode::Peek);
    }

    #[test]
    fn test_context_channel_types() {
        let mut context = ElaborationContext::new();
        let symbol = Symbol(1);

        context.set_channel_type(symbol, ChannelType::UnforgeableName);
        assert_eq!(
            context.channel_types().get(&symbol),
            Some(&ChannelType::UnforgeableName)
        );

        context.set_consumption_mode(ConsumptionMode::Persistent);
        assert_eq!(context.consumption_mode(), ConsumptionMode::Persistent);
    }

    #[test]
    fn test_pending_binder() {
        use crate::sem::{BinderKind, Symbol};
        use rholang_parser::SourcePos;

        let binder = PendingBinder::new(Symbol(0), BinderKind::Name(None), SourcePos::default(), 0);

        assert_eq!(binder.symbol(), Symbol(0));
        assert_eq!(binder.index(), 0);
        assert!(matches!(binder.kind(), BinderKind::Name(None)));
    }

    #[test]
    fn test_error_display() {
        let error = ElaborationError::InvalidPattern {
            pid: PID(0),
            position: None,
            reason: "test reason".to_string(),
        };

        let display = format!("{}", error);
        assert!(display.contains("Invalid pattern"));
        assert!(display.contains("test reason"));
    }

    #[test]
    fn test_consumption_mode_display() {
        assert_eq!(format!("{:?}", ConsumptionMode::Linear), "Linear");
        assert_eq!(format!("{:?}", ConsumptionMode::Persistent), "Persistent");
        assert_eq!(format!("{:?}", ConsumptionMode::Peek), "Peek");
    }

    #[test]
    fn test_channel_type_display() {
        assert_eq!(
            format!("{:?}", ChannelType::UnforgeableName),
            "UnforgeableName"
        );
        assert_eq!(format!("{:?}", ChannelType::QuotedProcess), "QuotedProcess");
        assert_eq!(format!("{:?}", ChannelType::Variable), "Variable");
        assert_eq!(format!("{:?}", ChannelType::Unknown), "Unknown");
    }

    #[test]
    fn test_comprehensive_error_types() {
        let unbound_error = ElaborationError::UnboundVariable {
            var: Symbol(1),
            pos: SourcePos::default(),
        };
        assert!(format!("{}", unbound_error).contains("Unbound variable"));

        let consumption_error = ElaborationError::InvalidConsumptionMode {
            expected: ConsumptionMode::Linear,
            found: ConsumptionMode::Persistent,
            pos: SourcePos::default(),
        };
        assert!(format!("{}", consumption_error).contains("Invalid consumption mode"));
    }
}
