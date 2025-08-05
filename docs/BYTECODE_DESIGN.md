# Rholang Bytecode Design

For a complete and accurate implementation of bytecode, here we will describe a tentative structure of converting each Rholang instruction from grammar into bytecode.

## Theoretical Foundation

### VM Specification as Graph-Structured Lambda Theory (GSLT)
The VM specification must be representable as a graph-structured lambda theory to ensure internal consistency. This requires:

- **Fully Abstract Encoding**: A complete mapping from RSpace states to VM states that preserves all behavioral properties
- **Provable Bisimilarity**: For any RSpace states R1 and R2, if R1 ≈ R2 (bisimilar), then [[R1]] ≈ [[R2]] (their VM translations are also bisimilar)
- **Parametric Bisimulation**: The bisimilarity notion is parametric in the appropriate form of bisimulation (potentially weak bisimulation in presence of stack operations)

### RSpace State Interpretation
An RSpace state corresponds to a row calculus expression composed of:
- Four comprehensions in parallel with outputs
- No internal interactions (redexes) - the state is quiescent
- Can be interpreted as collections of parked logical threads where:
    - Run method for input threads: performs RSpace.get operations
    - Run method for output threads: performs RSpace.put operations
    - No comma events generated when no matches are found

### Constrained VM Design
The VM shape is highly constrained by the requirements to:
- Handle primitive operations (string ops, arithmetic ops) available on ambient hardware
- Dispatch on codes for row calculus constructs (par, for, chan)
- Maintain bisimilarity with RSpace semantics
- Support the interpretation function requirements

## Architecture Overview
- **Mini-Stack VM**: Handles computational operations, control flow, local variables
- **RSpace Integration**: VM communicates with appropriate RSpace through typed instructions
    - `RSPACE_MEM_SEQ`: In-memory sequential (hashmap) - single-threaded, local operations
    - `RSPACE_MEM_CONC`: In-memory concurrent (concurrent hashmap) - multi-threaded, local operations
    - `RSPACE_STORE_SEQ`: On-store sequential (LMDB wrapper) - single-threaded, persistent
    - `RSPACE_STORE_CONC`: On-store concurrent (LMDB wrapper) - multi-threaded, persistent, blockchain operations
- **State Representation**: VM state must be representable as GSLT with fully abstract encoding

## Compilation Pipeline
### Three-Stage Architecture
The compilation follows a structured three-stage pipeline:

```
Rholang 1.2 → Rholang 1.1 → Rholang 1.0 (core) → Bytecode
     ↓              ↓              ↓
High-level     RSpace       Core row
desugaring     dispatch     calculus
```

**Stage Responsibilities**:
- **Rholang 1.2 → 1.1**: High-level desugaring, RSpace type determination based on whole expression context
- **Rholang 1.1**: Handles dispatch to different RSpaces, includes four RSpace parameters at the top level
- **Rholang 1.0**: Core row calculus operations, primitive operations - always hits `RSPACE_STORE_CONC` by default

### RSpace Type Selection Strategy
RSpace type selection must consider the **whole expression context**, not just channel properties:
**Example Analysis**:
```rholang
let x = <expression> in (P1(x) | P2(x))
```
The parallel composition (`|`) in the continuation determines that `x` requires concurrent access in memory (`RSPACE_MEM_CONC`), regardless of channel properties.

#### Static Analysis Requirements
The compiler must perform static analysis to determine:

1. **Concurrent vs Sequential Access**:
    - Count how many subprocesses a name is used in
    - Detect parallel compositions where names are shared
    - Analyze let bindings (sequential vs concurrent)

2. **Escape Analysis**:
    - Does the name escape its lexical scope?
    - Is it passed to other processes?
    - Does it appear in top-level constructs?

3. **Expression-Level Analysis**:
    - `let sequential`: Uses `RSPACE_MEM_SEQ` or `RSPACE_STORE_SEQ`
    - `let concurrent` (with `&`): Uses `RSPACE_MEM_CONC` or `RSPACE_STORE_CONC`
    - Parallel compositions: Force concurrent RSpace types
    - Top-level expressions: Default to `RSPACE_STORE_CONC`

