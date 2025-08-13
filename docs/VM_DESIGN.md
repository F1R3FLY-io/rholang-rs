# Rholang VM Design

This document describes the design of the Rholang Virtual Machine (VM) as implemented in the `rholang-vm` crate, aligning the implementation with the bytecode specification in `BYTECODE_DESIGN.md`. It aims to be both an architectural overview and a reference for instruction semantics, execution model, RSpace integration, and extension points.

Related documents and code:
- Bytecode specification: `docs/BYTECODE_DESIGN.md`
- VM engine: `rholang-vm/src/vm.rs`
- Bytecode types and instructions: `rholang-vm/src/bytecode.rs`
- RSpace interface and implementations: `rholang-vm/src/rspace.rs`
- Compiler (Rholang AST to bytecode): `rholang-vm/src/compiler.rs`
- Public library interface: `rholang-vm/src/lib.rs`
- Examples/tests: `rholang-vm/tests/*.rs`

## Goals and Design Constraints

The VM is designed under the following constraints (see BYTECODE_DESIGN.md Theoretical Foundation):
- Faithful representation of Rholang processes and RSpace semantics with a VM state that can be treated within a Graph-Structured Lambda Theory (GSLT) framework.
- Fully abstract encoding of RSpace states into VM states, preserving observable behavior.
- Bisimilarity preservation for corresponding RSpace/VM states.
- Practical execution on ambient hardware with efficient primitives (arithmetic, string ops) and explicit instructions for row calculus constructs (par, for, names/channels).

## Architectural Overview

The VM consists of the following layers and components:

1. Bytecode Model (`bytecode.rs`)
   - Defines the value model (`Value`), RSpace flavors (`RSpaceType`), control-flow `Label`, and the `Instruction` enum for bytecode opcodes.
   - Provides serialization hooks for bytecode (currently as utility functions and tests).

2. Execution Engine (`vm.rs`)
   - A mini-stack VM with:
     - Operand stack
     - Local variable array (dynamically grown via `AllocLocal`)
     - Constant pool
     - Process heap (map of process/object handles)
     - Continuation table (id→continuation records)
     - Pattern cache
     - Name registry
     - Program counter (pc) with `Label`-based jumps
   - Asynchronous execution entry point to allow integration with async RSpace backends.

3. RSpace Integration (`rspace.rs`)
   - Trait `RSpace` abstracts channel-based storage and matching operations.
   - MemorySequential and MemoryConcurrent implementations provide different concurrency semantics.
   - A factory constructs `RSpace` instances by `RSpaceType`.

4. Compiler (`compiler.rs`)
   - Translates parsed Rholang AST (from `rholang_parser`) into an instruction sequence matching the bytecode spec and VM expectations.
   - Manages labels for control flow and introduces RSpace-related opcodes for `new`, `for`, sends, and matches.

5. Public API (`lib.rs`)
   - `RholangVM` exposes methods to compile and execute Rholang source and to execute raw bytecode.

## Data Model

### Values
The VM uses a compact `Value` enum to represent runtime values:
- Int(i64)
- String(String)
- Bool(bool)
- Process(String) — textual representation of a quoted process; can be evaluated or sent.
- Name(String) — represents a channel identifier (for in-memory backends a convenient string id).
- List(Vec<Value>), Tuple(Vec<Value>), Map(Vec<(Value, Value)>)
- Nil

This value set is aligned with abstractions in BYTECODE_DESIGN.md (Proc, Name, Pattern, etc.) while choosing a pragmatic in-memory representation.

### Labels
Control flow targets are encoded as `Label { id: String }` in bytecode. The VM maintains a label index for quick jumps.

### Continuations and Patterns
- Continuations are represented in the VM as `ContinuationRecord` entries in a continuation table, carrying:
  - Code pointer (e.g., label or index)
  - Environment / locals snapshot
  - Pattern information (compiled pattern key)
  - RSpaceType binding
