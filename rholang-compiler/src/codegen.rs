//! Code generation context and compilation logic
//!
//! This module implements the core compilation logic that transforms
//! Rholang AST nodes into bytecode instructions

use anyhow::{anyhow, bail, Result};
use librho::sem::{BinderId, SemanticDb, SymbolOccurrence, PID};
use num_bigint::BigInt;
use num_rational::BigRational;
use rholang_bytecode::core::{instructions::Instruction, opcodes::Opcode};
use rholang_parser::ast::{
    AnnProc, BinaryExpOp, Bind, Collection, Name, Proc, Receipts, Source, Var,
};
use rholang_process::{Process, Value};
use std::collections::HashMap;

/// Compilation context for generating bytecode from Rholang AST
pub struct CodegenContext<'a> {
    db: &'a SemanticDb<'a>,

    /// The bytecode instruction stream being generated
    instructions: Vec<Instruction>,

    /// String pool for string literals (index -> string)
    strings: Vec<String>,

    /// Typed constant pool for numeric values (Float, BigInt, BigRat, FixedPoint,
    /// and integers outside i16 range). Indexed by PUSH_CONST operand.
    constants: Vec<Value>,

    /// Map from variable binder IDs to local slot indices
    locals: HashMap<BinderId, u16>,

    /// Next available local variable slot
    next_local: u16,

    /// Map from label IDs to instruction indices
    labels: HashMap<u32, usize>,

    /// Forward references (instruction index, label ID, opcode) for patching
    forward_refs: Vec<(usize, u32, Opcode)>,

    /// Next available label ID
    next_label: u32,

    /// Process index for source references
    proc_index: usize,
}

impl<'a> CodegenContext<'a> {
    /// Create a new code generation context.
    ///
    /// # Arguments
    /// * `db` - The semantic database for variable resolution
    /// * `proc_index` - Index of the process being compiled (for source references)
    pub fn new(db: &'a SemanticDb<'a>, proc_index: usize) -> Self {
        Self {
            db,
            instructions: Vec::new(),
            strings: Vec::new(),
            constants: Vec::new(),
            locals: HashMap::new(),
            next_local: 0,
            labels: HashMap::new(),
            forward_refs: Vec::new(),
            next_label: 0,
            proc_index,
        }
    }

