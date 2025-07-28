# Rholang Virtual Machine

## Introduction

The Rholang Virtual Machine (RhoVM) is a specialized runtime environment designed to execute Rholang programs. Unlike traditional virtual machines that manage concurrency through shared memory and locks, RhoVM embraces the true nature of Rholang's concurrency model by treating each process as an independent execution unit with its own isolated state machine.

This document describes the architecture, design principles, and implementation details of the RhoVM, focusing on its bytecode representation, execution model, and integration with the RSpace storage layer.

## Theoretical Design: Path-Based Architecture

The theoretical design of the RhoVM is built on a path-based architecture that directly reflects the mathematical foundations of the Rho-calculus. In this architecture:

1. **Processes are Paths**: Each concurrent process executes within its own path context
2. **Concurrency is Explicit**: Parallel composition creates independent execution paths
3. **Communication is Message-Based**: Processes interact solely through message passing
4. **State is Isolated**: Each path maintains its own isolated execution state
5. **Execution is Truly Concurrent**: Paths can execute in parallel without artificial sequentialization

### Core Mathematical Principles

The path-based RhoVM design is guided by two key mathematical principles that ensure functoriality - the property that compilation preserves the structure of the source language:

1. **Functoriality of Quoting**: The compilation of a quoted process (@P) must be equivalent to a QUOTE operation on the already-compiled bytecode of the process P. This is expressed by the formula:
   ```
   [| @P |] = QUOTE [| P |]
   ```
   This ensures that quoting operations maintain their semantic meaning through compilation.

2. **Functoriality of Concurrency**: The compilation of a parallel composition of processes (P1 | … | Pn) must result in a multiset containing the individual compiled bytecodes of each process. This is expressed by the formula:
   ```
   [| P1 | … | Pn |] = {| [| P1 |], …, [| Pn |] |}
   ```
   This ensures that parallel composition directly translates to independent bytecode sequences.

These principles ensure that the VM's behavior precisely matches the mathematical semantics of the Rho-calculus, making the system both predictable and formally verifiable.

### Path-Based Bytecode Design

The path-based RhoVM uses a 32-bit fixed-width bytecode format with a path-based execution model. The bytecode is designed to preserve the mathematical properties of the Rho-calculus throughout the compilation and execution pipeline.

#### Core Path-Based Bytecode Instructions

```
PATH_ALLOC              // Allocate new execution path
PATH_FORK n             // Fork current path into n child paths
PATH_JOIN paths         // Join multiple paths into current path
PATH_SYNC paths         // Synchronize multiple paths at barrier
PATH_BIND var path      // Bind variable to specific path
PATH_LOAD var path      // Load variable from path context
PATH_STORE var path     // Store variable in path context
PATH_EXEC path          // Execute process in specific path
PATH_SPAWN path         // Spawn process in new path thread
PATH_ROUTE chan paths   // Route message across paths
PATH_LISTEN chan paths  // Listen on channel across paths
```

### Path-Based Execution Model

#### Path-Based State Machine

Each bytecode sequence is executed by a dedicated state machine within its own execution path. The path-based state machine maintains:

1. **Instruction Pointer**: Points to the current instruction being executed
2. **Operand Stack**: Holds intermediate values during computation
3. **Path Context**: Contains the execution environment specific to this path
4. **Local Variables**: Stores variables declared within the process, bound to the path
5. **Channel References**: Maintains references to channels used by the process
6. **Private Channel**: A unique channel assigned to this path for primitive operations
7. **Path Relationships**: References to parent, child, and sibling paths

The execution cycle follows these steps:

1. Fetch the next instruction from the bytecode sequence
2. Decode the instruction to determine the operation
3. Execute the operation, which may modify the path's state
4. Update the instruction pointer and path state
5. Repeat until the bytecode sequence is exhausted or blocked on a receive operation

#### Thread Management

In the path-based RhoVM, threads are invoked at the level of execution paths, each identified by its path context in RSpace. A thread is not a traditional OS thread or green thread, but an execution context associated with a specific path.

