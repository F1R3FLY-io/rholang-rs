# Rholang Bytecode Design

For a complete and accurate implementation of bytecode, here we will describe a tentative structure of converting each Rholang instruction from grammar into bytecode.

## Architecture Overview
- **Mini-Stack VM**: Handles computational operations, control flow, local variables
- **RSpace Types**:
    - `RSPACE_MEM_SEQ`: In-memory sequential (hashmap)
    - `RSPACE_MEM_CONC`: In-memory concurrent (concurrent hashmap)
    - `RSPACE_STORE_SEQ`: On-store sequential (LMDB wrapper)
    - `RSPACE_STORE_CONC`: On-store concurrent (LMDB wrapper)
- **Integration**: VM communicates with appropriate RSpace through typed instructions (need to change to VM that representable as a GLST and fully abstract encoding of VM state and RSpace state)

### RSpace Type Selection Guidelines
**Name Analysis and Escape Analysis**: The choice of RSpace type is determined by static analysis of name usage and escape analysis during compilation, not at runtime. This analysis determines:

- **Local vs Shared Names**: Whether a name escapes its local scope
- **Concurrent vs Sequential**: Whether multiple threads will access the name
- **Persistent vs Memory**: Whether the name needs to survive process restarts

**`RSPACE_MEM_SEQ`**:
- Simple, single-threaded operations
- Temporary data that doesn't need persistence
- Local variable bindings (determined by escape analysis)
- Names that don't escape their local scope

**`RSPACE_MEM_CONC`**:
- Concurrent operations within a single process
- Temporary data with multiple threads
- High-performance concurrent operations
- Real-time processing
- Local names that are accessed concurrently (e.g., in `let x = P & y = Q`)

**`RSPACE_STORE_SEQ`**:
- Persistent data that needs to survive process restarts
- Single-threaded persistent operations
- Simple persistent contracts

**`RSPACE_STORE_CONC`**:
- Persistent data with concurrent access
- Multi-process communication
- Production blockchain operations
- Distributed systems
- Top-level names and contracts

**Examples**
```
// Top-level: MUST use persistent storage
new toplevel in { contract Foo() = ... }  // → RSPACE_STORE_CONC

// Nested: CAN use memory storage  
{ new local in { local!(42) | for(x <- local) P } }  // → RSPACE_MEM_CONC
```

### Core VM Instructions
```
COMPUTATIONAL INSTRUCTIONS:
├── NOP                 // No operation
├── PUSH_INT n          // Push integer literal
├── PUSH_STR s          // Push string literal
├── PUSH_BOOL b         // Push boolean literal
├── PUSH_PROC proc      // Push process to stack
├── POP                 // Pop top of stack
├── DUP                 // Duplicate top of stack
├── LOAD_VAR n          // Load variable by index
├── LOAD_LOCAL n        // Load local variable by index
├── STORE_LOCAL n       // Store to local variable
├── ALLOC_LOCAL         // Allocate new local slot
├── BRANCH_TRUE L       // Conditional jump if true
├── BRANCH_FALSE L      // Conditional jump if false
├── BRANCH_SUCCESS L    // Branch if operation succeeded
├── JUMP L              // Unconditional jump
├── CMP_EQ              // Equality comparison
├── CMP_NEQ             // Inequality comparison
├── CMP_LT              // Less than comparison
├── CMP_LTE             // Less than or equal
├── CMP_GT              // Greater than comparison
├── CMP_GTE             // Greater than or equal
├── ADD                 // Arithmetic addition
├── SUB                 // Arithmetic subtraction
├── MUL                 // Arithmetic multiplication
├── DIV                 // Arithmetic division
├── MOD                 // Arithmetic modulo
├── NEG                 // Arithmetic negation
├── NOT                 // Logical NOT
├── CONCAT              // String/collection concatenation
├── DIFF                // Collection difference
├── INTERPOLATE         // String interpolation
├── CREATE_LIST n       // Create list from n stack elements
├── CREATE_TUPLE n      // Create tuple from n stack elements
├── CREATE_MAP n        // Create map from n key-value pairs on stack
├── INVOKE_METHOD       // Method invocation

EVALUATION INSTRUCTIONS:
├── EVAL                // Evaluate process on stack (with current locals)
├── EVAL_BOOL           // Evaluate to boolean
├── EXEC                // Execute process on stack

PATTERN MATCHING INSTRUCTIONS:
├── PATTERN pat         // Load pattern
├── MATCH_TEST          // Test pattern match (leaves boolean on stack)
├── EXTRACT_BINDINGS    // Extract bound variables from pattern match

PROCESS CONTROL INSTRUCTIONS:
├── SPAWN_ASYNC         // Spawn process asynchronously
├── PROC_NEG            // Process negation

REFERENCE INSTRUCTIONS:
├── COPY                // Copy value
├── MOVE                // Move value
├── REF                 // Create reference
├── LOAD_METHOD name    // Load method name for invocation
```

