# Rholang VM Design (current implementation)

This document describes the current design and implementation status of the rholang-vm, rholang-process, and rholang-rspace crates. It reflects the Process-based VM rebuilt on top of rholang-bytecode, the RSpace storage abstraction, process state management, and the supported instruction subset.

The design intentionally favors clarity and composability over completeness at this stage. It is meant to guide contributors, inform integrators, and serve as a reference for tests and examples.


## Goals and Scope

- Provide a minimal, well-factored Rholang virtual machine that executes bytecode produced and described by the rholang-bytecode crate.
- Adopt a Process-centric execution model with a simple value stack and locals.
- Support RSpace as an abstract storage interface with pluggable implementations.
- Model process states (wait, ready, value, error) for scheduling and execution.
- Support core opcodes for arithmetic, comparisons, logic, collections, control flow, pattern placeholders, locals, and RSpace operations.
- Keep tight alignment with rholang-bytecode's Instruction/Opcode and evolve by incrementally implementing more semantics and opcodes.

Out of scope for the current milestone:
- Full evaluator for process expressions.
- Real matching engine and tuplespace semantics.
- String literal infrastructure (ExtendedInstruction) and complex data pools.
- Full contract installation semantics beyond a minimal placeholder.


## Crate Architecture

The implementation is split across three crates with clear separation of concerns:

### rholang-vm (Source of Truth for Execution)
Core types, traits, and execution logic:
- **Execution**: `execute::step(vm, locals, names, inst)` - single instruction dispatcher that operates on VM state and process-provided locals/names
- **VM state**: `VM` struct with stack, rspace, continuation storage, name counter
- **Values**: `Value` enum (Int, Bool, Str, Name, List, Tuple, Map, Par, Nil)
- **Process**: `Process` struct manages execution state, event callbacks, and drives the step loop
- **State machine**: `ProcessState` enum (Wait, Ready, Value, Error) for scheduling
- **Events**: `ProcessEvent` enum (Value, Error) for execution callbacks
- **RSpace trait**: abstract storage interface (tell, ask, peek, reset)
- **Errors**: `ExecError` type for execution errors

Key design: `step()` takes `locals: &mut Vec<Value>` and `names: &[Value]` instead of a Process reference, allowing clean separation between execution logic and process management. The EVAL opcode returns `StepResult::Eval(Value)` which Process handles for sub-process execution.

### rholang-process (Re-export Facade)
Re-exports all public types from rholang-vm:
- Provides a stable API for downstream crates
- Enables dependency inversion between rholang-vm and rholang-rspace
- Contains only `lib.rs` with re-exports (no duplicate source files)

### rholang-rspace
RSpace implementations and utilities:
- `InMemoryRSpace`: HashMap-backed FIFO queues for tests and simple runs
- `PathMapRSpace`: PathMap-backed queues for hierarchical storage (default)
- `drain_ready_processes()`: helper for ready-queue scanning
- `ensure_kind_matches_channel()`: channel-kind validation
- Re-exports all types from rholang-process for convenience


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


## Core Data Structures

### Value
Runtime data used by the VM and stored in RSpace:
- `Int(i64)` - 64-bit signed integer
- `Bool(bool)` - boolean
- `Str(String)` - string
- `Name(String)` - channel name (formatted as `@<kind>:<id>`)
- `List(Vec<Value>)` - ordered list
- `Tuple(Vec<Value>)` - fixed-size tuple
- `Map(Vec<(Value, Value)>)` - key-value pairs
- `Par(Vec<Process>)` - parallel composition of processes
- `Nil` - null value

### Process
Executable unit with state:
- `code: Vec<Instruction>` - bytecode to execute
- `source_ref: String` - provenance/debug tag (stable name for callbacks)
- `locals: Vec<Value>` - local variable slots
- `names: Vec<Value>` - string pool for PUSH_STR
- `vm: VM` - embedded VM instance for execution
- `state: ProcessState` - current process state
- `parameters: Vec<Parameter>` - named bindings to RSpace channels

### Parameter
Named binding that relates a process to an entry (channel, process, or value) in RSpace:
- `name: String` - unique name identifying the entry in RSpace