### Example Format
All examples should follow this format with four RSpace parameters at the top level:
```
Interpreter(
  RSPACE_MEM_SEQ: mem_seq_instance,
  RSPACE_MEM_CONC: mem_conc_instance, 
  RSPACE_STORE_SEQ: store_seq_instance,
  RSPACE_STORE_CONC: store_conc_instance,
  Expression: [[rholang_1.1_expression]]
)
```

## Evaluation Semantics
### Lazy Evaluation with Explicit Stars
Based on Rholang 1.3 design principles:
- **Default Behavior**: Expressions remain frozen until explicitly evaluated
- **Star Syntax**: Only expressions marked with `*` are evaluated immediately
- **Environment Efficiency**: Avoids copying environments when expressions contain local variables
- **Performance Trade-off**: Developer controls when evaluation happens vs when environment copying occurs

### Top-level vs Process Expression Evaluation
- **Top-level expressions**: Can be evaluated at compile time (e.g., `10 + 5` → `15`)
- **Process arguments**: Remain as suspended computations unless starred
- **Pattern matching**: Works on introspection form, not evaluated values
- **Local variables**: Evaluation strategy affects environment copying costs

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
├── EVAL_STAR           // Explicit evaluation (star syntax)
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

### RSpace Type Selection Implementation
The `<type>` parameter is determined by combining:
1. **Explicit flags**: Override any channel properties for specific instructions
2. **Channel properties**: RSpace remembers properties when names are created
3. **Expression context**: Analysis of the whole expression, especially parallel compositions


### Real rholang code to bytecode transformation examples
**Name Creation**

***Top-level Contract Names***
```rholang
new x, y in P  // Top level
```
→ **Analysis**: Top-level scope, potentially shared across processes, must survive restarts
→ **RSpace**: `RSPACE_STORE_CONC`
```
├── NAME_CREATE RSPACE_STORE_CONC     // x goes to persistent concurrent
├── STORE_LOCAL 0                     
├── NAME_CREATE RSPACE_STORE_CONC     // y goes to persistent concurrent
├── STORE_LOCAL 1                     
├── PUSH_PROC P                       
└── EVAL
```

***Local Names with Concurrent Access***
```rholang
let x = <expression> in (P1(x) | P2(x))  // Parallel composition using x
```
→ **Analysis**: Local scope but parallel access in continuation
→ **RSpace**: `RSPACE_MEM_CONC`
```
├── NAME_CREATE RSPACE_MEM_CONC       // Concurrent due to parallel composition
├── STORE_LOCAL 0                     
├── PUSH_PROC (P1(x) | P2(x))         
└── EVAL
```

***Sequential Local Names***
```rholang
let x = <expression> in P(x)  // Sequential access only
```
→ **Analysis**: Local scope, sequential access
→ **RSpace**: `RSPACE_MEM_SEQ`
```
├── NAME_CREATE RSPACE_MEM_SEQ        // Sequential is sufficient
├── STORE_LOCAL 0                     
├── PUSH_PROC P(x)                    
└── EVAL
```

**Send Operation**
***Top-level Channel with Lazy Evaluation***
```rholang
chan!(complex_process)  // No star - keep as closure
```
→ **Analysis**: Top-level channel, process not evaluated
→ **RSpace**: `RSPACE_STORE_CONC`
```
├── PUSH_PROC complex_process          // Suspended computation
├── PUSH_PROC chan                     
├── EVAL                               // Evaluate channel only
└── RSPACE_PRODUCE RSPACE_STORE_CONC
```

***Local Channel with Explicit Evaluation***
```rholang
localChan!(*arithmetic_expr)  // Star forces evaluation
```
→ **Analysis**: Local channel, expression explicitly evaluated
→ **RSpace**: `RSPACE_MEM_CONC`
```
├── PUSH_PROC arithmetic_expr          
├── EVAL_STAR                          // Explicit evaluation due to star
├── PUSH_PROC localChan                
├── EVAL                               
└── RSPACE_PRODUCE RSPACE_MEM_CONC
```

