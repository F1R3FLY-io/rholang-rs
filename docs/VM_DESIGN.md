# Rholang VM Design (current implementation)

This document describes the current design and implementation status of the new rholang-vm crate. It reflects the Process-based VM rebuilt on top of rholang-bytecode, the supported instruction subset, execution model, and the intended evolution path.

The design intentionally favors clarity and composability over completeness at this stage. It is meant to guide contributors, inform integrators, and serve as a reference for tests and examples.


## Goals and Scope

- Provide a minimal, well-factored Rholang virtual machine that executes bytecode produced and described by the rholang-bytecode crate.
- Adopt a Process-centric execution model with a simple value stack and locals.
- Support a subset of core opcodes for arithmetic, collections, simple pattern placeholders, locals, and a minimal in-VM RSpace approximation sufficient for tests and examples.
- Keep tight alignment with rholang-bytecode’s Instruction/Opcode and evolve by incrementally implementing more semantics and opcodes.

Out of scope for the current milestone:
- Full control-flow (labels/jumps) semantics and a full evaluator for process expressions.
- Real RSpace integration with storage backends and matching engine.
- String literal infrastructure (ExtendedInstruction) and complex data pools.
- Continuations and full contract installation semantics beyond a minimal placeholder.


## Relationship to rholang-bytecode

The rholang-vm relies on the rholang-bytecode crate for:
- Opcode definitions (core::opcodes::Opcode).
- 32-bit fixed-width instruction format (core::instructions::Instruction) with compact immediates (op1/op2/op16).
- ExtendedInstruction and InstructionData definitions (when larger operands are required in the future).

The VM does not define its own instruction enum. All tests and examples construct programs with rholang_bytecode::core::instructions::Instruction using the provided helpers:
- Instruction::nullary(opcode)
- Instruction::unary(opcode, u16)
- Instruction::binary(opcode, u8, u8)

This ensures the VM and tooling stay aligned with the bytecode specification.


## Architecture Overview

The crate is split into small focused modules:
- value.rs — Value enum representing runtime values on the stack and in locals.
- process.rs — Process structure: code, source_ref, locals, names. The VM executes a Process.
- vm.rs — VM structure and the main execute loop operating on a Process.
- opcode_exec.rs — The instruction dispatcher. One step() per instruction with concrete semantics.

Re-exports are provided via rholang_vm::api to simplify imports in tests/examples:
- api::{Instruction, Opcode} from rholang-bytecode.
- api::{VM, Process, Value} from this crate.

### Core Data Structures

- Value: runtime data used by the VM
  - Int(i64), Bool(bool), Str(String), Name(String)
  - List(Vec<Value>), Tuple(Vec<Value>), Map(Vec<(Value, Value)>)
  - Nil

- Process:
  - code: Vec<Instruction> — bytecode to execute
  - source_ref: String — optional provenance/debug tag
  - locals: Vec<Value> — local variable slots
  - names: Vec<Value> — placeholder vector for future name-related bookkeeping

- VM:
  - stack: Vec<Value> — value stack
  - rspace: HashMap<(u16 kind, String channel), Vec<Value>> — minimal in-VM queue per channel
  - cont_table: HashMap<u32, Value> — minimal continuation table for future use (tests may leverage)
  - next_cont_id: u32 — monotonic continuation id counter
  - next_name_id: u64 — monotonic fresh-name counter (for NAME_CREATE)


## Execution Model

- Single public entry: VM::execute(&mut self, process: &mut Process) -> Result<Value>.
- The VM clears its stack at the start of each execution for test isolation.
- A simple PC-based loop fetches instructions from process.code and dispatches them to opcode_exec::step().
- step() returns a boolean indicating HALT. The VM stops on HALT or when code ends.
- The result is the top of the stack (Value) at termination or Value::Nil if the stack is empty.

Error handling:
- Type errors or stack underflow emit anyhow::Error with descriptive messages (e.g., "DIV requires Ints", "division by zero").
- Out-of-bounds locals accesses also error (e.g., LOAD_LOCAL/STORE_LOCAL).

Determinism and isolation:
- The stack is reset per execution call.
- Fresh names are generated via VM.next_name_id to avoid test cross-contamination.
- rspace store is VM-owned and persists across executions unless reset via VM::reset_rspace() (helper for tests).


## Implemented Opcode Semantics (subset)

Arithmetic (Int only unless specified):
- ADD: Int+Int -> Int; Str+Str -> Str; List+List -> List concat; else error.
- SUB: Int-Int -> Int; else error.
- MUL: Int*Int -> Int; else error.
- DIV: Int/Int -> Int; error if divisor is 0 or non-Ints.
- MOD: Int%Int -> Int; error if divisor is 0 or non-Ints.
- NEG: -Int -> Int; else error.

Stack/Push:
- PUSH_INT (unary; i16 immediate sign-extended).
- PUSH_BOOL (nullary+op1 used as boolean bit).
- POP (discard top).