The lifecycle of a path-based thread includes:

1. **Path Creation**: When a new process is spawned
2. **Path Forking**: When parallel compositions create multiple child paths
3. **Path Scheduling**: When paths are selected for execution
4. **Path Execution**: When a path executes its bytecode sequence
5. **Path Blocking**: When a path blocks on a receive operation
6. **Path Synchronization**: When paths synchronize at barriers
7. **Path Resumption**: When a blocked path is resumed
8. **Path Joining**: When child paths are joined back into their parent
9. **Path Termination**: When a path completes its execution

### Path-Based RSpace Integration

The path-based RhoVM deeply integrates with RSpace (the tuplespace-based storage layer) through a technique called the "Channel Trick." This technique embeds all computation, including primitive operations, into the RSpace communication model.

When a path-based state machine executes a primitive operation:

1. It creates a message containing the operation, operands, path context, and continuation channel
2. This message is sent on the path's private channel to a primitive processor
3. The processor performs the operation in the context of the specified path
4. The result is sent back on the continuation channel, tagged with the path identifier
5. The state machine receives the result and continues execution

This approach makes the integration with RSpace natural and powerful, as each path is identified by its context in RSpace, and all interaction happens through message passing.

### Distributed Execution

The path-based architecture naturally extends to distributed execution across multiple physical machines. Since each path is identified by its context in RSpace, and all interaction happens through message passing, the physical location of a path is transparent to other paths.

In a distributed setting:

1. **Path-Aware Channel Routing**: Messages are routed to the appropriate physical node
2. **Path Migration**: Entire paths can be migrated between nodes for load balancing
3. **Path-Based Distributed Scheduling**: Resource allocation is coordinated across nodes
4. **Path-Level Fault Tolerance**: Failed paths can be recovered and resumed on other nodes
5. **Path Locality Optimization**: Related paths can be co-located to minimize communication overhead

## Current Implementation: Stack-Based VM

While the path-based architecture provides a strong theoretical foundation, the current implementation of the Rholang VM follows a more traditional stack-based approach as described in BYTECODE_DESIGN.md. This implementation provides a practical starting point that can evolve toward the theoretical design over time.

### Stack-Based Architecture Overview

The current Rholang VM implementation consists of several key components:

1. **Stack-Based VM**: A traditional stack-based virtual machine that executes bytecode instructions
2. **RSpace Types**: Different storage types for data (memory/store, sequential/concurrent)
3. **Bytecode Format**: A set of instructions for computational operations, control flow, and RSpace interactions
4. **Compiler**: Translates Rholang code to bytecode
5. **Interpreter Provider**: Integrates the VM with the shell

### Stack-Based Bytecode Format

The current implementation uses a bytecode format with several categories of instructions:

#### Computational Instructions
```
NOP                 // No operation
PUSH_INT n          // Push integer literal
PUSH_STR s          // Push string literal
PUSH_BOOL b         // Push boolean literal
PUSH_PROC proc      // Push process to stack
POP                 // Pop top of stack
DUP                 // Duplicate top of stack
LOAD_VAR n          // Load variable by index
LOAD_LOCAL n        // Load local variable by index
STORE_LOCAL n       // Store to local variable
ALLOC_LOCAL         // Allocate new local slot
BRANCH_TRUE L       // Conditional jump if true
BRANCH_FALSE L      // Conditional jump if false
BRANCH_SUCCESS L    // Branch if operation succeeded
JUMP L              // Unconditional jump
CMP_EQ              // Equality comparison
CMP_NEQ             // Inequality comparison
CMP_LT              // Less than comparison
CMP_LTE             // Less than or equal
CMP_GT              // Greater than comparison
CMP_GTE             // Greater than or equal
ADD                 // Arithmetic addition
SUB                 // Arithmetic subtraction
MUL                 // Arithmetic multiplication
DIV                 // Arithmetic division
MOD                 // Arithmetic modulo
NEG                 // Arithmetic negation
NOT                 // Logical NOT
CONCAT              // String/collection concatenation
DIFF                // Collection difference
INTERPOLATE         // String interpolation
CREATE_LIST n       // Create list from n stack elements
CREATE_TUPLE n      // Create tuple from n stack elements
INVOKE_METHOD       // Method invocation
```