### RSpace Instructions (Type-Specific)
```
RSPACE INSTRUCTIONS:
├── RSPACE_PRODUCE <type>          // Produce data to specified RSpace
├── RSPACE_CONSUME <type>          // Consume data from specified RSpace (blocking)
├── RSPACE_CONSUME_NONBLOCK <type> // Consume data from specified RSpace (non-blocking)
├── RSPACE_PEEK <type>             // Peek at data without consuming
├── RSPACE_MATCH <type>            // Pattern match against specified RSpace data
├── RSPACE_SELECT <type>           // Atomic select operation across channels
├── NAME_CREATE <type>             // Create fresh name in specified RSpace
├── NAME_QUOTE <type>              // Quote process to name in specified RSpace
├── NAME_UNQUOTE <type>            // Unquote name to process in specified RSpace
├── PATTERN_COMPILE <type>         // Compile pattern for specified RSpace matching
├── PATTERN_BIND <type>            // Bind pattern variables from specified RSpace match
├── CONTINUATION_STORE <type>      // Store continuation in specified RSpace
├── CONTINUATION_RESUME <type>     // Resume stored continuation from specified RSpace
├── RSPACE_BUNDLE_BEGIN <type>     // Start bundle in specified RSpace
├── RSPACE_BUNDLE_END <type>       // End bundle in specified RSpace
```

**Note on RSpace Type Selection**: The `<type>` parameter is determined by static analysis during compilation. Two possible implementation approaches:
1. **Explicit Type Flags**: Each instruction carries the RSpace type as a parameter
2. **Channel Properties**: RSpace remembers properties when names are created, requiring upgrade/downgrade instructions

### Real rholang code to bytecode transformation examples
**Name Creation**
```
[[new x, y in P]] <top level>
├──NAME_CREATE RSPACE_STORE_CONC     // Create fresh name x in persistent RSpace
├──STORE_LOCAL 0                     // Store x in local slot 0
├──NAME_CREATE RSPACE_STORE_CONC     // Create fresh name y in persistent RSpace  
├──STORE_LOCAL 1                     // Store y in local slot 1
├──PUSH_PROC P                       // Push process P
├──EVAL                              // Evaluate with local bindings

[[new x, y in { ... new p, q in Q ... }]] <nested context> (can use memory RSpace for nested names)
├──NAME_CREATE RSPACE_MEM_CONC       // Create fresh name x in memory RSpace
├──STORE_LOCAL 0                     // Store x in local slot 0
├──NAME_CREATE RSPACE_MEM_CONC       // Create fresh name y in memory RSpace
├──STORE_LOCAL 1                     // Store y in local slot 1
├──PUSH_PROC { ... new p, q in Q ... } // Push nested process
├──EVAL                              // Evaluate with local bindings
// Inner new p, q also uses RSPACE_MEM_CONC
```

**Send Operation**
```
[[chan!(data)]] <where chan is top-level>
├──PUSH_PROC data                    // Push data (not evaluated)
├──PUSH_PROC chan                    // Push channel name
├──EVAL                              // Evaluate channel only
├──RSPACE_PRODUCE RSPACE_STORE_CONC  // Produce to persistent RSpace

[[localChan!(10 + 5)]] <where localChan is local> (memory channel with top-level expression evaluation)
├──PUSH_INT 15                       // 10 + 5 evaluated to 15 at compile time
├──PUSH_PROC localChan               // Push channel name
├──EVAL                              // Evaluate channel
├──RSPACE_PRODUCE RSPACE_MEM_CONC    // Produce to memory RSpace
```