- Patterns are cached in `pattern_cache` under a string key. Pattern compilation is triggered via `Pattern`/`PatternCompile` instructions.

## Execution Model

### Context and Control Flow
The VM runs instructions in an `ExecutionContext`:
- Stack operations (`Push*`, `Pop`, `Dup`)
- Local variables (`AllocLocal`, `LoadLocal`, `StoreLocal`)
- Control flow (`Jump`, `BranchTrue`, `BranchFalse`, `BranchSuccess`)
- Arithmetic and comparisons (`Add`, `Sub`, `Mul`, `Div`, `Mod`, `Neg`, `Cmp*`)
- Boolean and string ops (`Not`, `Concat`, `Interpolate`, `Diff`)
- Data structure construction (`CreateList`, `CreateTuple`, `CreateMap`)
- Evaluation and invocation (`Eval*`, `InvokeMethod`, `LoadMethod`)
- Name/Process operations (`ProcNeg`, `Ref`, `Copy`, `Move`)

The program counter advances by one per instruction unless modified by a branch or jump. Labels are resolved before execution into concrete pc locations.

### Stack and Locals Discipline
- All non-void instructions consume arguments from the stack and push results back.
- Local variable slots are zero-based; `AllocLocal` appends a new uninitialized slot and returns its index; `StoreLocal`/`LoadLocal` access slots by index.
- Type errors (e.g., arithmetic on non-ints) produce immediate errors with descriptive messages.

### Error Handling
- The VM uses `anyhow::Result<()>` internally and escalates errors to the public API.
- Common errors include stack underflow, local index out of range, type mismatch, division/modulo by zero, unresolved labels, and RSpace operation failures.
- Branch operations check top-of-stack boolean and error if not Bool.

## RSpace Integration

RSpace abstracts the tuple-space (rho-calculus space) semantics required by Rholang:

Trait operations (selected):
- `put(channel, data)` / `produce(channel, data)`: Publish data to a channel.
- `get(channel)` / `get_nonblock(channel)`: Consume or peek data from a channel.
- `consume(channel, pattern, continuation)`: Register a waiting continuation with a match pattern.
- Name management: `name_create`, `name_quote`, `name_unquote`.
- Pattern matching: `pattern_match`, `peek`.

Backends:
- `MemorySequentialRSpace`: single-threaded, HashMap-based structures.
- `MemoryConcurrentRSpace`: multi-threaded variant using Mutex/Arc for shared state.

VM access:
- RSpace instances are allocated lazily per `RSpaceType` via `ExecutionContext::get_or_create_rspace` using a small cache keyed by type.
- RSpace-related bytecodes (`RSpaceProduce`, `RSpaceConsume`, `RSpacePeek`, `RSpaceMatch`, `NameCreate`, etc.) fetch the appropriate RSpace and perform the action.

Bundle operations:
- The VM supports bundle scoping around RSpace interactions via `RSpaceBundleBegin(type, BundleOp)` / `RSpaceBundleEnd(type)` to enforce read/write capabilities and equivalence constraints (see BYTECODE_DESIGN.md for bundle capability semantics).

## Instruction Semantics (Selected)

Below is a practical mapping for core instructions. For the authoritative list see `bytecode.rs::Instruction` and BYTECODE_DESIGN.md.