#### Evaluation Instructions
```
EVAL                // Evaluate process on stack
EVAL_BOOL           // Evaluate to boolean
EVAL_TO_RSPACE      // Evaluate and prepare for RSpace
EVAL_WITH_LOCALS    // Evaluate with local bindings
EVAL_IN_BUNDLE      // Evaluate in bundle context
EXEC                // Execute process on stack
```

#### Pattern Matching Instructions
```
PATTERN pat         // Load pattern
MATCH_TEST          // Test pattern match (leaves boolean on stack)
EXTRACT_BINDINGS    // Extract bound variables from pattern match
```

#### RSpace Instructions
```
RSPACE_PUT <type>          // Put data into specified RSpace
RSPACE_GET <type>          // Get data from specified RSpace (blocking)
RSPACE_GET_NONBLOCK <type> // Get data from specified RSpace (non-blocking)
RSPACE_CONSUME <type>      // Consume data from specified RSpace
RSPACE_PRODUCE <type>      // Produce data to specified RSpace
RSPACE_PEEK <type>         // Peek at data without consuming
RSPACE_MATCH <type>        // Pattern match against specified RSpace data
RSPACE_SELECT <type>       // Atomic select operation across channels
NAME_CREATE <type>         // Create fresh name in specified RSpace
NAME_QUOTE <type>          // Quote process to name in specified RSpace
NAME_UNQUOTE <type>        // Unquote name to process in specified RSpace
PATTERN_COMPILE <type>     // Compile pattern for specified RSpace matching
PATTERN_BIND <type>        // Bind pattern variables from specified RSpace match
CONTINUATION_STORE <type>  // Store continuation in specified RSpace
CONTINUATION_RESUME <type> // Resume stored continuation from specified RSpace
RSPACE_BUNDLE_BEGIN <type> // Start bundle in specified RSpace
RSPACE_BUNDLE_END <type>   // End bundle in specified RSpace
```

### Stack-Based Execution Model

The current VM implementation uses a traditional stack-based execution model:

1. **ExecutionContext**: Represents the execution state of a program, including:
   - Stack: For computational operations
   - Locals: For local variables
   - Instruction Pointer: Points to the current instruction
   - Labels: Mapping of labels to instruction indices

2. **VM**: The virtual machine that executes bytecode instructions:
   - Fetches instructions from the bytecode program
   - Executes each instruction, which may modify the execution context
   - Returns the result of execution

The execution cycle is:
1. Fetch the next instruction from the bytecode program
2. Execute the instruction, which may push/pop values from the stack, modify local variables, or interact with RSpace
3. Update the instruction pointer
4. Repeat until the program completes or an error occurs

### RSpace Types and Usage

The current implementation supports four RSpace types as defined in BYTECODE_DESIGN.md:

1. **MemorySequential**: In-memory sequential storage (hashmap)
   - Used for simple, single-threaded operations
   - Temporary data that doesn't need persistence
   - Testing and development
   - Local variable bindings

2. **MemoryConcurrent**: In-memory concurrent storage (concurrent hashmap)
   - Used for concurrent operations within a single process
   - Temporary data with multiple threads
   - High-performance concurrent operations
   - Real-time processing

3. **StoreSequential**: On-store sequential storage (not yet implemented)
   - Will be used for persistent data that needs to survive process restarts
   - Single-threaded persistent operations
   - Configuration data
   - Simple persistent contracts

4. **StoreConcurrent**: On-store concurrent storage (not yet implemented)
   - Will be used for persistent data with concurrent access
   - Multi-process communication
   - Production blockchain operations
   - Distributed systems