**Receive Operation**
```
[[for(x <- publicChannel) P]] <contract context> (persistent channel)
├──LOAD_VAR publicChannel            // Load channel name
├──ALLOC_LOCAL                       // Allocate slot for x
├──PATTERN_COMPILE RSPACE_STORE_CONC x  // Compile pattern for persistent RSpace
├──CONTINUATION_STORE RSPACE_STORE_CONC P // Store continuation persistently
├──RSPACE_CONSUME RSPACE_STORE_CONC  // Consume from persistent RSpace
├──PATTERN_BIND RSPACE_STORE_CONC    // Bind received value to x
├──CONTINUATION_RESUME RSPACE_STORE_CONC // Resume with bound x

[[for(x <- localChan) P]] <inside local scope> (memory channel)
├──LOAD_VAR localChan                // Load channel name
├──ALLOC_LOCAL                       // Allocate slot for x
├──PATTERN_COMPILE RSPACE_MEM_CONC x // Compile pattern for memory RSpace
├──CONTINUATION_STORE RSPACE_MEM_CONC P // Store continuation in memory
├──RSPACE_CONSUME RSPACE_MEM_CONC    // Consume from memory RSpace
├──PATTERN_BIND RSPACE_MEM_CONC      // Bind received value to x
├──CONTINUATION_RESUME RSPACE_MEM_CONC // Resume with bound x
```

**Complex Pattern**
```
[[for(pattern{x, y} <- chan) P]] <complex pattern>
├──LOAD_VAR chan                     // Load channel name
├──PATTERN_COMPILE RSPACE_STORE_CONC pattern{x, y} // Compile complex pattern
├──CONTINUATION_STORE RSPACE_STORE_CONC P // Store continuation
├──RSPACE_CONSUME RSPACE_STORE_CONC  // Consume with pattern matching
├──PATTERN_BIND RSPACE_STORE_CONC    // Extract bound variables x, y
├──CONTINUATION_RESUME RSPACE_STORE_CONC // Resume with all bindings
```

**Let Binding**
```
[[let x = 10 + 5; y = "hello" in P]] <sequential binding>
├──PUSH_INT 15                       // Evaluate 10 + 5 to 15
├──ALLOC_LOCAL                       // Allocate slot for x
├──STORE_LOCAL 0                     // Store 15 in x
├──PUSH_STR "hello"                  // Push string literal
├──ALLOC_LOCAL                       // Allocate slot for y
├──STORE_LOCAL 1                     // Store "hello" in y
├──PUSH_PROC P                       // Push body process
├──EVAL                              // Execute with bindings

[[let x = expensiveComputation() & y = anotherProcess() in P]] <concurrent binding>
├──NAME_CREATE RSPACE_MEM_CONC       // Create coordination channel
├──STORE_LOCAL 0                     // Store coordination channel
├──PUSH_PROC expensiveComputation()  // Push first process
├──CONTINUATION_STORE RSPACE_MEM_CONC x_binding // Store x binding continuation
├──SPAWN_ASYNC RSPACE_MEM_CONC       // Spawn first evaluation
├──PUSH_PROC anotherProcess()        // Push second process  
├──CONTINUATION_STORE RSPACE_MEM_CONC y_binding // Store y binding continuation
├──SPAWN_ASYNC RSPACE_MEM_CONC       // Spawn second evaluation
├──RSPACE_CONSUME RSPACE_MEM_CONC    // Wait for first binding
├──RSPACE_CONSUME RSPACE_MEM_CONC    // Wait for second binding
├──PUSH_PROC P                       // Push body process
├──EVAL                              // Execute with accumulated bindings
```

### Core Process Constructs
**Parallel Composition (par)**
```
par (P | Q)  -> BYTECODE
├── EVAL_PROCESS P       // Evaluate P in current context
└── EVAL_PROCESS Q       // Evaluate Q in current context
```

**Name Creation (new)**
Top-level names (persistent, shared):
```
new x, y in P -> BYTECODE (top-level, requires persistent RSpace)
├── NAME_CREATE RSPACE_STORE_CONC   // Create fresh name in persistent RSpace
├── STORE_LOCAL 0                   // Store x in local slot
├── NAME_CREATE RSPACE_STORE_CONC   // Create fresh name for y
├── STORE_LOCAL 1                   // Store y in local slot
├── PUSH_PROC P                     // Push process P
└── EVAL                            // Evaluate with local bindings
```