- PushInt(i64) / PushStr(String) / PushBool(bool): Push literal.
- Pop: Discard top value.
- Dup: Duplicate top value.
- Add/Sub/Mul/Div/Mod: Integer arithmetic on two top operands (lhs below rhs). Errors on type mismatch or zero division/modulo.
- Neg: Integer negation.
- Not: Boolean logical not.
- Concat: If top two are Strings, concatenates; if Lists, concatenates; otherwise error. Tests cover both cases.
- CmpEq/Neq/Lt/Lte/Gt/Gte: Comparison operators with type-aware semantics; mismatch errors where appropriate.
- CreateList(n): Pops n items and pushes List in insertion order.
- BranchTrue(Label)/BranchFalse(Label): Pops a Bool; jumps if condition is true/false.
- Jump(Label): Set pc to label.
- Label(Label): Marks a jump destination; not executed at runtime.
- Pattern(s): Load or compile a pattern designated by key `s` into the pattern cache.
- PatternCompile(rtype): Compile the pattern at top-of-stack or by key for the specified RSpace.
- ExtractBindings: After a successful match, extract variable bindings onto the stack or into locals (implementation-dependent; in current VM, produces a Value structure).
- RSpaceProduce(type): Pops (channel, data) and produces to the specified RSpace.
- RSpaceConsume(type): Pops (channel, pattern, continuation-id/closure) and registers a waiting continuation.
- RSpaceConsumeNonblock(type): Non-blocking consume variant (returns immediately with optional match).
- RSpaceConsumePersistent(type): Persistent contract-like consume; continuation remains registered.
- RSpacePeek(type), RSpaceMatch(type): Introspection operations.
- NameCreate(type): Creates a fresh unforgeable name in the chosen RSpace.
- NameQuote/NameUnquote(type): Quote/unquote processes to names.
- ContinuationStore/ContinuationResume(type): Save current continuation in table / resume by id.
- SpawnAsync(type): Spawn a concurrent task (e.g., to model parallel composition) that runs on the selected RSpace.

Note: The exact stack signatures and detailed edge cases are documented inline in `vm.rs::execute_instruction`. Tests in `tests/bytecode_examples_tests.rs` demonstrate realistic usage.

## Concurrency and Parallel Composition

- The VM offers `SpawnAsync` to represent Rholang parallel composition. In practice, the compiler emits appropriate combinations of spawn, produce/consume, and continuation instructions to model `P | Q`.
- `MemoryConcurrentRSpace` enables safe concurrent access to channels; `MemorySequentialRSpace` is suitable for deterministic, single-threaded execution.
- Continuation table allows suspension and resumption of computations upon channel matches, aligning with `for(...) { ... }` constructs.

## Determinism and Bisimilarity Considerations

- Determinism depends on the choice of RSpace backend and program structure. Sequential backend yields deterministic reduction order; concurrent backend reflects potential interleavings.
- The instruction set respects GSLT encoding boundaries by separating pure computation (stack machine) from RSpace effects (explicit instructions). This separation allows reasoning about observational equivalence and bisimulation at the boundary.

## Compiler Mapping (AST → Bytecode)

The compiler (`RholangCompiler`) walks `rholang_parser` AST and emits bytecode:
- Literals and simple expressions map to stack machine ops (`Push*`, arithmetic, comparisons).
- Control-flow (if/then/else) maps to conditional branches and labels.
- New/channel creation maps to `NameCreate` with the chosen `RSpaceType` (defaults are determined by compiler options; see `compiler.rs`).
- Sends (`x!(v)`) compile to `RSpaceProduce(type)` after evaluating `x` and `v`.
- Receives (`for(x <- ch) P`) compile to `PatternCompile`, `ContinuationStore`, and `RSpaceConsume`/`RSpaceConsumePersistent` followed by either immediate `ContinuationResume` or deferred resumption depending on match availability.
- Parallel composition (`P | Q`) compiles to `SpawnAsync` around each branch with appropriate synchronization as needed by the semantics.

The compiler maintains a label generator and environment for locals; see `gen_label`, `compile_proc`, and helpers in `compiler.rs`.

## Error Handling and Diagnostics

- VM and RSpace return `anyhow::Result` errors with context. Examples covered in tests:
  - Division/modulo by zero
  - Type mismatches in arithmetic/boolean ops
  - Stack underflow
- For test ergonomics, helper utilities assert on specific error strings.

## Testing Strategy