**Receive Operation**
***Contract Reception (Persistent)***
```rholang
for(x <- publicChannel) P  // Contract parameter
```
→ **Analysis**: Contract context, public channel shared across deployments
→ **RSpace**: `RSPACE_STORE_CONC`
```
├── LOAD_VAR publicChannel             
├── ALLOC_LOCAL                        
├── PATTERN_COMPILE RSPACE_STORE_CONC x
├── CONTINUATION_STORE RSPACE_STORE_CONC P
├── RSPACE_CONSUME RSPACE_STORE_CONC   
├── PATTERN_BIND RSPACE_STORE_CONC     
└── CONTINUATION_RESUME RSPACE_STORE_CONC
```

***Local Scope Reception***
```rholang
{ new local in { for(x <- local) P } }  // Nested local scope
```
→ **Analysis**: Local scope, confined to current process
→ **RSpace**: `RSPACE_MEM_CONC`
```
├── LOAD_VAR local                     
├── ALLOC_LOCAL                        
├── PATTERN_COMPILE RSPACE_MEM_CONC x  
├── CONTINUATION_STORE RSPACE_MEM_CONC P
├── RSPACE_CONSUME RSPACE_MEM_CONC     
├── PATTERN_BIND RSPACE_MEM_CONC       
└── CONTINUATION_RESUME RSPACE_MEM_CONC
```

**Let Binding**
***Concurrent Let with Expression Context***
```rholang
let x = P & y = Q in R  // Concurrent binding
```
→ **Analysis**: Concurrent binding operator (&) determines RSpace type
→ **RSpace**: `RSPACE_MEM_CONC` for coordination
```
├── NAME_CREATE RSPACE_MEM_CONC                 // Coordination channel
├── STORE_LOCAL 0                               
├── PUSH_PROC P                                 
├── CONTINUATION_STORE RSPACE_MEM_CONC x_binding
├── SPAWN_ASYNC RSPACE_MEM_CONC                 
├── PUSH_PROC Q                                 
├── CONTINUATION_STORE RSPACE_MEM_CONC y_binding
├── SPAWN_ASYNC RSPACE_MEM_CONC                 
├── RSPACE_CONSUME RSPACE_MEM_CONC              // Wait for x
├── RSPACE_CONSUME RSPACE_MEM_CONC              // Wait for y
├── PUSH_PROC R                                 
└── EVAL
```

***Sequential Let***
```rholang
let x = P; y = Q in R  // Sequential binding
```
→ **Analysis**: Sequential binding, stack-based evaluation sufficient
→ **RSpace**: Stack operations, no RSpace needed for coordination
```
├── PUSH_PROC P         
├── EVAL                // Stack-based evaluation
├── ALLOC_LOCAL         
├── STORE_LOCAL 0       
├── PUSH_PROC Q         
├── EVAL                
├── ALLOC_LOCAL         
├── STORE_LOCAL 1       
├── PUSH_PROC R         
└── EVAL
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
```rholang
bundle+ { P }  // Bundle context determines RSpace
```
***Memory-based bundle***:
```
├── RSPACE_BUNDLE_BEGIN RSPACE_MEM_CONC WRITE
├── PUSH_PROC P                             
├── EVAL_IN_BUNDLE                          
└── RSPACE_BUNDLE_END RSPACE_MEM_CONC
```

***Persistent bundle***:
```
├── RSPACE_BUNDLE_BEGIN RSPACE_STORE_CONC WRITE
├── PUSH_PROC P                               
├── EVAL_IN_BUNDLE                            
└── RSPACE_BUNDLE_END RSPACE_STORE_CONC
```

**Quote/Unquote**
***Static Quote (Compile-time)***
```rholang
@{known_process}  // Static analysis can determine
```
```
├── PUSH_PROC known_process     // Introspection form cached
└── NOP                         // No evaluation needed
```

***Dynamic Quote (Runtime)***
```rholang
@x  // Where x is free variable
```
```
├── LOAD_VAR x                          
└── NAME_QUOTE RSPACE_MEM_CONC          // Context-dependent RSpace
```

***Unquote***
```rholang
*name
```
```
├── LOAD_VAR name                       
└── NAME_UNQUOTE RSPACE_MEM_CONC        // Match quote context
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
