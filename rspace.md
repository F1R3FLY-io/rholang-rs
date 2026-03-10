### RSpace Structure and Execution Rules (LLM Guide)

#### Purpose
This document describes how RSpace is structured, how it stores processes and data, and how the scheduler interacts with it for parallel execution. It is written for LLM agents implementing or reviewing the system.

#### Crate Structure
- **`rholang-vm`**: Source of truth for core types (`Value`, `Process`, `VM`, `RSpace` trait, `execute::step`)
- **`rholang-process`**: Re-export facade that exposes all `rholang-vm` types
- **`rholang-rspace`**: RSpace implementations (`InMemoryRSpace`, `PathMapRSpace`) + re-exports from `rholang-process`

This structure allows `rholang-rspace` to depend on `rholang-process` (the facade) while `rholang-vm` remains independent.

#### RSpace Interface
`RSpace` is the minimal storage API trait, defined in `rholang-vm` and re-exported via `rholang-process`:
- `tell(kind, channel, data)` → append `Value` to a channel queue.
- `ask(kind, channel)` → destructive read of the oldest `Value` (FIFO).
- `peek(kind, channel)` → non-destructive read of the oldest `Value`.
- `reset()` → clear storage (test-only).

#### Channel Naming and Kinds
- Channels are strings formatted as `@<kind>:<name>`.
- `kind` is a `u16` namespace identifier; it must match the `@<kind>:` prefix.
- Any mismatch between `kind` and channel prefix is an error.

#### Stored Values
RSpace stores the `Value` enum from `rholang-process`:
- Primitive values (`Int`, `Bool`, `Str`, `Name`, `Nil`)
- Collections (`List`, `Tuple`, `Map`)
- Processes (`Par(Vec<Process>)`)

#### Process Storage
- Processes are stored inside `Value::Par` at a channel.
- The `source_ref` of a process acts as a stable name for event callbacks and debugging.
- Processes hold their VM instance (`process.vm`) to preserve state across executions.

#### Process States in RSpace
Each stored `Process` is in exactly one of the following states:
- `wait`: blocked; must not be executed.
- `ready`: eligible for execution.
- `value`: finished successfully with a final `Value`.
- `error`: failed with a human-readable error message.

Terminal states (`value`, `error`) must not be re-executed. The `Process::execute()` method enforces this:
- Processes in `Value` state return their cached value immediately.
- Processes in `Error` state return an error without re-execution.
- Processes in `Wait` state return an error (must be transitioned to `Ready` first).

#### Ready-Queue Drain
The helper `drain_ready_processes` performs the ready-queue scan:
1. `ask()` the channel for `Value::Par`.
2. Split into ready vs. pending (`wait` or terminal) processes.
3. Re-store pending processes in the same channel (preserving order).
4. Return only ready processes to the scheduler.

#### Execution Flow with RSpace
1. Scheduler reads ready processes from RSpace (via `drain_ready_processes`).
2. Each process executes in its own VM instance.
3. Process state transitions to `value` or `error`.
4. Event callback fires with the process name.
5. Updated processes are written back into RSpace.

#### RSpace Implementations
- `InMemoryRSpace`: HashMap-backed FIFO queues for tests and simple runs.
- `PathMapRSpace`: PathMap-backed queues for hierarchical storage.
- Both must obey FIFO semantics and channel-kind validation.

#### Concurrency Rules
- RSpace access is guarded by `Arc<Mutex<Box<dyn RSpace>>>` inside each VM.
- Parallel execution is allowed as long as RSpace operations are synchronized.
- Processes should not share mutable data outside of RSpace.

#### Testing Expectations
- Tests must verify FIFO ordering for `tell`/`ask`.
- Tests must verify ready-only draining and pending re-storage.
- Tests must validate channel-kind mismatches raise errors.

#### Documentation Requirements
- This file (`rspace.md`) must be kept in sync with RSpace semantics.
- Any changes to channel naming, process storage, or draining logic must be reflected here.