Collections:
- CREATE_LIST n: pop n values (rightmost first), reverse, push List.
- CREATE_TUPLE n: pop n values (rightmost first), reverse, push Tuple.
- CREATE_MAP n: pop n pairs (value then key), reverse, push Map of (key, value).
- CONCAT: Str+Str or List+List — push result; else error.
- DIFF: List-List difference with multiset semantics (each RHS occurrence removes one LHS occurrence), order-preserving.

Patterns (placeholders):
- PATTERN: push Nil (placeholder compiled pattern).
- MATCH_TEST: pop value and pattern placeholder, push Bool(true) as a stand-in.
- EXTRACT_BINDINGS: push empty Map.

Locals:
- ALLOC_LOCAL: push Nil into process.locals (new slot).
- LOAD_LOCAL idx: push a clone of locals[idx]; error if OOB.
- STORE_LOCAL idx: pop value, assign to locals[idx]; error if OOB or underflow.

Minimal RSpace (in-VM queue model):
- NAME_CREATE kind(u16 immediate): generate fresh channel name as "@{kind}:{id}", push Value::Name.
- TELL kind: pop data then channel Name; append data to queue keyed by (kind, channel). Push Bool(true).
- ASK kind: pop channel Name; pop head of queue (if any) and push it, else Nil.
- PEEK kind: pop channel Name; push clone of head of queue (if any), else Nil.

Notes:
- The kind code (u16 immediate) is a test-facing convention to segregate logical RSpace types (e.g., sequential vs concurrent, in-memory vs store). It is not a stable ABI.
- NAME_CREATE name freshness is per-VM via a monotonic counter (next_name_id), not cryptographic.


## Not Implemented (yet) / Roadmap

Near-term priorities:
- Control flow: JUMP/BRANCH_TRUE/BRANCH_FALSE/RETURN; proper label resolution and PC management (requires either a prepass or a label table associated with the Process).
- String support via ExtendedInstruction/InstructionData::String and a string pool.
- Continuations: full semantics for CONT_STORE/CONT_RESUME aligned with rholang-bytecode, including environment capture.
- ASK_NB behavior and richer selection/peek semantics.
- Comparison (CMP_*) and logical ops (NOT/AND/OR) with well-defined type rules.
- PUSH_NAME/NAME_QUOTE/NAME_UNQUOTE semantics aligned with NameRef in rholang-bytecode types.

Medium-term:
- Replace the in-VM RSpace mock with a proper RSpace interface and pluggable backends.
- Full process semantics: EVAL, EVAL_BOOL, EVAL_STAR, EXEC, SPAWN_ASYNC.
- Bundles (BUNDLE_BEGIN/BUNDLE_END) and capability propagation.
- Method dispatch: LOAD_METHOD/INVOKE_METHOD once object model stabilizes.


## Testing and Examples

Tests live under rholang-vm/tests and use rholang-bytecode instructions via rholang_vm::api. They illustrate the semantics above:
- minimal_vm_tests.rs — basic addition and HALT.
- arithmetic_tests.rs — MUL/DIV/MOD/NEG and error cases.
- collections_tests.rs — list/tuple/map creation and concat.
- collection_diff_tests.rs — list difference.
- rspace_operations_tests.rs — NAME_CREATE/TELL/ASK/PEEK and locals.

Examples (rholang-vm/examples) demonstrate usage patterns with the Process-only API:
- simple_arithmetic.rs — arithmetic flows over bytecode.
- greeter_contract.rs — a simplified greeter scenario using integer payloads and RSpace queue semantics.

Run:
- Build: `cargo build -p rholang-vm`
- Tests: `cargo test -p rholang-vm`
- Example: `cargo run -p rholang-vm --example simple_arithmetic`


## Extensibility and Contribution Guidelines

- Favor small, focused opcode implementations in opcode_exec.rs.
- Keep Process the single input to VM::execute; avoid adding parallel raw execute APIs. Helper builders are fine.
- Maintain deterministic behavior for tests (reset stack; unique NAME_CREATE ids; add helpers like VM::reset_rspace() as needed).
- Align opcode semantics to rholang-bytecode’s specification; when using immediate operands (op16), document conventions in comments and tests.
- Update this document when expanding opcode coverage or changing execution model details.


## Appendix: Mapping to rholang-bytecode

- Instruction encoding: 32-bit fixed width. VM uses Instruction::nullary/unary/binary constructors; no VM-specific instruction type exists.
- Opcode coverage matrix (current subset):
  - Control: HALT, NOP (handled).
  - Stack/Push: PUSH_INT, PUSH_BOOL, POP (handled).
  - Arithmetic: ADD, SUB, MUL, DIV, MOD, NEG (handled).
  - Compare/Logic: planned.
  - Collections: CREATE_LIST, CREATE_TUPLE, CREATE_MAP, CONCAT, DIFF (handled).
  - Process ops: planned.
  - RSpace: NAME_CREATE, TELL, ASK, PEEK (handled minimal semantics).
  - Pattern: PATTERN, MATCH_TEST, EXTRACT_BINDINGS (placeholder semantics).
  - Reference/Method: planned.

This matrix should be kept current alongside tests as implementation progresses.