Nested names (local, can use memory RSpace):
```
new x, y in { ... new p, q in Q ... } -> BYTECODE (nested, can use memory)
├── NAME_CREATE RSPACE_MEM_CONC     // Create fresh name in memory RSpace
├── STORE_LOCAL 0                   // Store x in local slot
├── NAME_CREATE RSPACE_MEM_CONC     // Create fresh name for y
├── STORE_LOCAL 1                   // Store y in local slot
├── PUSH_PROC P                     // Push process P
└── EVAL                            // Evaluate with local bindings
```

**Asynchronous Send (channel!)**
Memory-based (local channels):
```
chan!(data)  -> BYTECODE
├── PUSH_PROC data                  // Push data to stack (not evaluated)
├── PUSH_PROC chan                  // Push channel name
├── EVAL                            // Evaluate channel only
└── RSPACE_PRODUCE RSPACE_MEM_CONC  // Produce data into concurrent memory RSpace
```

Persistent (shared/top-level channels):
```
chan!(data)  -> BYTECODE
├── PUSH_PROC data                  // Push data to stack (not evaluated)
├── PUSH_PROC chan                  // Push channel name  
├── EVAL                            // Evaluate channel only
└── RSPACE_PRODUCE RSPACE_STORE_CONC // Produce data into persistent concurrent RSpace
```
**Note on Evaluation**: Only top-level expressions are evaluated during send. Arguments remain as suspended computations (closures) to preserve pattern matching capabilities and enable lazy evaluation semantics.

**Synchronous Send (send_sync)**
```
chan!?(data); P  -> BYTECODE (memory-based)
├── LOAD_VAR chan                       // Load channel name
├── PUSH_PROC data                      // Push data process (not evaluated)
├── NAME_QUOTE RSPACE_MEM_CONC          // Convert to name through memory RSpace
├── CONTINUATION_STORE RSPACE_MEM_CONC P // Store continuation in memory RSpace
├── RSPACE_PRODUCE_SYNC RSPACE_MEM_CONC // Send and wait for ack
└── CONTINUATION_RESUME RSPACE_MEM_CONC // Resume when ack received

chan!?(data); P  -> BYTECODE (persistent)
├── LOAD_VAR chan                         // Load channel name
├── PUSH_PROC data                        // Push data process (not evaluated)
├── NAME_QUOTE RSPACE_STORE_CONC          // Convert to name through persistent RSpace
├── CONTINUATION_STORE RSPACE_STORE_CONC P // Store continuation in persistent RSpace
├── RSPACE_PRODUCE_SYNC RSPACE_STORE_CONC // Send and wait for ack
└── CONTINUATION_RESUME RSPACE_STORE_CONC // Resume when ack received
```

**Input/Receive (for)**
Simple receive without pattern matching (memory-based):
```
for(x <- chan) P  -> BYTECODE
├── LOAD_VAR chan                       // Load channel name
├── ALLOC_LOCAL                         // Allocate slot for x
├── PATTERN_COMPILE RSPACE_MEM_CONC x   // Compile simple variable pattern
├── CONTINUATION_STORE RSPACE_MEM_CONC P // Store continuation in memory RSpace
├── RSPACE_CONSUME RSPACE_MEM_CONC      // Consume from memory RSpace
├── PATTERN_BIND RSPACE_MEM_CONC        // Bind received value to x
└── CONTINUATION_RESUME RSPACE_MEM_CONC // Resume with bound x
```

Pattern matching receive (memory-based):
```
for(pattern <- chan) P  -> BYTECODE
├── LOAD_VAR chan                         // Load channel name
├── PATTERN_COMPILE RSPACE_MEM_CONC pattern // Compile pattern for memory RSpace
├── CONTINUATION_STORE RSPACE_MEM_CONC P  // Store continuation in memory RSpace
├── RSPACE_CONSUME RSPACE_MEM_CONC        // Consume from memory RSpace with pattern matching
├── PATTERN_BIND RSPACE_MEM_CONC          // Extract all bound variables
└── CONTINUATION_RESUME RSPACE_MEM_CONC   // Resume with bindings
```
**Multiple receive patterns**:
```
for (x <- a; y <- b) { P }     // Sequential - both must match
for (x <- a & y <- b) { P }    // Concurrent - can match in any order
for (x,y,z <= chan) { P }      // Repeated receive: for(x,y,z <- chan) { P | for(x,y,z <- chan) { P } }
for (x,y,z <<- chan) { P }     // Peek bind: for(x,y,z <- chan) { chan!(x,y,z) | P }
```