The repository includes unit and integration tests:
- Bytecode-level tests (`tests/bytecode_examples_tests.rs`) cover arithmetic, logical ops, string/list concat, comparisons, and RSpace examples for sends/receives and parallel composition.
- Compiler tests (`compiler.rs` tests) verify compilation of arithmetic, if/then/else, and `new` constructs.
- RSpace tests (`rspace.rs` tests) verify basic put/get/peek/name operations and the factory.
- Benches (`benches/execution.rs`) measure VM execution throughput on selected bytecode sequences.

Developers should add tests alongside new instructions or RSpace features to maintain coverage and prevent regressions.

## Extensibility

- Instruction Set: Add new opcodes to `bytecode.rs::Instruction` and implement semantics in `vm.rs::execute_instruction`. Update compiler if needed.
- Values: Extend `Value` with new variants; ensure display/serialization and VM semantics exist.
- RSpace Backends: Implement the `RSpace` trait for new storage engines (e.g., persistent LMDB-backed variants for blockchain integration as outlined in BYTECODE_DESIGN.md). Register in the RSpace factory.
- Patterns: Enhance `PatternCompiled` and related instructions to support advanced guards and structural matching.
- Bundles: Enforce capability constraints more strictly in `RSpaceBundleBegin/End` handling.

## Execution Walkthrough Examples

1. Arithmetic program
```
PushInt(10), PushInt(3), Mod   => stack: [10,3] -> [1]
PushInt(5), Neg                => stack: [5] -> [-5]
```

2. String and list concatenation
```
PushStr("hello "), PushStr("world"), Concat -> String("hello world")
PushInt(1), PushInt(2), CreateList(2), PushInt(3), PushInt(4), CreateList(2), Concat -> List([1,2,3,4])
```

3. Simple send/receive
```
// new ch in { for(x <- ch) { ch!(x + 1) } | ch!(5) }
NameCreate(MemorySequential) -> push Name("ch#...")
Pattern("x"), PatternCompile(MemorySequential)
ContinuationStore(MemorySequential), RSpaceConsume(MemorySequential)
SpawnAsync(MemorySequential) { PushInt(5); RSpaceProduce(MemorySequential) }
// RSpace matches and resumes continuation with binding x=5
```

These correspond to tests in `tests/bytecode_examples_tests.rs`.

## Notes on Async Runtime

- The VM exposes an async `execute(&program).await -> Result<String>` to accommodate potential async RSpace operations in the future and to interoperate with asynchronous runtimes used in tests (`tokio::runtime::Runtime::new()?.block_on(...)`).
- Internally, current RSpace implementations are synchronous; the async boundary prepares the design for future networked/persistent backends.

## Deterministic Output and Display

- The VM’s `execute` returns a `String` representation of the top-of-stack result (e.g., `Int(1)`, `Bool(true)`, `List([...])`). This is primarily for test verification; production integration would expose typed results or effects.

## Future Work

- Implement persistent RSpace backends (`StoreSequential`, `StoreConcurrent`) as sketched in the bytecode design.
- Enrich pattern language and guards; compile AST patterns into compact bytecode and efficient matchers.
- Gas metering and cost accounting at instruction granularity.
- Deterministic parallel reduction strategies for blockchain contexts.
- Enhanced diagnostics and tracing for debugging and formal verification.

## Cross-References

- See `docs/BYTECODE_DESIGN.md` for the abstract types (Proc, Name, Pattern, Continuation) and encoding format. The VM binds these abstractions to concrete runtime structures (`Value`, `ContinuationRecord`, `PatternCompiled`) while preserving the separation between pure computation and RSpace effects.
- Code anchors:
  - `vm.rs::ExecutionContext` for state layout and helpers
  - `vm.rs::VM::execute_instruction` for opcode dispatch and semantics
  - `rspace.rs::RSpace` trait and in-memory backends
  - `compiler.rs::RholangCompiler` for AST→bytecode mapping

---
This document is intended to evolve with the implementation. When adding new instructions, RSpace capabilities, or compiler features, update both this document and `BYTECODE_DESIGN.md` to remain consistent.