    /// Compile a process node into bytecode instructions
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - An unsupported process variant is encountered
    /// - Binary operator mapping fails
    /// - Integer literal is out of range for MVP
    pub fn compile_proc(&mut self, proc: &AnnProc<'a>) -> Result<()> {
        match proc.proc {
            Proc::Nil => {
                self.emit(Instruction::nullary(Opcode::PUSH_NIL));
            }

            Proc::Unit => {
                // Unit is the empty tuple ()
                self.emit(Instruction::unary(Opcode::CREATE_TUPLE, 0));
            }

            Proc::BoolLiteral(b) => {
                self.emit(Instruction::unary(Opcode::PUSH_BOOL, *b as u16));
            }

            Proc::LongLiteral(n) => {
                self.emit_int(*n)?;
            }

            Proc::SignedIntLiteral { value, bits } => {
                self.compile_signed_int(value, *bits)?;
            }

            Proc::UnsignedIntLiteral { value, bits } => {
                self.compile_unsigned_int(value, *bits)?;
            }

            Proc::BigIntLiteral(s) => {
                self.compile_bigint(s)?;
            }

            Proc::BigRatLiteral(s) => {
                self.compile_bigrat(s)?;
            }

            Proc::FloatLiteral { value, bits } => {
                self.compile_float(value, *bits)?;
            }

            Proc::FixedPointLiteral { value, scale } => {
                self.compile_fixed_point(value, *scale)?;
            }

            Proc::StringLiteral(s) => {
                let idx = self.add_string(s);
                self.emit(Instruction::unary(Opcode::PUSH_STR, idx));
            }

            Proc::BinaryExp { op, left, right } => {
                // Compile operands first (stack-based evaluation)
                self.compile_proc(left)?;
                self.compile_proc(right)?;
                self.emit_binop(*op)?;
            }

            Proc::ProcVar(var) => {
                // Variable used in process position - may need implicit EVAL
                // SAFETY: We cast proc to the correct lifetime since it comes from the AST
                let pid = match self.db.lookup(unsafe { &*(proc as *const AnnProc<'a>) }) {
                    Some(pid) => pid,
                    None => bail!("ProcVar at {} not indexed", proc.span.start),
                };
                self.compile_var(var, pid, true)?;
            }

            Proc::IfThenElse {
                condition,
                if_true,
                if_false,
            } => {
                self.compile_if_then_else(condition, if_true, if_false.as_ref())?;
            }

            Proc::Collection(coll) => {
                self.compile_collection(coll)?;
            }

            Proc::New {
                decls: _,
                proc: body,
            } => {
                // Look up PID for this new declaration
                // SAFETY: We cast proc to the correct lifetime since it comes from the AST
                let pid = match self.db.lookup(unsafe { &*(proc as *const AnnProc<'a>) }) {
                    Some(pid) => pid,
                    None => bail!(
                        "New declaration at {} not indexed in semantic database",
                        proc.span.start
                    ),
                };
                self.compile_new(pid, body)?;
            }

            Proc::Send {
                channel,
                inputs,
                send_type: _,
            } => {
                // Look up PID for the send operation
                // SAFETY: We cast proc to the correct lifetime since it comes from the AST
                let pid = match self.db.lookup(unsafe { &*(proc as *const AnnProc<'a>) }) {
                    Some(pid) => pid,
                    None => bail!("Send at {} not indexed", proc.span.start),
                };
                self.compile_send(pid, channel, inputs)?;
            }

            Proc::ForComprehension {
                receipts,
                proc: body,
            } => {
                // Look up PID for the for-comprehension to resolve free variables
                // SAFETY: We cast proc to the correct lifetime since it comes from the AST
                let pid = match self.db.lookup(unsafe { &*(proc as *const AnnProc<'a>) }) {
                    Some(pid) => pid,
                    None => bail!("For-comprehension at {} not indexed", proc.span.start),
                };
                self.compile_for_comprehension(pid, receipts, body)?;
            }

            Proc::Par { left, right } => {
                self.compile_par(left, right)?;
            }

            _ => bail!(
                "Unsupported process variant in MVP: {:?}",
                std::mem::discriminant(proc.proc)
            ),
        }

        Ok(())
    }

    fn emit(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }

    /// Emit an integer literal instruction.
    ///
    /// Values in i16 range use PUSH_INT (inline immediate).
    /// Values outside i16 range use PUSH_CONST (constant pool).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn emit_int(&mut self, n: i64) -> Result<()> {
        if n >= i16::MIN as i64 && n <= i16::MAX as i64 {
            let bits = (n as i16) as u16;
            self.emit(Instruction::unary(Opcode::PUSH_INT, bits));
        } else {
            let idx = self.add_constant(Value::Int(n));
            self.emit(Instruction::unary(Opcode::PUSH_CONST, idx));
        }
        Ok(())
    }

    /// Add a typed constant to the constant pool and return its index.
    #[allow(clippy::cast_possible_truncation)]
    fn add_constant(&mut self, val: Value) -> u16 {
        // Deduplicate: reuse existing index if an equal constant already exists
        if let Some(idx) = self.constants.iter().position(|c| c == &val) {
            return idx as u16;
        }
        let idx = self.constants.len();
        assert!(
            idx <= u16::MAX as usize,
            "Constant pool overflow (max {} constants)",
            u16::MAX
        );
        self.constants.push(val);
        idx as u16
    }

    /// Compile a signed integer literal (e.g., `-52i64`, `127i8`).
    fn compile_signed_int(&mut self, value: &str, bits: u32) -> Result<()> {
        let n: i128 = value
            .parse()
            .map_err(|e| anyhow!("Invalid signed integer literal '{}': {}", value, e))?;

        let max = 1i128 << (bits - 1);
        let min = -max;
        if n < min || n >= max {
            bail!(
                "Signed integer literal {} out of range for i{} ({} to {})",
                value,
                bits,
                min,
                max - 1
            );
        }

        if bits <= 64 {
            self.emit_int(n as i64)
        } else {
            let big = BigInt::from(n);
            let idx = self.add_constant(Value::BigInt(big));
            self.emit(Instruction::unary(Opcode::PUSH_CONST, idx));
            Ok(())
        }
    }

    /// Compile an unsigned integer literal (e.g., `65535u16`, `255u8`).
    fn compile_unsigned_int(&mut self, value: &str, bits: u32) -> Result<()> {
        let n: u128 = value
            .parse()
            .map_err(|e| anyhow!("Invalid unsigned integer literal '{}': {}", value, e))?;

        let max = if bits >= 128 {
            u128::MAX
        } else {
            (1u128 << bits) - 1
        };
        if n > max {
            bail!(
                "Unsigned integer literal {} out of range for u{} (0 to {})",
                value,
                bits,
                max
            );
        }

        if n <= i64::MAX as u128 {
            self.emit_int(n as i64)
        } else {
            let big = BigInt::from(n);
            Value::check_bigint_size(&big)
                .map_err(|e| anyhow!("Unsigned integer literal {}{}: {}", value, bits, e))?;
            let idx = self.add_constant(Value::BigInt(big));
            self.emit(Instruction::unary(Opcode::PUSH_CONST, idx));
            Ok(())
        }
    }

    /// Compile a BigInt literal (e.g., `10n`, `-1n`).
    fn compile_bigint(&mut self, s: &str) -> Result<()> {
        let n: BigInt = s
            .parse()
            .map_err(|e| anyhow!("Invalid bigint literal '{}n': {}", s, e))?;
        Value::check_bigint_size(&n)
            .map_err(|e| anyhow!("BigInt literal {}n: {}", s, e))?;
        let idx = self.add_constant(Value::BigInt(n));
        self.emit(Instruction::unary(Opcode::PUSH_CONST, idx));
        Ok(())
    }

    /// Compile a BigRat literal (e.g., `3r`, `-1r`).
    fn compile_bigrat(&mut self, s: &str) -> Result<()> {
        let n: BigInt = s
            .parse()
            .map_err(|e| anyhow!("Invalid bigrat literal '{}r': {}", s, e))?;
        let r = BigRational::from(n);
        let idx = self.add_constant(Value::BigRat(r));
        self.emit(Instruction::unary(Opcode::PUSH_CONST, idx));
        Ok(())
    }

    /// Compile a float literal (e.g., `-1.234e5f32`, `3.14f64`).
    fn compile_float(&mut self, value: &str, bits: u16) -> Result<()> {
        match bits {
            32 | 64 => {}
            other => bail!(
                "Float width f{} not supported at runtime (only f32 and f64)",
                other
            ),
        }

        let f: f64 = value
            .parse()
            .map_err(|e| anyhow!("Invalid float literal '{}f{}': {}", value, bits, e))?;

        if bits == 32 {
            let f32_val = f as f32;
            if f32_val.is_infinite() && !f.is_infinite() {
                bail!(
                    "Float literal '{}' overflows f32 (would become Inf)",
                    value
                );
            }
        }

        let idx = self.add_constant(Value::Float(f));
        self.emit(Instruction::unary(Opcode::PUSH_CONST, idx));
        Ok(())
    }

    /// Compile a fixed-point literal (e.g., `3.3p1`, `0.25p2`, `100p0`).
    fn compile_fixed_point(&mut self, value: &str, scale: u32) -> Result<()> {
        let unscaled = parse_fixed_point_unscaled(value, scale)?;
        Value::check_bigint_size(&unscaled).map_err(|e| {
            anyhow!("FixedPoint literal '{}p{}': {}", value, scale, e)
        })?;
        let idx = self.add_constant(Value::FixedPoint { unscaled, scale });
        self.emit(Instruction::unary(Opcode::PUSH_CONST, idx));
        Ok(())
    }

    /// Add a string to the string pool and return its index
    ///
    /// If the string pool exceeds u16::MAX entries, compilation will fail
    #[allow(clippy::cast_possible_truncation)]
    fn add_string(&mut self, s: &str) -> u16 {
        // Check if string already exists in pool (deduplication)
        for (idx, existing) in self.strings.iter().enumerate() {
            if existing == s {
                return idx as u16;
            }
        }

        // Add new string
        let idx = self.strings.len();
        assert!(
            idx <= u16::MAX as usize,
            "String pool overflow (max {} strings)",
            u16::MAX
        );

        self.strings.push(s.to_string());

        idx as u16
    }

    /// Emit a binary operator instruction
    ///
    /// # Errors
    ///
    /// Returns an error if the operator is not supported
    fn emit_binop(&mut self, op: BinaryExpOp) -> Result<()> {
        let opcode = match op {
            // Arithmetic operators
            BinaryExpOp::Add => Opcode::ADD,
            BinaryExpOp::Sub => Opcode::SUB,
            BinaryExpOp::Mult => Opcode::MUL,
            BinaryExpOp::Div => Opcode::DIV,
            BinaryExpOp::Mod => Opcode::MOD,

            // Comparison operators
            BinaryExpOp::Eq => Opcode::CMP_EQ,
            BinaryExpOp::Neq => Opcode::CMP_NEQ,
            BinaryExpOp::Lt => Opcode::CMP_LT,
            BinaryExpOp::Lte => Opcode::CMP_LTE,
            BinaryExpOp::Gt => Opcode::CMP_GT,
            BinaryExpOp::Gte => Opcode::CMP_GTE,

            // Logical operators
            BinaryExpOp::And => Opcode::AND,
            BinaryExpOp::Or => Opcode::OR,

            // Unsupported
            _ => bail!("Unsupported binary operator: {:?}", op),
        };

        self.emit(Instruction::nullary(opcode));
        Ok(())
    }

    /// Compile a variable reference
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The variable is not found in the semantic database
    /// - The variable is not allocated to a local slot
    fn compile_var(&mut self, var: &Var<'_>, pid: PID, as_process: bool) -> Result<()> {
        match var {
            Var::Wildcard => {
                // Wildcard evaluates to Nil
                self.emit(Instruction::nullary(Opcode::PUSH_NIL));
            }
            Var::Id(id) => {
                let symbol = self.db.intern(id.name);
                let occ = SymbolOccurrence {
                    symbol,
                    position: id.pos,
                };

                // Get the binder for this variable occurrence
                match self.db.binder_of(occ) {
                    Some(binding) => {
                        // Resolve the binding (handles both Bound and Free)
                        let binder_id = self.db.resolve_var_binding(pid, binding);

                        // Look up the local slot for this binder
                        // For free variables (e.g., from for-comprehension patterns),
                        // we may have already allocated a local in an enclosing scope
                        let local_idx = self.locals.get(&binder_id).ok_or_else(|| {
                            anyhow!(
                                "Variable '{}' at {} is not allocated to a local slot",
                                id.name,
                                id.pos
                            )
                        })?;

                        self.emit(Instruction::unary(Opcode::LOAD_LOCAL, *local_idx));

                        // Auto-emit EVAL for implicit evaluation:
                        // When a name binder is used in process position, we need to
                        // unquote it to get the underlying process value
                        if as_process && self.db.is_name(binder_id) {
                            self.emit(Instruction::nullary(Opcode::EVAL));
                        }
                    }
                    None => {
                        bail!("Unbound variable '{}' at {}", id.name, id.pos);
                    }
                }
            }
        }

        Ok(())
    }

    /// Compile an if-then-else expression
    ///
    /// # Errors
    ///
    /// Returns an error if compilation of any branch fails.
    fn compile_if_then_else(
        &mut self,
        condition: &AnnProc<'a>,
        if_true: &AnnProc<'a>,
        if_false: Option<&AnnProc<'a>>,
    ) -> Result<()> {
        self.compile_proc(condition)?;

        // Create a label for the else branch (or end if no else)
        let label_else = self.new_label();

        // Emit BRANCH_FALSE with placeholder
        let branch_idx = self.instructions.len();
        self.emit(Instruction::nullary(Opcode::NOP)); // Placeholder
        self.forward_refs
            .push((branch_idx, label_else, Opcode::BRANCH_FALSE));

        // Compile the then branch
        self.compile_proc(if_true)?;

        if let Some(else_proc) = if_false {
            // If there's an else branch, we need a jump over it
            let label_end = self.new_label();

            // Jump to end after then branch
            let jump_idx = self.instructions.len();
            self.emit(Instruction::nullary(Opcode::NOP)); // Placeholder
            self.forward_refs.push((jump_idx, label_end, Opcode::JUMP));

            self.define_label(label_else);

            self.compile_proc(else_proc)?;

            self.define_label(label_end);
        } else {
            self.define_label(label_else);
        }

        Ok(())
    }

    /// Compile a collection (list or tuple)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A collection with remainder is encountered (not supported in MVP)
    /// - Maps or Sets are encountered (not supported in MVP)
    /// - Element compilation fails
    /// - Element count exceeds u16::MAX
    fn compile_collection(&mut self, coll: &Collection<'a>) -> Result<()> {
        match coll {
            Collection::List {
                elements,
                remainder,
            } => {
                if remainder.is_some() {
                    bail!("List remainder not supported in MVP");
                }

                for elem in elements {
                    self.compile_proc(elem)?;
                }

                let count = elements.len();
                if count > u16::MAX as usize {
                    bail!("List has too many elements (max {})", u16::MAX);
                }

                self.emit(Instruction::unary(Opcode::CREATE_LIST, count as u16));
            }

            Collection::Tuple(elements) => {
                for elem in elements {
                    self.compile_proc(elem)?;
                }

                let count = elements.len();
                if count > u16::MAX as usize {
                    bail!("Tuple has too many elements (max {})", u16::MAX);
                }

                self.emit(Instruction::unary(Opcode::CREATE_TUPLE, count as u16));
            }

            Collection::Set { .. } => {
                bail!("Sets not supported in MVP");
            }

            Collection::Map { .. } => {
                bail!("Maps not supported in MVP");
            }

            Collection::PathMap { .. } => {
                bail!("PathMaps not supported in MVP");
            }
        }

        Ok(())
    }

    /// Compile a new channel declaration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The new declaration has no scope information
    /// - Local allocation fails
    /// - Body compilation fails
    fn compile_new(&mut self, new_pid: PID, body: &AnnProc<'a>) -> Result<()> {
        let scope = self
            .db
            .get_scope(new_pid)
            .ok_or_else(|| anyhow!("New declaration (PID {}) has no scope information", new_pid))?;

        // Iterate over all binders introduced by this new declaration
        // Each binder corresponds to a channel name in the declaration
        for (binder_id, _binder) in self.db.binders_full(scope) {
            // Create a fresh channel name
            // For MVP, we use a default kind (3 = persistent concurrent storage)
            const DEFAULT_NAME_KIND: u16 = 3;
            self.emit(Instruction::unary(Opcode::NAME_CREATE, DEFAULT_NAME_KIND));

            // Allocate a local slot on the VM stack
            self.emit(Instruction::nullary(Opcode::ALLOC_LOCAL));

            // Track this binder's local slot index for later references
            let slot = self.alloc_local(binder_id)?;
            self.emit(Instruction::unary(Opcode::STORE_LOCAL, slot));
        }

        // Compile the body with channels in scope
        self.compile_proc(body)?;

        Ok(())
    }

    /// Compile a send operation
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Channel compilation fails
    /// - Input compilation fails
    /// - Input count exceeds u16::MAX
    #[allow(clippy::cast_possible_truncation)]
    fn compile_send(&mut self, pid: PID, channel: &Name<'a>, inputs: &[AnnProc<'a>]) -> Result<()> {
        self.compile_name(channel, pid)?;

        for input in inputs {
            self.compile_proc(input)?;
        }

        // Package inputs into a list (only if multiple inputs)
        // For MVP, single values are sent directly without wrapping
        let count = inputs.len();
        if count > u16::MAX as usize {
            bail!("Too many send inputs (max {})", u16::MAX);
        }

        if count != 1 {
            self.emit(Instruction::unary(Opcode::CREATE_LIST, count as u16));
        }

        // Send the message
        const DEFAULT_SEND_KIND: u8 = 3;
        self.emit(Instruction::binary(
            Opcode::TELL,
            DEFAULT_SEND_KIND,
            0, // reserved
        ));

        Ok(())
    }

    /// Compile a for-comprehension (receive operation)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Non-linear receives are encountered (repeated/peek not supported)
    /// - Complex sources are encountered (only Simple sources supported)
    /// - Channel compilation fails
    /// - Body compilation fails
    fn compile_for_comprehension(
        &mut self,
        pid: PID,
        receipts: &Receipts<'a>,
        body: &AnnProc<'a>,
    ) -> Result<()> {
        for receipt in receipts {
            // For MVP, we only support single binds per receipt
            for bind in receipt {
                match bind {
                    Bind::Linear { lhs, rhs } => {
                        match rhs {
                            Source::Simple { name } => {
                                self.compile_name(name, pid)?;
                            }
                            Source::ReceiveSend { .. } | Source::SendReceive { .. } => {
                                bail!("Complex sources not supported in MVP");
                            }
                        }

                        // Emit ASK to receive from the channel
                        const DEFAULT_RECEIVE_KIND: u8 = 3;
                        self.emit(Instruction::binary(
                            Opcode::ASK,
                            DEFAULT_RECEIVE_KIND,
                            0, // reserved
                        ));

                        // Bind received values to variables
                        // For MVP, we expect the result to be a list that we unpack
                        for name in &lhs.names {
                            match name {
                                Name::NameVar(Var::Id(id)) => {
                                    // Resolve the binder for this variable
                                    let symbol = self.db.intern(id.name);
                                    let occ = SymbolOccurrence {
                                        symbol,
                                        position: id.pos,
                                    };

                                    match self.db.binder_of(occ) {
                                        Some(binding) => {
                                            // Resolve the binding (handles both Bound and Free)
                                            let binder_id =
                                                self.db.resolve_var_binding(pid, binding);

                                            self.emit(Instruction::nullary(Opcode::ALLOC_LOCAL));

                                            let slot = self.alloc_local(binder_id)?;
                                            self.emit(Instruction::unary(
                                                Opcode::STORE_LOCAL,
                                                slot,
                                            ));
                                        }
                                        None => {
                                            bail!("Unbound variable '{}' at {}", id.name, id.pos);
                                        }
                                    }
                                }
                                Name::NameVar(Var::Wildcard) => {
                                    // Wildcard binding - pop the value
                                    self.emit(Instruction::nullary(Opcode::POP));
                                }
                                Name::Quote(_) => {
                                    bail!("Quote patterns not supported in MVP");
                                }
                            }
                        }
                    }
                    Bind::Repeated { .. } => {
                        bail!("Repeated receives not supported in MVP");
                    }
                    Bind::Peek { .. } => {
                        bail!("Peek receives not supported in MVP");
                    }
                }
            }
        }

        // Compile the continuation body
        self.compile_proc(body)?;

        Ok(())
    }

    /// Compile a parallel composition
    /// For MVP, parallel composition is executed sequentially
    ///
    /// # Errors
    ///
    /// Returns an error if compilation of either side fails
    fn compile_par(&mut self, left: &AnnProc<'a>, right: &AnnProc<'a>) -> Result<()> {
        // Compile left side
        self.compile_proc(left)?;

        // Discard the result of left side
        self.emit(Instruction::nullary(Opcode::POP));

        // Compile right side (its result stays on stack)
        self.compile_proc(right)?;

        Ok(())
    }

    /// Compile a channel name
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Name is a Quote (not supported in MVP)
    /// - Variable compilation fails
    fn compile_name(&mut self, name: &Name<'a>, pid: PID) -> Result<()> {
        match name {
            Name::NameVar(var) => self.compile_var(var, pid, false),
            Name::Quote(_) => bail!("Quote not supported in MVP"),
        }
    }

    /// Allocate a local slot for a variable binder
    ///
    /// Returns the allocated slot index
    ///
    /// # Errors
    ///
    /// Returns an error if we've exceeded the maximum number of local variables (u16::MAX)
    fn alloc_local(&mut self, binder_id: BinderId) -> Result<u16> {
        if self.next_local == u16::MAX {
            bail!("Too many local variables (maximum {})", u16::MAX);
        }

        let slot = self.next_local;
        self.locals.insert(binder_id, slot);
        self.next_local += 1;

        Ok(slot)
    }

    fn new_label(&mut self) -> u32 {
        let label = self.next_label;
        self.next_label += 1;
        label
    }

    fn define_label(&mut self, label: u32) {
        let pos = self.instructions.len();
        self.labels.insert(label, pos);
    }

    /// Resolve all forward references to labels
    ///
    /// This patches all placeholder instructions with the correct jump targets
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A label is referenced but never defined
    /// - A jump target is out of range for u16
    #[allow(clippy::cast_possible_truncation)]
    fn resolve_labels(&mut self) -> Result<()> {
        for (inst_idx, label_id, opcode) in &self.forward_refs {
            // Look up the target instruction index
            let target = self.labels.get(label_id).ok_or_else(|| {
                anyhow!(
                    "Undefined label {} referenced at instruction {}",
                    label_id,
                    inst_idx
                )
            })?;

            // Check that the target fits in u16
            if *target > u16::MAX as usize {
                bail!("Jump target {} is too large (max {})", target, u16::MAX);
            }

            // Patch the instruction with the correct opcode and target
            self.instructions[*inst_idx] = Instruction::unary(*opcode, *target as u16);
        }

        Ok(())
    }

    /// Finalize compilation and produce the executable Process
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The instruction stream is empty
    /// - Labels are unresolved
    pub fn finalize(mut self) -> Result<Process> {
        // Ensure we have at least one instruction
        if self.instructions.is_empty() {
            return Err(anyhow!("Cannot finalize empty instruction stream"));
        }

        // Resolve all forward references
        self.resolve_labels()?;

        // Add HALT instruction at the end
        self.emit(Instruction::nullary(Opcode::HALT));

        // Create source reference
        let source_ref = format!("proc_{}", self.proc_index);

        // Build the process with string pool
        let mut process = Process::new(self.instructions, source_ref);

        // Convert string pool to Value::Str
        process.names = self.strings.into_iter().map(Value::Str).collect();

        // Set the constant pool
        process.constants = self.constants;

        Ok(process)
    }
}

/// Parse a fixed-point literal value string into an unscaled BigInt.
///
/// For example, `"3.3"` with scale=1 → unscaled=33.
/// `"100"` with scale=0 → unscaled=100.
/// `"0.25"` with scale=2 → unscaled=25.
fn parse_fixed_point_unscaled(value: &str, scale: u32) -> Result<BigInt> {
    let scale = scale as usize;

    if let Some(dot_pos) = value.find('.') {
        let integer_part = &value[..dot_pos];
        let frac_part = &value[dot_pos + 1..];

        if frac_part.len() > scale {
            bail!(
                "Fixed-point literal '{}p{}' has {} fractional digits but scale is {}",
                value,
                scale,
                frac_part.len(),
                scale
            );
        }

        // Pad fractional part to match scale
        let padded_frac = format!("{:0<width$}", frac_part, width = scale);
        let combined = format!("{}{}", integer_part, padded_frac);
        combined
            .parse::<BigInt>()
            .map_err(|e| anyhow!("Invalid fixed-point literal '{}p{}': {}", value, scale, e))
    } else {
        // No decimal point — multiply by 10^scale
        let base: BigInt = value
            .parse()
            .map_err(|e| anyhow!("Invalid fixed-point literal '{}p{}': {}", value, scale, e))?;
        let multiplier = num_traits::pow::pow(BigInt::from(10), scale);
        Ok(base * multiplier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use librho::sem::SemanticDb;
    use rholang_parser::{ast::Proc, SourceSpan};

    /// Helper to create an annotated proc for testing
    fn ann_proc<'a>(proc: &'a Proc<'a>) -> AnnProc<'a> {
        AnnProc {
            proc,
            span: SourceSpan::default(),
        }
    }

    #[test]
    fn test_compile_nil() {
        let db = SemanticDb::new();
        let proc = Proc::Nil;
        let mut ctx = CodegenContext::new(&db, 0);

        assert!(ctx.compile_proc(&ann_proc(&proc)).is_ok());
        assert_eq!(ctx.instructions.len(), 1);
        assert_eq!(ctx.instructions[0].opcode().unwrap(), Opcode::PUSH_NIL);
    }

    #[test]
    fn test_compile_bool_true() {
        let db = SemanticDb::new();
        let proc = Proc::BoolLiteral(true);
        let mut ctx = CodegenContext::new(&db, 0);

        assert!(ctx.compile_proc(&ann_proc(&proc)).is_ok());
        assert_eq!(ctx.instructions.len(), 1);
        assert_eq!(ctx.instructions[0].opcode().unwrap(), Opcode::PUSH_BOOL);
        assert_eq!(ctx.instructions[0].op16(), 1);
    }

    #[test]
    fn test_compile_bool_false() {
        let db = SemanticDb::new();
        let proc = Proc::BoolLiteral(false);
        let mut ctx = CodegenContext::new(&db, 0);

        assert!(ctx.compile_proc(&ann_proc(&proc)).is_ok());
        assert_eq!(ctx.instructions.len(), 1);
        assert_eq!(ctx.instructions[0].opcode().unwrap(), Opcode::PUSH_BOOL);
        assert_eq!(ctx.instructions[0].op16(), 0);
    }

    #[test]
    fn test_compile_int_positive() {
        let db = SemanticDb::new();
        let proc = Proc::LongLiteral(42);
        let mut ctx = CodegenContext::new(&db, 0);

        assert!(ctx.compile_proc(&ann_proc(&proc)).is_ok());
        assert_eq!(ctx.instructions.len(), 1);
        assert_eq!(ctx.instructions[0].opcode().unwrap(), Opcode::PUSH_INT);
    }

    #[test]
    fn test_compile_int_negative() {
        let db = SemanticDb::new();
        let proc = Proc::LongLiteral(-100);
        let mut ctx = CodegenContext::new(&db, 0);

        assert!(ctx.compile_proc(&ann_proc(&proc)).is_ok());
        assert_eq!(ctx.instructions.len(), 1);
        assert_eq!(ctx.instructions[0].opcode().unwrap(), Opcode::PUSH_INT);
    }

    #[test]
    fn test_compile_int_large_uses_constant_pool() {
        let db = SemanticDb::new();
        let proc = Proc::LongLiteral(100_000);
        let mut ctx = CodegenContext::new(&db, 0);

        let result = ctx.compile_proc(&ann_proc(&proc));
        assert!(result.is_ok(), "large int should compile via constant pool");
        // Verify a PUSH_CONST instruction was emitted
        assert!(
            ctx.instructions
                .iter()
                .any(|i| i.opcode().map(|o| o == Opcode::PUSH_CONST).unwrap_or(false)),
            "should emit PUSH_CONST for out-of-i16-range integer"
        );
        assert_eq!(ctx.constants.len(), 1);
    }

    #[test]
    fn test_compile_string() {
        let db = SemanticDb::new();
        let s = "hello";
        let proc = Proc::StringLiteral(s);
        let mut ctx = CodegenContext::new(&db, 0);

        assert!(ctx.compile_proc(&ann_proc(&proc)).is_ok());
        assert_eq!(ctx.instructions.len(), 1);
        assert_eq!(ctx.instructions[0].opcode().unwrap(), Opcode::PUSH_STR);
        assert_eq!(ctx.strings.len(), 1);
        assert_eq!(ctx.strings[0], "hello");
    }

    #[test]
    fn test_string_deduplication() {
        let db = SemanticDb::new();
        let mut ctx = CodegenContext::new(&db, 0);

        let idx1 = ctx.add_string("test");
        let idx2 = ctx.add_string("test");
        let idx3 = ctx.add_string("different");

        assert_eq!(idx1, idx2); // Same string, same index
        assert_ne!(idx1, idx3); // Different string, different index
        assert_eq!(ctx.strings.len(), 2); // Only 2 unique strings
    }

    #[test]
    fn test_finalize_adds_halt() {
        let db = SemanticDb::new();
        let proc = Proc::Nil;
        let mut ctx = CodegenContext::new(&db, 0);

        ctx.compile_proc(&ann_proc(&proc)).unwrap();
        let initial_len = ctx.instructions.len();

        let process = ctx.finalize().unwrap();

        // Should have added HALT instruction
        assert_eq!(process.code.len(), initial_len + 1);
        assert_eq!(process.code.last().unwrap().opcode().unwrap(), Opcode::HALT);
    }

    #[test]
    fn test_finalize_empty_fails() {
        let db = SemanticDb::new();
        let ctx = CodegenContext::new(&db, 0);

        let result = ctx.finalize();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("empty instruction stream"));
    }
}