**Replicated Receive (contract)**
```
contract Name(x) = P  -> BYTECODE
├── PUSH_PROC Name                      // Push contract name
├── EVAL                                // Evaluate name
├── PATTERN_COMPILE RSPACE_STORE_CONC x // Compile pattern for persistent RSpace
├── CONTINUATION_STORE RSPACE_STORE_CONC P // Store contract body persistently
└── RSPACE_CONSUME_PERSISTENT RSPACE_STORE_CONC // Set up persistent consumer
```

### Control Flow Constructs
**Conditional (if-else)**
```
if (cond) P else Q  -> BYTECODE
├── PUSH_PROC cond      // Push condition
├── EVAL_BOOL           // Evaluate to boolean
├── BRANCH_FALSE L1     // Jump to L1 if false
├── PUSH_PROC P         // Push then-branch
├── EXEC                // Execute P
├── JUMP L2             // Skip else-branch
├── L1: PUSH_PROC Q     // Label L1: Push else-branch
├── EXEC                // Execute Q
└── L2: NOP             // Label L2: Continue
```

**Pattern Matching (match)**
```
match expr { pat1 => P1; pat2 => P2 }  -> BYTECODE
├── PUSH_PROC expr      // Push expression to match
├── EVAL                // Evaluate expression
├── PATTERN pat1        // Try pattern 1
├── MATCH_TEST          // Test match (leaves boolean on stack)
├── BRANCH_FALSE L1     // Jump if no match
├── EXTRACT_BINDINGS    // Extract bound variables
├── PUSH_PROC P1        // Push body 1
├── EXEC                // Execute P1
├── JUMP L_END          // Jump to end
├── L1: PATTERN pat2    // Try pattern 2
├── MATCH_TEST          // Test match
├── BRANCH_FALSE L2     // Jump if no match
├── EXTRACT_BINDINGS    // Extract bound variables
├── PUSH_PROC P2        // Push body 2
├── EXEC                // Execute P2
└── L_END: NOP          // Continue
```

**Select/Choice (select)**
* Dont use for now, not implemented in Rholang
```
select { x <- chan1 => P1; y <- chan2 => P2 }  -> BYTECODE
├── RSPACE_SELECT_BEGIN RSPACE_MEM_CONC // Begin atomic select in memory RSpace
├── PUSH_PROC chan1                     // Push first channel
├── EVAL_TO_RSPACE                      // Evaluate channel 1
├── PATTERN_COMPILE RSPACE_MEM_CONC x   // Compile pattern for memory RSpace
├── CONTINUATION_STORE RSPACE_MEM_CONC P1 // Store continuation in memory RSpace
├── RSPACE_SELECT_ADD RSPACE_MEM_CONC   // Add to select set
├── PUSH_PROC chan2                     // Push second channel
├── EVAL_TO_RSPACE                      // Evaluate channel 2
├── PATTERN_COMPILE RSPACE_MEM_CONC y   // Compile pattern for memory RSpace
├── CONTINUATION_STORE RSPACE_MEM_CONC P2 // Store continuation in memory RSpace
├── RSPACE_SELECT_ADD RSPACE_MEM_CONC   // Add to select set
├── RSPACE_SELECT_WAIT RSPACE_MEM_CONC  // Wait for any channel
├── PATTERN_BIND RSPACE_MEM_CONC        // Bind variables from selected channel
└── CONTINUATION_RESUME RSPACE_MEM_CONC // Resume selected continuation
```
* The RSpace would need to support this atomic selection mechanism, probably using something like compare-and-swap operations or locks to ensure atomicity

