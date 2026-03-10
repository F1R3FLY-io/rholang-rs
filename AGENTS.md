### Rholang Project Guide for LLM Agents

#### Purpose
This document provides a high-level map of the Rholang workspace, intended for LLM agents making changes across crates. Keep it current when architecture or execution flow changes.

#### Workspace Structure
- `rholang-process`: Core runtime types.
  - Owns `Process`, `ProcessState`, `ProcessEvent`, `ProcessEventHandler`, `Value`, `ExecError`.
  - Owns `VM` and bytecode execution logic.
  - Owns the `RSpace` trait.
- `rholang-rspace`: Storage implementations and helpers.
  - Implements `RSpace` (`InMemoryRSpace`, `PathMapRSpace`).
  - Re-exports core types from `rholang-process`.
  - Contains `drain_ready_processes` for ready-queue scheduling.
- `rholang-vm`: API facade.
  - Re-exports process/VM types.
  - Provides `execute_ready_processes` batch execution helper.
- `rholang-compiler`: Compiler from AST to bytecode `Process` values.
- `rholang-interpreter`: Async interpreter that compiles, stores, retrieves, and executes processes via RSpace.
- `rholang-shell`: CLI and shell utilities (compile/disassemble/execute flows).
- `rholang-bytecode`: Instruction set and bytecode utilities.
- `rholang-lib`: Semantic analysis and compiler support.
- `rholang-parser`: Parsing front-end.
- `rholang-wasm`: WASM bindings for eval/disassemble.

#### Execution Model (Summary)
- Processes have a state machine: `wait` → `ready` → (`value` | `error`).
- Each process executes in its own `VM` instance; the VM owns an `Arc<Mutex<Box<dyn RSpace>>>` handle.
- `Process::execute` is the primary execution entry point.
- `execute_ready_processes` runs ready processes in parallel and emits events.

#### RSpace Contract
- RSpace stores `Value::Par(Vec<Process>)` for process queues.
- `drain_ready_processes` extracts ready processes and re-stores pending ones.
- Channel names must match the `kind` prefix (`@<kind>:...`).
- See `rspace.md` for full RSpace details.

#### Event Emission Rules
- After successful execution, processes emit `ProcessEvent::Value(process_name)`.
- After failure, processes emit `ProcessEvent::Error(process_name)`.
- Event handlers must be thread-safe and can be invoked from parallel contexts.

#### Testing Focus Areas
- Process state transitions and event emission.
- Ready-only execution in batch runs.
- RSpace FIFO behavior and channel-kind validation.
- Integration: compile → store in RSpace → retrieve → execute.

#### Documentation Requirements
- Keep `AGENTS.md` and `rspace.md` synchronized with architecture changes.
- Update docs when process states, event semantics, or execution flow changes.