A parameter represents a dependency that must be resolved before a process can execute. The parameter's name identifies an entry in RSpace, and the parameter is "solved" based on the entry's type and state.

**Entry Types in RSpace:**
```rust
pub enum Entry {
    /// Channel with FIFO queue of values
    Channel(Vec<Value>),
    /// Registered process with state tracking
    Process { state: ProcessState },
    /// Direct terminal value
    Value(Value),
}
```

**Solved State Rules:**
1. A parameter is **unsolved** if no entry exists with that name
2. A parameter is **solved** if the entry is `Entry::Channel` with a non-empty queue
3. A parameter is **solved** if the entry is `Entry::Process` in terminal `ProcessState::Value` state
4. A parameter is **unsolved** if the entry is `Entry::Process` in `Wait`, `Ready`, or `Error` state
5. A parameter is **solved** if the entry is `Entry::Value` (always terminal)

**Process Ready State:**
- A process with zero parameters is always ready (if in `ProcessState::Ready`)
- A process with one or more parameters is ready only if ALL parameters are solved
- The `is_ready()` method checks both the process state and parameter states

```rust
pub struct Parameter {
    pub name: String,  // Entry name (e.g., "input", "worker_result")
}

impl Parameter {
    /// Check if this parameter is solved by looking up entry state in RSpace
    pub fn is_solved(&self, rspace: &dyn RSpace) -> bool;
}
```

**Name Resolution Flow:**
```
                    ┌─────────────────────┐
                    │ Parameter with name │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │ Look up entry in    │
                    │ RSpace by name      │
                    └──────────┬──────────┘
                               │
         ┌─────────────────────┼─────────────────────┐
         │ Not found           │ Found               │
         │                     │                     │
   ┌─────▼─────┐    ┌──────────▼──────────┐   ┌─────▼─────┐
   │ UNSOLVED  │    │ Check entry type    │   │           │
   └───────────┘    └──────────┬──────────┘   └───────────┘
                               │
         ┌─────────────────────┼─────────────────────┐
         │                     │                     │
   ┌─────▼─────┐         ┌─────▼─────┐         ┌─────▼─────┐
   │ Channel   │         │ Process   │         │ Value     │
   └─────┬─────┘         └─────┬─────┘         └─────┬─────┘
         │                     │                     │
   ┌─────▼─────┐         ┌─────▼─────┐         ┌─────▼─────┐
   │ Non-empty?│         │ In Value  │         │ SOLVED    │
   │ → SOLVED  │         │ state?    │         │ (always)  │
   │ Empty?    │         │ → SOLVED  │         └───────────┘
   │ → UNSOLVED│         │ Other?    │
   └───────────┘         │ → UNSOLVED│
                         └───────────┘
```

**Process Dependency Example:**
```
Process A (name="worker"):
  - Registered in RSpace as Entry::Process
  - Executes and produces result
  - State changes to Value(result)

Process B (parameters=["worker"]):
  - Waits for "worker" entry to be solved
  - When Process A completes, "worker" entry becomes solved
  - Process B can now execute
```

### ProcessState
Execution state machine:
- `Ready` - eligible for execution (default state)
- `Wait` - blocked, must not be executed
- `Value(Value)` - finished successfully (terminal)
- `Error(String)` - failed with error message (terminal)

Terminal states must not be re-executed.

### VM
Virtual machine state:
- `stack: Vec<Value>` - value stack
- `rspace: Arc<Mutex<Box<dyn RSpace>>>` - synchronized RSpace access
- `cont_last: Option<(u32, Value)>` - single-slot continuation storage
- `next_cont_id: u32` - monotonic continuation counter
- `next_name_id: u64` - monotonic fresh-name counter

### RSpace Trait
Unified storage interface for channels, processes, and values (from rholang-vm, re-exported via rholang-process):