### Expression Constructs
**Arithmetic Operations** - The conversion of these operations will be almost the same, so I won't describe them all. But here are the types we have in general:
Addition, Subtraction, Multiplication, Division, Modulo, Negation?
```
P + Q  -> BYTECODE
├── PUSH_PROC P         // Push left operand
├── EVAL                // Evaluate P
├── PUSH_PROC Q         // Push right operand
├── EVAL                // Evaluate Q
└── ADD                 // Perform addition
```

**Logical Operations**
```
P and Q  -> BYTECODE
├── PUSH_PROC P         // Push left operand
├── EVAL_BOOL           // Evaluate to boolean
├── DUP                 // Duplicate result
├── BRANCH_FALSE L1     // Short-circuit if false
├── POP                 // Remove duplicate
├── PUSH_PROC Q         // Push right operand
├── EVAL_BOOL           // Evaluate to boolean
└── L1: NOP             // Result is on stack
```

**Method Call**
```
obj.method(args)  -> BYTECODE
├── PUSH_PROC obj                       // Push receiver object
├── EVAL                                // Evaluate receiver
├── PUSH_PROC args                      // Push arguments
├── EVAL                                // Evaluate arguments
├── LOAD_METHOD method                  // Load method name
└── INVOKE_METHOD RSPACE_MEM_CONC       // Invoke method with RSpace context
```

### Comparison Expression Constructs
**Equality Comparison**
```
P == Q  -> BYTECODE
├── PUSH_PROC P         // Push left operand
├── EVAL                // Evaluate P
├── PUSH_PROC Q         // Push right operand
├── EVAL                // Evaluate Q
└── CMP_EQ              // Compare for equality, push boolean result
```

**Inequality Comparison**
```
P != Q  -> BYTECODE
├── PUSH_PROC P         // Push left operand
├── EVAL                // Evaluate P
├── PUSH_PROC Q         // Push right operand
├── EVAL                // Evaluate Q
└── CMP_NEQ             // Compare for inequality, push boolean result
```

**All Other Comparisons (!=, <, <=, >, >=)**
```
P <op> Q  -> BYTECODE
├── PUSH_PROC P         // Push left operand
├── EVAL                // Evaluate P
├── PUSH_PROC Q         // Push right operand
├── EVAL                // Evaluate Q
└── CMP_<OP>            // Perform comparison operation
```

### Logical/Pattern Constructs
**Matches Expression**
```
P matches Q  -> BYTECODE
├── PUSH_PROC P         // Push value to match
├── EVAL                // Evaluate P
├── PATTERN Q           // Load pattern Q
└── MATCH_TEST          // Test if P matches Q (leaves boolean on stack)
```

**Logical NOT**
```
not P  -> BYTECODE
├── PUSH_PROC P         // Push operand
├── EVAL_BOOL           // Evaluate to boolean
└── NOT                 // Logical negation
```

**Logical OR**
```
P or Q  -> BYTECODE
├── PUSH_PROC P         // Push left operand
├── EVAL_BOOL           // Evaluate to boolean
├── DUP                 // Duplicate result
├── BRANCH_TRUE L1      // Short-circuit if true
├── POP                 // Remove duplicate
├── PUSH_PROC Q         // Push right operand
├── EVAL_BOOL           // Evaluate to boolean
└── L1: NOP             // Result is on stack
```

### Process Logic Constructs
**Conjunction (Process AND)**
```
P /\ Q  -> BYTECODE
├── PUSH_PROC P         // Push left process
├── PUSH_PROC Q         // Push right process
└── CONJ                // Process conjunction (both must succeed)
```

**Disjunction (Process OR)**
```
P \/ Q  -> BYTECODE
├── PUSH_PROC P         // Push left process
├── PUSH_PROC Q         // Push right process
└── DISJ                // Process disjunction (either can succeed)
```

**Process Negation**
```
~P  -> BYTECODE
├── PUSH_PROC P         // Push process
└── PROC_NEG            // Process negation
```

### String/Collection Operations
**String Concatenation**
```
P ++ Q  -> BYTECODE
├── PUSH_PROC P         // Push left operand
├── EVAL                // Evaluate P
├── PUSH_PROC Q         // Push right operand
├── EVAL                // Evaluate Q
└── CONCAT              // Concatenate strings/collections
```

