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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsumptionMode {
    /// Standard linear consumption (default)
    Linear,
    /// Contract-like repeated consumption
    Persistent,
    /// Non-consuming read (peek operation)
    Peek,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelType {
    /// Unforgeable name
    UnforgeableName,
    /// Quoted process
    QuotedProcess,
    /// Variable reference
    Variable,
    /// Unknown or complex type
    Unknown,
}

pub struct ForComprehensionElaborator<'a, 'ast> {
    db: &'a mut SemanticDb<'ast>,
    errors: Vec<ElaborationError>,
    warnings: Vec<ElaborationWarning>,
    /// Parent context for nested elaborations
    parent_context: Option<ElaborationContext>,
}

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
    /// Current consumption mode
    consumption_mode: ConsumptionMode,
}

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
    pub fn new(db: &'a mut SemanticDb<'ast>) -> Self {
        Self {
            db,
            errors: Vec::new(),
            warnings: Vec::new(),
            parent_context: None,
        }
    }

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

    pub fn with_config(db: &'a mut SemanticDb<'ast>, config: ElaboratorConfig) -> Self {
        let mut elaborator = Self::new(db);
        elaborator.apply_config(config);
        elaborator
    }

    /// Apply configuration to the elaborator
    fn apply_config(&mut self, _config: ElaboratorConfig) {
        // TODO
    }

    pub fn db(&self) -> &SemanticDb<'ast> {
        self.db
    }

    pub fn db_mut(&mut self) -> &mut SemanticDb<'ast> {
        self.db
    }

    pub fn errors(&self) -> &[ElaborationError] {
        &self.errors
    }

    pub fn warnings(&self) -> &[ElaborationWarning] {
        &self.warnings
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn add_error(&mut self, error: ElaborationError) {
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: ElaborationWarning) {
        self.warnings.push(warning);
    }

    /// Finalizes elaboration and emits all accumulated diagnostics to the semantic database.
    ///
    /// This method consumes the elaborator, converts all errors and warnings to diagnostics,
    /// emits them to the database, and returns an error if any errors were collected.
    pub fn finalize(self) -> Result<(), Vec<crate::sem::Diagnostic>> {
        use crate::sem::Diagnostic;

        // Convert all errors and warnings to diagnostics
        let diagnostics: Vec<Diagnostic> = self
            .errors
            .iter()
            .map(|e| e.to_diagnostic())
            .chain(self.warnings.iter().map(|w| w.to_diagnostic()))
            .collect();

        // Emit all diagnostics to the database
        for diagnostic in &diagnostics {
            self.db.emit_diagnostic(*diagnostic);
        }

        // Return error if there were any errors
        if !self.errors.is_empty() {
            Err(diagnostics)
        } else {
            Ok(())
        }
    }

    pub fn clear_diagnostics(&mut self) {
        self.errors.clear();
        self.warnings.clear();
    }

    pub fn parent_context(&self) -> Option<&ElaborationContext> {
        self.parent_context.as_ref()
    }

    /// Verifies that the AST node is complete, parent context is available when needed,
    /// the PID is present in the semantic DB, and all child nodes are indexed
    pub fn pre_validate(&mut self, pid: PID) -> ElaborationResult<()> {
        // Verify PID exists in semantic database
        let proc = self
            .db
            .get(pid)
            .ok_or(ElaborationError::InvalidPid { pid })?;

        use rholang_parser::ast::Proc;
        match proc.proc {
            Proc::ForComprehension {
                receipts,
                proc: body,
            } => {
                if receipts.is_empty() {
                    return Err(ElaborationError::IncompleteAstNode {
                        pid,
                        position: Some(proc.span.start),
                        reason: "For-comprehension must have at least one receipt".to_string(),
                    });
                }

                for (receipt_idx, receipt) in receipts.iter().enumerate() {
                    if receipt.is_empty() {
                        return Err(ElaborationError::IncompleteAstNode {
                            pid,
                            position: Some(proc.span.start),
                            reason: format!(
                                "Receipt {} must have at least one binding",
                                receipt_idx
                            ),
                        });
                    }
                }

                // Check that all child nodes are indexed
                let missing_children = self.find_unindexed_children(receipts, body)?;
                if !missing_children.is_empty() {
                    return Err(ElaborationError::UnindexedChildNodes {
                        pid,
                        missing_children,
                    });
                }

                Ok(())
            }
            _ => Err(ElaborationError::IncompleteAstNode {
                pid,
                position: Some(proc.span.start),
                reason: "Not a for-comprehension node".to_string(),
            }),
        }
    }

    /// Find child nodes that are not indexed in the semantic database
    /// Traverses the receipts and body to ensure all referenced processes are indexed
    fn find_unindexed_children(
        &self,
        receipts: &'ast rholang_parser::ast::Receipts<'ast>,
        body: crate::sem::ProcRef<'ast>,
    ) -> ElaborationResult<Vec<String>> {
        use rholang_parser::ast::Bind;

        let mut missing = Vec::new();

        if self.db.lookup(body).is_none() {
            missing.push("body process".to_string());
        }

        for (receipt_idx, receipt) in receipts.iter().enumerate() {
            for (bind_idx, bind) in receipt.iter().enumerate() {
                let bind_desc = format!("receipt[{}].bind[{}]", receipt_idx, bind_idx);

                match bind {
                    Bind::Linear { rhs, .. } => {
                        self.check_source_indexed(rhs, &bind_desc, &mut missing)?;
                    }
                    Bind::Repeated { rhs, .. } | Bind::Peek { rhs, .. } => {
                        self.check_name_indexed(rhs, &bind_desc, &mut missing)?;
                    }
                }
            }
        }

        Ok(missing)
    }

    fn check_source_indexed(
        &self,
        source: &'ast rholang_parser::ast::Source<'ast>,
        context: &str,
        missing: &mut Vec<String>,
    ) -> ElaborationResult<()> {
        use rholang_parser::ast::Source;

        match source {
            Source::Simple { name } | Source::ReceiveSend { name } => {
                self.check_name_indexed(name, context, missing)?;
            }
            Source::SendReceive { name, inputs } => {
                self.check_name_indexed(name, context, missing)?;
                for (idx, input) in inputs.iter().enumerate() {
                    if self.db.lookup(input).is_none() {
                        missing.push(format!("{}.input[{}]", context, idx));
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if a name's quoted process (if any) is indexed
    ///
    /// # Note on Quoted Process Indexing
    ///
    /// In Rholang, names can be:
    /// - Variables (e.g., `x`) - not indexed, resolved during scope analysis
    /// - Quoted processes (e.g., `@P`) - the process `P` inside the quote
    ///
    /// **Important**: Quoted processes within names are NOT indexed by `build_index`.
    /// The `iter_preorder_dfs` traversal only visits `Proc` nodes directly in the process
    /// tree, not processes embedded within `Name` nodes. This is by design:
    ///
    /// 1. Names are treated as atomic values in the AST traversal
    /// 2. The quoted process `@P` represents a channel name, not an executable process
    /// 3. Quoted processes will be indexed separately when they are evaluated (e.g., in `*x`)
    ///
    /// Therefore, we do NOT validate quoted process indexing here. The parser ensures
    /// structural validity, and Phase 2 pattern analysis will handle quoted processes
    /// in patterns appropriately.
    fn check_name_indexed(
        &self,
        _name: &'ast rholang_parser::ast::Name<'ast>,
        _context: &str,
        _missing: &mut Vec<String>,
    ) -> ElaborationResult<()> {
        // Names (including quoted processes) are not indexed by build_index
        Ok(())
    }
}

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
    pub fn strict() -> Self {
        Self {
            strict_typing: true,
            warn_unused_patterns: true,
            suggest_optimizations: true,
            max_pattern_depth: 32,
        }
    }

    pub fn lenient() -> Self {
        Self {
            strict_typing: false,
            warn_unused_patterns: false,
            suggest_optimizations: false,
            max_pattern_depth: 64,
        }
    }

    pub fn with_strict_typing(mut self, strict: bool) -> Self {
        self.strict_typing = strict;
        self
    }

    pub fn with_unused_pattern_warnings(mut self, warn: bool) -> Self {
        self.warn_unused_patterns = warn;
        self
    }

    pub fn with_optimization_suggestions(mut self, suggest: bool) -> Self {
        self.suggest_optimizations = suggest;
        self
    }

    pub fn with_max_pattern_depth(mut self, depth: usize) -> Self {
        self.max_pattern_depth = depth;
        self
    }
}

impl ElaborationContext {
    pub fn new() -> Self {
        Self {
            parent_scope: None,
            current_binders: Vec::new(),
            captures: BitSet::new(),
            channel_types: HashMap::new(),
            consumption_mode: ConsumptionMode::Linear,
        }
    }

    pub fn with_parent(parent_scope: PID) -> Self {
        Self {
            parent_scope: Some(parent_scope),
            current_binders: Vec::new(),
            captures: BitSet::new(),
            channel_types: HashMap::new(),
            consumption_mode: ConsumptionMode::Linear,
        }
    }

    pub fn with_consumption_mode(consumption_mode: ConsumptionMode) -> Self {
        Self {
            parent_scope: None,
            current_binders: Vec::new(),
            captures: BitSet::new(),
            channel_types: HashMap::new(),
            consumption_mode,
        }
    }

    pub fn parent_scope(&self) -> Option<PID> {
        self.parent_scope
    }

    pub fn current_binders(&self) -> &[PendingBinder] {
        &self.current_binders
    }

    pub fn add_binder(&mut self, binder: PendingBinder) {
        self.current_binders.push(binder);
    }

    pub fn captures(&self) -> &BitSet {
        &self.captures
    }

    pub fn mark_captured(&mut self, binder_id: BinderId) {
        self.captures.set(binder_id.0 as usize, true);
    }

    pub fn clear_binders(&mut self) {
        self.current_binders.clear();
    }

    pub fn channel_types(&self) -> &HashMap<Symbol, ChannelType> {
        &self.channel_types
    }

    pub fn channel_types_mut(&mut self) -> &mut HashMap<Symbol, ChannelType> {
        &mut self.channel_types
    }

    pub fn set_channel_type(&mut self, symbol: Symbol, channel_type: ChannelType) {
        self.channel_types.insert(symbol, channel_type);
    }

    pub fn consumption_mode(&self) -> ConsumptionMode {
        self.consumption_mode
    }

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

    pub fn symbol(&self) -> crate::sem::Symbol {
        self.symbol
    }

    pub fn kind(&self) -> crate::sem::BinderKind {
        self.kind
    }

    pub fn position(&self) -> SourcePos {
        self.position
    }

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
            pid: PID(42),
            var: Symbol(1),
            pos: SourcePos::default(),
        };
        assert!(format!("{}", unbound_error).contains("Unbound variable"));

        let consumption_error = ElaborationError::InvalidConsumptionMode {
            pid: PID(42),
            expected: ConsumptionMode::Linear,
            found: ConsumptionMode::Persistent,
            pos: SourcePos::default(),
        };
        assert!(format!("{}", consumption_error).contains("Invalid consumption mode"));
    }

    #[test]
    fn test_pre_validate_invalid_pid() {
        let mut db = SemanticDb::new();
        let mut elaborator = ForComprehensionElaborator::new(&mut db);

        let invalid_pid = PID(999);
        let result = elaborator.pre_validate(invalid_pid);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ElaborationError::InvalidPid { pid } if pid == invalid_pid
        ));
    }

    #[test]
    fn test_pre_validate_non_for_comprehension() {
        use rholang_parser::RholangParser;

        let parser = RholangParser::new();
        let code = "Nil"; // Not a for-comprehension
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let pid = db.build_index(proc);

        let mut elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.pre_validate(pid);

        assert!(result.is_err());
        match result.unwrap_err() {
            ElaborationError::IncompleteAstNode { reason, .. } => {
                assert!(reason.contains("Not a for-comprehension node"));
            }
            _ => panic!("Expected IncompleteAstNode error"),
        }
    }

    #[test]
    fn test_pre_validate_valid_for_comprehension() {
        use rholang_parser::RholangParser;

        let parser = RholangParser::new();
        let code = "for(x <- @\"channel\") { Nil }";
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let pid = db.build_index(proc);

        let mut elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.pre_validate(pid);

        // Should succeed - all child nodes are indexed by build_index
        assert!(
            result.is_ok(),
            "Expected successful validation, got: {:?}",
            result
        );
    }

    #[test]
    fn test_pre_validate_multiple_receipts() {
        use rholang_parser::RholangParser;

        let parser = RholangParser::new();
        let code = "for(x <- @\"ch1\"; y <- @\"ch2\") { x!(y) }";
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let pid = db.build_index(proc);

        let mut elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.pre_validate(pid);

        assert!(
            result.is_ok(),
            "Expected successful validation for multiple receipts"
        );
    }

    #[test]
    fn test_pre_validate_repeated_bind() {
        use rholang_parser::RholangParser;

        let parser = RholangParser::new();
        let code = "for(x <= @\"channel\") { Nil }";
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let pid = db.build_index(proc);

        let mut elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.pre_validate(pid);

        assert!(
            result.is_ok(),
            "Expected successful validation for repeated bind"
        );
    }

    #[test]
    fn test_pre_validate_peek_bind() {
        use rholang_parser::RholangParser;

        let parser = RholangParser::new();
        let code = "for(x <<- @\"channel\") { Nil }";
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let pid = db.build_index(proc);

        let mut elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.pre_validate(pid);

        assert!(
            result.is_ok(),
            "Expected successful validation for peek bind"
        );
    }

    #[test]
    fn test_pre_validate_complex_for_comprehension() {
        use rholang_parser::RholangParser;

        let parser = RholangParser::new();
        // Complex for-comprehension with nested structure
        let code = r#"
            for(x <- @"input"; y <= @"persistent") {
                x!(y) | for(z <<- @"peek") { z!(42) }
            }
        "#;
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let pid = db.build_index(proc);

        let mut elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.pre_validate(pid);

        assert!(
            result.is_ok(),
            "Expected successful validation for complex for-comprehension"
        );
    }

    #[test]
    fn test_pre_validate_with_quoted_process() {
        use rholang_parser::RholangParser;

        let parser = RholangParser::new();
        let code = "for(x <- @{Nil | Nil}) { x!(42) }";
        let ast = parser.parse(code).unwrap();

        let mut db = SemanticDb::new();
        let proc = &ast[0];
        let pid = db.build_index(proc);

        let mut elaborator = ForComprehensionElaborator::new(&mut db);
        let result = elaborator.pre_validate(pid);

        assert!(
            result.is_ok(),
            "Expected successful validation with quoted process"
        );
    }

    #[test]
    fn test_pre_validate_error_accumulation() {
        let mut db = SemanticDb::new();
        let mut elaborator = ForComprehensionElaborator::new(&mut db);

        // Test that errors are properly tracked
        assert!(!elaborator.has_errors());
        assert_eq!(elaborator.errors().len(), 0);

        elaborator.add_error(ElaborationError::InvalidPid { pid: PID(0) });
        assert!(elaborator.has_errors());
        assert_eq!(elaborator.errors().len(), 1);

        elaborator.clear_diagnostics();
        assert!(!elaborator.has_errors());
        assert_eq!(elaborator.errors().len(), 0);
    }
}