```rust
/// Entry types that can be stored in RSpace
pub enum Entry {
    /// Channel with FIFO queue of values
    Channel(Vec<Value>),
    /// Registered process with state tracking
    Process { state: ProcessState },
    /// Direct terminal value
    Value(Value),
}

pub trait RSpace: Send + Sync {
    // === Entry-based API (unified) ===

    /// Get entry by name
    fn get_entry(&self, name: &str) -> Option<Entry>;

    /// Check if an entry exists and is in a solved state
    fn is_solved(&self, name: &str) -> bool;

    // === Channel operations (for Entry::Channel) ===

    /// Put data into a channel (creates channel if not exists)
    fn tell(&mut self, name: &str, data: Value) -> Result<()>;

    /// Destructive read: remove and return oldest value from channel
    fn ask(&mut self, name: &str) -> Result<Option<Value>>;

    /// Non-destructive read: return oldest value without removing
    fn peek(&self, name: &str) -> Result<Option<Value>>;

    // === Process operations (for Entry::Process) ===

    /// Register a process by name (initial state)
    fn register_process(&mut self, name: &str, state: ProcessState) -> Result<()>;

    /// Update a registered process's state
    fn update_process(&mut self, name: &str, state: ProcessState) -> Result<()>;

    /// Get a registered process's state
    fn get_process_state(&self, name: &str) -> Option<ProcessState>;

    // === Value operations (for Entry::Value) ===

    /// Store a direct value (terminal, immutable)
    fn set_value(&mut self, name: &str, value: Value) -> Result<()>;

    /// Get a stored value
    fn get_value(&self, name: &str) -> Option<Value>;

    // === Utility ===

    /// Reset all storage
    fn reset(&mut self);
}
```

**Entry Semantics:**
- **Channel**: FIFO queue, supports multiple values, `tell` appends, `ask` pops first
- **Process**: State tracked, solved when in `ProcessState::Value` state
- **Value**: Immutable once set, always considered solved

**Name Uniqueness:**
Each name in RSpace identifies exactly one entry. You cannot have a channel and a process with the same name. Attempting to use channel operations on a process entry (or vice versa) results in an error.


## Channel Naming and Kinds

- Channels are strings formatted as `@<kind>:<name>`.
- `kind` is a `u16` namespace identifier; it must match the `@<kind>:` prefix in the channel string.
- Any mismatch between `kind` and channel prefix raises an error.
- NAME_CREATE generates fresh channels using the VM's monotonic counter.


## Execution Model

### Single Process Execution
- Entry point: `Process::execute(&mut self) -> Result<Value, ExecError>`
- With event callback: `Process::execute_with_event(&mut self, handler) -> Result<Value, ExecError>`
- The VM clears its stack at the start of each execution for isolation.
- A PC-based loop fetches instructions and dispatches them to `execute::step()`.
- step() signature: `step(vm, locals, names, inst) -> Result<StepResult, ExecError>`
  - Takes process locals and names by reference, not the Process itself
  - Allows clean separation between VM execution and process management
- step() returns `StepResult`: Next, Stop, Jump(usize), or Eval(Value).
  - `Eval(Value)`: returned by EVAL opcode; Process handles sub-process execution
- The result is the top of the stack at termination or Value::Nil if empty.
- Process state transitions to Value or Error after execution.
- Event callback fires with the process source_ref.

### Ready-Queue Drain
The `drain_ready_processes()` helper in rholang-rspace:
1. Calls `ask()` on the channel for `Value::Par`.
2. Splits into ready vs. pending (Wait or terminal) processes.
3. Re-stores pending processes in the same channel (preserving order).
4. Returns only ready processes to the caller.

### Parallel Execution
The `execute_ready_processes()` helper in rholang-vm:
1. Takes a list of processes and optional event handler.
2. Executes only Ready processes (skips Wait/terminal).
3. Updates process states to Value or Error.
4. Returns updated processes and execution results.

### Error Handling
- Type errors or stack underflow emit `ExecError` with descriptive messages.
- Out-of-bounds locals accesses also error.
- Channel-kind mismatches raise errors.

### Determinism and Isolation
- The stack is reset per execution call.
- Fresh names are generated via VM.next_name_id.
- RSpace access is synchronized via Arc<Mutex<>>.
- RSpace can be reset via VM::reset_rspace() for test isolation.


## Implemented Opcode Semantics