**Collection Difference**
```
P -- Q  -> BYTECODE
├── PUSH_PROC P         // Push left operand
├── EVAL                // Evaluate P
├── PUSH_PROC Q         // Push right operand
├── EVAL                // Evaluate Q
└── DIFF                // Collection difference operation
```

**String Interpolation**
```
P %% Q  -> BYTECODE
├── PUSH_PROC P         // Push format string
├── EVAL                // Evaluate P
├── PUSH_PROC Q         // Push value to interpolate  
├── EVAL                // Evaluate Q
└── INTERPOLATE         // Perform string interpolation
```

### Variable binding constructs
**Let Binding (Linear)**
```
let x = P; y = Q in R  -> BYTECODE
├── PUSH_PROC P         // Push value for x
├── EVAL                // Evaluate P (stack-based)
├── ALLOC_LOCAL         // Allocate slot for x
├── STORE_LOCAL 0       // Store P result in x
├── PUSH_PROC Q         // Push value for y
├── EVAL                // Evaluate Q (stack-based)
├── ALLOC_LOCAL         // Allocate slot for y
├── STORE_LOCAL 1       // Store Q result in y
├── PUSH_PROC R         // Push body process R
└── EVAL                // Evaluate R with local bindings
```

**Let Binding (Concurrent)**
```
let x = P & y = Q in R  -> BYTECODE (memory-based)
├── NAME_CREATE RSPACE_MEM_CONC                 // Create coordination channel
├── STORE_LOCAL 0                               // Store coordination channel
├── PUSH_PROC P                                 // Push process P
├── CONTINUATION_STORE RSPACE_MEM_CONC x_binding // Store x binding continuation
├── SPAWN_ASYNC RSPACE_MEM_CONC                 // Spawn P evaluation in memory RSpace
├── PUSH_PROC Q                                 // Push process Q
├── CONTINUATION_STORE RSPACE_MEM_CONC y_binding // Store y binding continuation
├── SPAWN_ASYNC RSPACE_MEM_CONC                 // Spawn Q evaluation in memory RSpace
├── RSPACE_CONSUME RSPACE_MEM_CONC              // Wait for both bindings
├── RSPACE_CONSUME RSPACE_MEM_CONC              // Wait for both bindings
├── PUSH_PROC R                                 // Push body process R
└── EVAL                                        // Execute R with accumulated bindings
```

### Data Constructs
**List Construction**
```
[P, Q, R]  -> BYTECODE
├── PUSH_PROC P         // Push first element
├── EVAL                // Evaluate P
├── PUSH_PROC Q         // Push second element
├── EVAL                // Evaluate Q
├── PUSH_PROC R         // Push third element
├── EVAL                // Evaluate R
└── CREATE_LIST 3       // Create list from 3 elements on stack
```

**Map Construction**
```
{key1: val1, key2: val2}  -> BYTECODE
├── PUSH_PROC key1      // Push first key
├── EVAL                // Evaluate key1
├── PUSH_PROC val1      // Push first value
├── EVAL                // Evaluate val1
├── PUSH_PROC key2      // Push second key
├── EVAL                // Evaluate key2
├── PUSH_PROC val2      // Push second value
├── EVAL                // Evaluate val2
└── CREATE_MAP 2        // Create map from 2 key-value pairs on stack
```

**Single Element Tuple**
```
(P,)  -> BYTECODE
├── PUSH_PROC P         // Push element
├── EVAL                // Evaluate element
└── CREATE_TUPLE 1      // Create tuple with 1 element
```

**Multi-Element Tuple**
```
(P, Q, R)  -> BYTECODE
├── PUSH_PROC P         // Push first element
├── EVAL                // Evaluate P
├── PUSH_PROC Q         // Push second element
├── EVAL                // Evaluate Q
├── PUSH_PROC R         // Push third element
├── EVAL                // Evaluate R
└── CREATE_TUPLE 3      // Create tuple with 3 elements
```

### Advanced Constructs
**Bundle Operations**
```
bundle+ { P }  -> BYTECODE (memory-based)
├── RSPACE_BUNDLE_BEGIN RSPACE_MEM_CONC WRITE // Start write bundle in memory RSpace
├── PUSH_PROC P                             // Push bundled process
├── EVAL_IN_BUNDLE                          // Evaluate in bundle context
└── RSPACE_BUNDLE_END RSPACE_MEM_CONC       // End bundle (atomic commit)

bundle+ { P }  -> BYTECODE (persistent)
├── RSPACE_BUNDLE_BEGIN RSPACE_STORE_CONC WRITE // Start write bundle in persistent RSpace
├── PUSH_PROC P                               // Push bundled process
├── EVAL_IN_BUNDLE                            // Evaluate in bundle context
└── RSPACE_BUNDLE_END RSPACE_STORE_CONC       // End bundle (atomic commit)
```

