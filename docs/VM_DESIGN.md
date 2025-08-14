# Rholang VM Design

This is a practical guide to how the Rholang VM in `rholang-vm` is put together and how to work with it. If you’re trying to add an instruction, wire up a new RSpace backend, or figure out what the compiler is doing, this document is for you.

Related code and docs:
- Bytecode reference: `docs/BYTECODE_DESIGN.md`
- VM engine: `rholang-vm/src/vm.rs`
- Bytecode types/opcodes: `rholang-vm/src/bytecode.rs`
- RSpace trait and in-memory backends: `rholang-vm/src/rspace.rs`
- Compiler from AST to bytecode: `rholang-vm/src/compiler.rs`
- Public entry points: `rholang-vm/src/lib.rs`
- Examples/tests: `rholang-vm/tests/*.rs`

## What the VM tries to do (in plain terms)
- Run Rholang code that has been compiled to a small bytecode set.
- Keep pure computation (a small stack machine) separate from effects on the tuple space (RSpace).
- Make RSpace interactions explicit (produce, consume, match, name ops) so they’re easy to reason about and test.
- Be fast enough for everyday development, with a clear path to more serious backends later.

## Big picture
The VM is a classic stack machine wrapped around an execution context. Pure ops (numbers, strings, lists, control flow) live on the stack. Anything touching channels or patterns goes through the RSpace interface. The compiler emits labels for control flow; the VM resolves them up front for quick jumps.

Components at a glance:
1. Bytecode model (`bytecode.rs`)
   - `Value` for runtime values, `Instruction` for opcodes, `Label` for jumps, `RSpaceType` for backend selection.
2. Execution engine (`vm.rs`)
   - Operand stack, locals, a small constant pool, continuation table, pattern cache, name registry, and a program counter.
   - Async entry point so we can integrate with async runtimes and future async RSpace backends.
3. RSpace (`rspace.rs`)
   - `RSpace` trait plus in-memory sequential and concurrent implementations.
   - A small factory to get an instance by `RSpaceType`.
4. Compiler (`compiler.rs`)
   - Translates `rholang_parser` AST to bytecode: literals/ops/branches + explicit RSpace instructions for `new`, `for`, sends, etc.
5. Public API (`lib.rs`)
   - `RholangVM` lets you compile and execute Rholang or run raw bytecode.

## Data model you’ll actually see
- Value
  - Int(i64), String(String), Bool(bool)
  - Process(String) — quoted process as text
  - Name(String) — channel identifier (string for in-memory backends)
  - List(Vec<Value>), Tuple(Vec<Value>), Map(Vec<(Value, Value)>)
  - Nil
- Labels
  - `Label { id: String }` in bytecode; resolved to instruction indices before running.
- Continuations and patterns
  - Continuations live in a table with: a code target, snapshot of locals, pattern info, and which RSpace type they belong to.
  - Patterns are cached by string key; compiled via `Pattern/PatternCompile` instructions.

## How execution works
- The `ExecutionContext` holds the stack, locals, label index, rspace cache, etc.
- Most instructions pop arguments from the stack and push results back.
- Locals are simple slots (`AllocLocal`, `LoadLocal`, `StoreLocal`). Indices are zero-based.
- Errors are early and explicit (type mismatches, underflow, bad indices, divide by zero, unresolved labels, RSpace failures). We bubble them up with `anyhow::Result`.

Common instruction families (not exhaustive):
- Stack: `Push*`, `Pop`, `Dup`
- Control flow: `Jump`, `BranchTrue`, `BranchFalse`
- Arithmetic/compare: `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Neg`, `Cmp*`
- Bool/string/list: `Not`, `Concat`
- Collections: `CreateList`, `CreateTuple`, `CreateMap`
- RSpace: `RSpaceProduce`, `RSpaceConsume`, `RSpacePeek`, `RSpaceMatch`, `NameCreate`, `NameQuote`, `NameUnquote`
- Continuations: `ContinuationStore`, `ContinuationResume`
- Concurrency: `SpawnAsync`

If you need the exact stack signatures, look at `vm.rs::execute_instruction` — the code is the source of truth and has comments for edge cases. Tests in `rholang-vm/tests` cover realistic flows.