### Control Flow
- `HALT` - stop execution
- `NOP` - no operation
- `JUMP` - unconditional jump to absolute index (op16)
- `BRANCH_TRUE` - conditional jump if stack top is true
- `BRANCH_FALSE` - conditional jump if stack top is false
- `BRANCH_SUCCESS` - jump if stack top indicates success

### Stack/Push
- `PUSH_INT` - push i16 immediate sign-extended to i64
- `PUSH_BOOL` - push boolean from op1 bit
- `PUSH_STR` - push string from names pool at index op16
- `PUSH_NIL` - push Nil
- `POP` - discard top

### Arithmetic (Int only unless specified)
- `ADD` - Int+Int -> Int; Str+Str -> Str; List+List -> List concat
- `SUB` - Int-Int -> Int
- `MUL` - Int*Int -> Int
- `DIV` - Int/Int -> Int (error on zero or non-Ints)
- `MOD` - Int%Int -> Int (error on zero or non-Ints)
- `NEG` - -Int -> Int

### Comparisons
- `CMP_EQ` - equality test (any types), push Bool
- `CMP_NEQ` - inequality test, push Bool
- `CMP_LT` - less than (Int only), push Bool
- `CMP_LTE` - less than or equal (Int only), push Bool
- `CMP_GT` - greater than (Int only), push Bool
- `CMP_GTE` - greater than or equal (Int only), push Bool

### Logical
- `AND` - Bool && Bool -> Bool
- `OR` - Bool || Bool -> Bool
- `NOT` - !Bool -> Bool

### Collections
- `CREATE_LIST n` - pop n values, push List
- `CREATE_TUPLE n` - pop n values, push Tuple
- `CREATE_MAP n` - pop n pairs, push Map
- `CONCAT` - Str+Str or List+List concatenation
- `DIFF` - List-List difference (multiset semantics)

### Patterns (placeholders)
- `PATTERN` - push Nil
- `MATCH_TEST` - pop value and pattern, push Bool(true)
- `EXTRACT_BINDINGS` - push empty Map

### Locals
- `ALLOC_LOCAL` - push Nil into process.locals
- `LOAD_LOCAL idx` - push clone of locals[idx]
- `STORE_LOCAL idx` - pop value, assign to locals[idx]

### Continuations
- `CONT_STORE` - pop value, store in cont_last, push id
- `CONT_RESUME` - pop id, push stored value or Nil

### RSpace Operations
- `NAME_CREATE kind` - generate fresh channel, push Name
- `TELL kind` - pop data then channel, append to queue, push Bool(true)
- `ASK kind` - pop channel, pop head of queue (or Nil)
- `PEEK kind` - pop channel, clone head of queue (or Nil)

### Process Operations
- `EVAL` - evaluate value on stack: if Par, execute ready processes and return results; otherwise pass through unchanged
- `SPAWN_ASYNC n` - pop n values, combine Par values into a single Par


## RSpace Implementations

Both implementations use unified Entry-based storage.

### InMemoryRSpace
- `HashMap<String, Entry>` storage
- Name-based lookup for all entry types
- FIFO queue semantics for channels
- Suitable for tests and simple runs

### PathMapRSpace
- `PathMap<Entry>` storage with hierarchical keys
- Name-based lookup for all entry types
- FIFO queue semantics for channels
- Default implementation for the system

### Entry Storage Example
```rust
// Channel entry (FIFO queue)
rspace.tell("inbox", Value::Int(42));    // Creates Entry::Channel
rspace.tell("inbox", Value::Int(43));    // Appends to queue
rspace.ask("inbox");                      // Returns Some(42), removes it

// Process entry (state tracked)
rspace.register_process("worker", ProcessState::Ready);
rspace.update_process("worker", ProcessState::Value(Value::Int(100)));
rspace.is_solved("worker");               // Returns true

// Value entry (immutable)
rspace.set_value("config", Value::Str("production".into()));
rspace.get_value("config");               // Returns Some(Str("production"))
```


## Concurrency Rules

- RSpace access is guarded by `Arc<Mutex<Box<dyn RSpace>>>` inside each VM.
- Parallel execution is allowed as long as RSpace operations are synchronized.
- Processes should not share mutable data outside of RSpace.
- Each process preserves its VM instance across executions.