**Quote/Unquote**
Static quote (known process):
```
@P  -> BYTECODE
├── PUSH_PROC P                     // Push process P (with introspection form cached)
└── NOP                             // Quote is essentially a no-op - don't evaluate
```

Dynamic quote (free variable):
```
@x  -> BYTECODE (where x is free variable)
├── LOAD_VAR x                          // Load the variable x
└── NAME_QUOTE RSPACE_MEM_CONC          // Quote through memory RSpace
```
Unquote:
```
*name  -> BYTECODE
├── LOAD_VAR name                       // Load the name
└── NAME_UNQUOTE RSPACE_MEM_CONC        // Convert name to process
```

**Dynamic Quote with free variables**
```
@x  -> BYTECODE (where x is free variable)
├── LOAD_VAR x                          // Load the variable x
└── NAME_QUOTE RSPACE_MEM_CONC          // Quote through memory RSpace
```

**Variable Reference**
*Requires clarification from Jeff
```
=var  -> BYTECODE
├── LOAD_VAR var                        // Load variable
├── COPY                                // Create copy
└── REF RSPACE_MEM_CONC                 // Create reference through memory RSpace

=*var  -> BYTECODE
├── LOAD_VAR var                        // Load variable
├── MOVE                                // Transfer ownership
└── REF RSPACE_MEM_CONC                 // Create reference with move through memory RSpace
```

### Literal Constructs*
**I think the list of these constructs is supposed to be already loaded into Stack and so it will be an empty bytecode set**
```
integer (42) → BYTECODE

negative_int (-17) → BYTECODE

string ("hello") → BYTECODE

uri (`uri:example`) → BYTECODE

boolean_true (true) → BYTECODE

boolean_false (false) → BYTECODE

nil (Nil) → BYTECODE
```

For the sake of clarity, I translated all the Rholang constructs into bytecode.
But in reality I think that bytecode instructions will be reduced due to the use of Desugaring. For example:
```
contract Name(x) = { P }
```
↓ DESUGAR TO ↓
```
for (x <= Name) { P }
```
↓ BYTECODE ↓
```
├── LOAD_VAR Name
├── ALLOC_LOCAL
├── PATTERN_COMPILE RSPACE_STORE_CONC x
├── CONTINUATION_STORE RSPACE_STORE_CONC P
├── RSPACE_CONSUME_PERSISTENT RSPACE_STORE_CONC
├── PATTERN_BIND RSPACE_STORE_CONC
└── CONTINUATION_RESUME RSPACE_STORE_CONC
```

### Evaluation Semantics

- **Top-level expressions**: Always evaluated (e.g., `10 + 5` becomes `15`)
- **Process arguments**: Remain as suspended computations/closures
- **Pattern matching**: Works on introspection form, not evaluated values
- **Lazy evaluation**: Arguments evaluated only when needed

### Static Analysis-Driven Selection
```
Local Variable → Escape Analysis → RSpace Type Selection
    ↓                    ↓                    ↓
let x = 5     →    Doesn't escape    →   RSPACE_MEM_SEQ
new chan      →    Shared across     →   RSPACE_STORE_CONC
              →    processes         →
```

### Compilation Pipeline
```
Source Code → AST → Name Analysis → Escape Analysis → Bytecode Generation
                           ↓
                   RSpace Type Assignment
```

**Key Analysis Phases**
1. Name Usage Analysis:
   * Track where names are defined
   * Identify where names are used
   * Detect cross-process communication
2. Escape Analysis:
   * Does this name leave its lexical scope?
   * Is it passed to other processes?
   * Does it appear in top-level constructs?
3. Concurrency Analysis:
   * Are multiple threads accessing this name?
   * Is it used in parallel compositions (P | Q)?
   * Does it appear in concurrent let bindings (let x = P & y = Q)?