The VM interacts with RSpace through type-specific instructions that specify which RSpace type to use for each operation.

### Compilation Process

The current implementation includes a compiler that translates Rholang code to bytecode:

1. **Parsing**: The Rholang parser converts source code to an Abstract Syntax Tree (AST)
2. **Compilation**: The compiler translates each AST node to a sequence of bytecode instructions
3. **Optimization**: (Not yet implemented) Optimize the bytecode for better performance
4. **Execution**: The VM executes the bytecode

The compiler follows the patterns defined in BYTECODE_DESIGN.md for translating Rholang constructs to bytecode.

### Shell Integration

The current implementation integrates with the shell through the InterpreterProvider interface:

1. **RholangVMInterpreterProvider**: Implements the InterpreterProvider interface using the Rholang VM
2. **Interpretation Process**:
   - Parse Rholang code to AST
   - Compile AST to bytecode
   - Execute bytecode in the VM
   - Return the result
3. **Process Management**:
   - List running processes
   - Kill specific processes
   - Kill all processes

This integration allows the VM to be used as a drop-in replacement for other interpreter providers in the shell.

### Examples

Here's an example of how a simple Rholang program is compiled to bytecode and executed:

Rholang code:
```
1 + 2
```

Bytecode:
```
PUSH_INT 1
PUSH_INT 2
ADD
```

Execution:
1. Push 1 onto the stack
2. Push 2 onto the stack
3. Pop 2 and 1 from the stack, add them, and push the result (3) back onto the stack

Another example with control flow:

Rholang code:
```
if (true) { 1 } else { 2 }
```

Bytecode:
```
PUSH_BOOL true
EVAL_BOOL
BRANCH_FALSE else_branch
PUSH_INT 1
JUMP end
LABEL else_branch
PUSH_INT 2
LABEL end
```

Execution:
1. Push true onto the stack
2. Evaluate to boolean (still true)
3. Branch to else_branch if false (not taken)
4. Push 1 onto the stack
5. Jump to end
6. End of program, result is 1

## Relationship Between Theoretical Design and Current Implementation

The theoretical path-based design and the current stack-based implementation represent different approaches to executing Rholang code:

1. **Concurrency Model**:
   - Path-based: Concurrency is explicit through independent execution paths
   - Stack-based: Concurrency is managed through RSpace operations

2. **Execution Model**:
   - Path-based: Each process executes in its own isolated path context
   - Stack-based: Processes execute in a shared VM with a stack and local variables

3. **RSpace Integration**:
   - Path-based: Deep integration through the "Channel Trick"
   - Stack-based: Integration through specific RSpace instructions

4. **Distributed Execution**:
   - Path-based: Naturally distributed through path-aware channel routing
   - Stack-based: Not yet designed for distributed execution

The current stack-based implementation provides a practical starting point that can evolve toward the theoretical path-based design over time. Future work may include:

1. Introducing path contexts to the stack-based VM
2. Implementing path-based concurrency within the current framework
3. Enhancing RSpace integration to support the "Channel Trick"
4. Adding support for distributed execution

## Conclusion

The Rholang Virtual Machine represents a significant advancement in the execution of concurrent, distributed programs. The theoretical path-based design provides a strong foundation based on the mathematical principles of the Rho-calculus, while the current stack-based implementation offers a practical starting point.

By continuing to evolve the implementation toward the theoretical design, the RhoVM will become an increasingly powerful platform for executing Rholang programs in a way that fully embraces the language's concurrency model.

The key innovations in the RhoVM design include:

1. The direct representation of parallel composition as independent paths (theoretical)
2. The execution of each process in its own isolated path context (theoretical)
3. The use of path-aware channels to embed all computation into the RSpace communication model (theoretical)
4. The integration of a stack-based VM with RSpace for practical implementation (current)
5. The support for different RSpace types to handle various use cases (current)

These innovations enable a VM that is mathematically consistent, practically implementable, and can evolve to be highly scalable, fault-tolerant, and naturally distributed.