## Testing and Examples

Tests live under rholang-vm/tests and rholang-rspace/tests:

### rholang-vm tests
- minimal_vm_tests.rs — basic addition and HALT
- arithmetic_tests.rs — MUL/DIV/MOD/NEG and error cases
- comparison_tests.rs — CMP_* operations
- control_flow_tests.rs — JUMP/BRANCH operations
- collections_tests.rs — list/tuple/map creation and concat
- collection_diff_tests.rs — list difference
- rspace_operations_tests.rs — NAME_CREATE/TELL/ASK/PEEK
- parallel_exec_tests.rs — parallel execution with events

### rholang-rspace tests
- rspace_rules_tests.rs — comprehensive tests for all rspace.md rules:
  - RSpace interface (tell, ask, peek, reset)
  - Channel naming and kind validation
  - FIFO ordering verification
  - All Value variants storage
  - Process storage with Value::Par
  - Process state handling
  - Ready-queue drain semantics
  - Execution flow and event callbacks
  - Concurrency with Arc<Mutex<>>

### Examples
- simple_arithmetic.rs — arithmetic flows over bytecode
- greeter_contract.rs — greeter scenario using RSpace queues

### Run Commands
- Build: `cargo build -p rholang-vm`
- Tests: `cargo test -p rholang-vm` or `cargo test -p rholang-rspace`
- Example: `cargo run -p rholang-vm --example simple_arithmetic`


## Not Implemented (yet) / Roadmap

Near-term priorities:
- String support via ExtendedInstruction/InstructionData::String and a string pool.
- Full continuation semantics with environment capture.
- ASK_NB behavior and richer selection/peek semantics.
- PUSH_NAME/NAME_QUOTE/NAME_UNQUOTE semantics.

Medium-term:
- Additional process semantics: EVAL_BOOL, EVAL_STAR, EXEC.
- Real matching engine and tuplespace semantics.
- Bundles (BUNDLE_BEGIN/BUNDLE_END) and capability propagation.
- Method dispatch: LOAD_METHOD/INVOKE_METHOD once object model stabilizes.


## Extensibility and Contribution Guidelines

- Favor small, focused opcode implementations in execute.rs.
- Keep Process as the single public execution entry point.
- Maintain deterministic behavior for tests.
- Align opcode semantics to rholang-bytecode's specification.
- All RSpace implementations must obey FIFO semantics and channel-kind validation.
- Update this document when expanding opcode coverage or changing execution model details.
- Keep rspace.md in sync with RSpace semantics.


## Appendix: Opcode Coverage Matrix

| Category      | Opcodes                                           | Status      |
|---------------|---------------------------------------------------|-------------|
| Control       | HALT, NOP, JUMP, BRANCH_TRUE/FALSE/SUCCESS        | Implemented |
| Stack/Push    | PUSH_INT, PUSH_BOOL, PUSH_STR, PUSH_NIL, POP      | Implemented |
| Arithmetic    | ADD, SUB, MUL, DIV, MOD, NEG                      | Implemented |
| Compare       | CMP_EQ, CMP_NEQ, CMP_LT, CMP_LTE, CMP_GT, CMP_GTE | Implemented |
| Logic         | AND, OR, NOT                                      | Implemented |
| Collections   | CREATE_LIST, CREATE_TUPLE, CREATE_MAP, CONCAT, DIFF | Implemented |
| Locals        | ALLOC_LOCAL, LOAD_LOCAL, STORE_LOCAL              | Implemented |
| Continuations | CONT_STORE, CONT_RESUME                           | Implemented |
| RSpace        | NAME_CREATE, TELL, ASK, PEEK                      | Implemented |
| Pattern       | PATTERN, MATCH_TEST, EXTRACT_BINDINGS             | Placeholder |
| Process ops   | EVAL, SPAWN_ASYNC                                 | Implemented |
| Process ops   | EXEC                                              | Planned     |
| Bundles       | BUNDLE_BEGIN, BUNDLE_END                          | Planned     |
| Methods       | LOAD_METHOD, INVOKE_METHOD                        | Planned     |

This matrix should be kept current alongside tests as implementation progresses.
