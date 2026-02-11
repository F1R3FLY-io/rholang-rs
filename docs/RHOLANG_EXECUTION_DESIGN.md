# Rholang Execution Design: Process-Oriented Bytecode

This document proposes an execution model for Rholang that explicitly splits compiled code into a set of independent, parameterized processes. Each process is a small, self-contained execution unit comprised of:

- Bytecode: a compact instruction stream
- Jump table: fast branch/continuation targets resolved at compile time
- Parameters: a vector of named or positional inputs required to run the process

A process is eligible to execute when its parameters are fully known (bound). This design enables fine‑grained scheduling, deterministic replay, and efficient distribution across threads or nodes.


## Goals

- Make concurrency and dataflow explicit at the bytecode level
- Enable incremental, demand‑driven execution (only run what is ready)
- Improve runtime schedulability and distribution
- Keep VM deterministic and replayable for blockchain and distributed settings
- Keep the surface model aligned with Rholang’s process calculus semantics


## High-Level Model

1. Compilation splits Rholang code into a directed graph of processes.
2. Each process P has:
   - P.code: immutable bytecode slice
   - P.jumps: immutable jump/label table (indices into P.code or external process ids)
   - P.params: a parameter schema (names/types/arity)
3. The runtime maintains a binding store that maps channel names, patterns, and data to values.
4. When all parameters of P become known (i.e., the binding store has values that satisfy P’s parameter schema), P becomes Ready and is put into the run queue.
5. The scheduler picks Ready processes, executes their bytecode to completion or to a yield point, and publishes any effects:
   - New name allocations
   - Sends/receives (continuations)
   - Spawned processes (new P’ with its own params)
   - State updates (per Rholang semantics)
6. Newly produced values can satisfy parameters of other processes, unlocking more Ready work.


## Process Abstraction

A process is a pure unit of execution parameterized by values. Conceptually:

- Identity: ProcessId (stable across replay)
- Schema: ordered or named parameters with optional type/shape annotations
- Code: a compact bytecode sequence for the process body
- Jump Table: pre-resolved offsets/targets for branches and continuations
- Metadata: source map, cost model slice, determinism hints

Readiness: A process is Ready when all required parameters are bound. Optional params may have defaults; variadics use a list/vec encoding.


## Parameters

Parameters represent data dependencies for the process:

- Named (by symbol) or positional (by index)
- May be values, names, patterns, or capabilities
- Binding occurs via standard Rholang communications (send/recv, matches)
- The runtime enforces that all required parameters are satisfied before execution

Example schema:

- params: [x: Int, y: Int, k: Cont]
- When x and y are both known and k is a continuation, the process executes


## Bytecode and Jump Table

- Bytecode: Uses the rholang-bytecode crate’s instruction set
- Jump Table: A compact table of offsets/labels for control flow and continuation points
  - Intra-process branches: indices into the local code slice
  - Inter-process continuations: indices or handles to other ProcessIds with parameter mapping
- Encoded to allow O(1) jump resolution without scanning bytecode


## Lifecycle

- Created: Produced by the compiler or by a running process (spawn)
- Waiting: Exists but missing some params
- Ready: All params known, queued for execution
- Running: Executing on a VM core
- Suspended: Yielded by cost limit, awaiting reschedule
- Completed: Reached terminal instruction; may produce outputs/spawns


## Scheduling

- Work-queue of Ready processes
- Fairness: round-robin or work-stealing for multi-core execution
- Determinism: execution order is derived deterministically (e.g., canonical queue order + seed)
- Yielding: long-running processes can periodically yield on cost accounting boundary
- Priority: system processes or protocol-critical continuations may be prioritized


## Effects and Dataflow

- Send/Receive operations publish values into the binding store
- Patterns bind variables (including name matches)
- Name allocation creates fresh names with scoped or global visibility
- Spawns create new processes with partially or fully known params
- Completing a process may unlock readiness of its dependents


## Example (Conceptual)

Rholang-like source:

new x, y in {
  x!(1) |
  y!(2) |
  for a <- x; b <- y do k!(a + b)
}

Compilation splits into processes:

- P1: send to x with param a=1
- P2: send to y with param b=2
- P3: join on x and y with params (xVal, yVal, k)

Runtime:

- P1 executes, publishes x=1
- P2 executes, publishes y=2
- P3 becomes Ready when xVal and yVal are known and k is available, computes sum, sends k!(3)


## Mapping to Crates

- rholang-parser: front-end AST and desugaring for process splitting
- rholang-bytecode: instruction set for process code and jump tables
- rholang-vm: runtime for scheduling, binding store, cost model, and execution


## Determinism and Replay

- Processes are immutable once created; only parameter bindings change
- Execution order is derived deterministically (e.g., canonical queue + deterministic tie-breakers)
- All effects are recorded in a journal for replay
- External inputs (e.g., deploys) are sequenced canonically


## Cost Model

- Each instruction carries a cost; processes must not exceed a per-run budget
- Long processes yield and re-enqueue, preserving fairness and liveness
- Cost accounting is part of the deterministic replay record


## Distribution

- Processes with independent parameters can be executed on different threads or nodes
- Parameter readiness can be propagated via messages across nodes
- Jump tables remain local to a process; inter-node jumps use process ids and param mapping


## Error Handling

- If parameter binding fails type/shape checks, the process is rejected with a deterministic error
- Runtime faults produce structured errors tied to ProcessId and instruction pointer
- Partial progress is guarded by journaled effects; either fully applied or rolled back on error


## Tooling and Debuggability

- Source maps connect ProcessId and instruction offsets back to AST spans
- Introspection: dump process graph, params, readiness state, and queues
- Deterministic trace logs for debugging and replay auditing