## RSpace: what you need to know
- The trait abstracts publish/subscribe on channels with pattern matching.
- Backends:
  - `MemorySequentialRSpace` — single-threaded, deterministic.
  - `MemoryConcurrentRSpace` — shared state via Arc/Mutex; use when you want to model parallelism.
- The VM caches RSpace instances by `RSpaceType`. RSpace opcodes fetch the right one and do the work.
- Bundle scoping (`RSpaceBundleBegin/End`) allows read/write capability checks around RSpace actions. Keep semantics aligned with `BYTECODE_DESIGN.md` if you change this.

## A few concrete examples
1) Arithmetic
```
PushInt(10), PushInt(3), Mod   => stack: [10,3] -> [1]
PushInt(5), Neg                => stack: [5] -> [-5]
```

2) Strings and lists
```
PushStr("hello "), PushStr("world"), Concat -> String("hello world")
PushInt(1), PushInt(2), CreateList(2), PushInt(3), PushInt(4), CreateList(2), Concat -> List([1,2,3,4])
```

3) Send / receive (sketch)
```
// new ch in { for(x <- ch) { ch!(x + 1) } | ch!(5) }
NameCreate(MemorySequential) -> push Name("ch#...")
Pattern("x"), PatternCompile(MemorySequential)
ContinuationStore(MemorySequential), RSpaceConsume(MemorySequential)
SpawnAsync(MemorySequential) { PushInt(5); RSpaceProduce(MemorySequential) }
// The RSpace matches and resumes the continuation with x=5
```
These correspond to tests in `tests/bytecode_examples_tests.rs`.

## Concurrency and determinism
- `SpawnAsync` is how we model `P | Q`. The compiler arranges the right combination of spawns, produces/consumes, and continuations.
- Use the sequential RSpace for deterministic runs. The concurrent backend reflects possible interleavings.

## What the compiler emits
- Literals/expressions → `Push*`, arith/compare ops, `Concat`, etc.
- Branches → `BranchTrue`/`BranchFalse` + `Jump` with generated labels.
- `new` → `NameCreate` of the selected RSpace type.
- `x!(v)` → evaluate `x` and `v`, then `RSpaceProduce`.
- `for(x <- ch) P` → pattern compile, store continuation, `RSpaceConsume` (or persistent variant), then resume when there’s a match.
- `P | Q` → `SpawnAsync` around each branch plus whatever synchronization the semantics require.

If you’re looking for details, see `compiler.rs` (`gen_label`, `compile_proc`, etc.).

## Errors and diagnostics
- We return `anyhow::Result` with descriptive messages. Typical failures:
  - divide/mod by zero
  - type mismatches on arithmetic/boolean ops
  - stack underflow / bad local index
- Tests assert on specific error messages where it helps readability.

## Extending the VM
- New instruction:
  - Add to `bytecode.rs::Instruction`.
  - Implement in `vm.rs::execute_instruction`.
  - Update the compiler if it needs to emit the new opcode.
  - Add tests (unit +, when relevant, compiler or integration).
- New value kind:
  - Extend `Value`, update display/serialization and semantics.
- New RSpace backend:
  - Implement `RSpace` for your store (e.g., LMDB, RocksDB).
  - Register it in the factory method so `RSpaceType` can select it.
- Patterns and bundles:
  - If you enhance matching or capability checks, make sure the compiler and tests reflect the new behavior.

## Async runtime notes
- The VM entry point is async (`execute(...).await`) to play nicely with async ecosystems. The in-memory backends are now implemented with Tokio (async locks) so they are non-blocking and ready for concurrency.

## Output and display
- `execute` returns a `String` for the top-of-stack value (e.g., `Int(1)`, `Bool(true)`, `List([...])`). This is mainly for tests; real integrations would likely use typed outputs.

## Where to look in the code
- `vm.rs::ExecutionContext` — state layout and helpers
- `vm.rs::VM::execute_instruction` — the bytecode dispatcher
- `rspace.rs::RSpace` — the trait and in-memory implementations
- `compiler.rs::RholangCompiler` — AST → bytecode
- `docs/BYTECODE_DESIGN.md` — authoritative opcode list and abstract model

---
If you change instructions, RSpace capabilities, or compiler behavior, please update this document and `BYTECODE_DESIGN.md` together. Keeping docs close to the code has saved us many round-trips during review.
