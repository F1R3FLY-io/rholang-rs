# Rholang Bytecode Design

For a complete and accurate implementation of bytecode, here we will describe a tentative structure of converting each Rholang instruction from grammar into bytecode. This bytecode design follows a stack-based VM where each Rholang instruction is translated into a sequence of low-level operations.

### Runtime System Instructions
```
CORE INSTRUCTIONS:
├── NOP                 // No operation
├── PUSH_INT n          // Push integer literal
├── PUSH_STR s          // Push string literal
├── PUSH_BOOL b         // Push boolean literal
├── POP                 // Pop top of stack
├── DUP                 // Duplicate top of stack
├── LOAD_VAR n          // Load variable by index
├── STORE_LOCAL n       // Store to local variable
├── ALLOC_LOCAL         // Allocate new local slot
├── ALLOC_NAME          // Allocate fresh name
├── EVAL                // Evaluate process on stack
├── EXEC                // Execute process on stack
├── QUOTE               // Process to name conversion
├── UNQUOTE             // Name to process conversion
├── SEND_ASYNC          // Asynchronous send
├── SEND_SYNC           // Synchronous send
├── RECEIVE             // Receive from channel
├── FORK                // Create parallel context
├── SPAWN               // Spawn process in new thread
├── JOIN_ALL            // Wait for all parallel processes
├── BRANCH_TRUE L       // Conditional jump if true
├── BRANCH_FALSE L      // Conditional jump if false
├── JUMP L              // Unconditional jump
├── CALL addr           // Call subroutine
├── CMP_EQ              // Equality comparison
├── CMP_NEQ             // Inequality comparison
├── CMP_LT              // Less than comparison
├── CMP_LTE             // Less than or equal
├── CMP_GT              // Greater than comparison
├── CMP_GTE             // Greater than or equal
├── CONCAT              // String/collection concatenation
├── DIFF                // Collection difference
├── INTERPOLATE         // String interpolation
├── NOT                 // Logical NOT
├── CONJ                // Process conjunction
├── DISJ                // Process disjunction
├── PROC_NEG            // Process negation
├── MATCH_TEST          // Pattern match test
├── COPY                // Copy value
├── MOVE                // Move value
├── REF                 // Create reference
├── TUPLE_BEGIN         // Start tuple construction
├── TUPLE_ADD           // Add element to tuple
└── TUPLE_END           // Finish tuple construction
```

### Core Process Constructs
**Parallel Composition (par)**
```
par (P | Q)  -> BYTECODE
├── EVAL_PROCESS P       // Evaluate P in current context
└── EVAL_PROCESS Q       // Evaluate Q in current context
```

**Name Creation (new)**
```
new x, y in P -> BYTECODE
├── ALLOC_NAME           // Allocate fresh name for x
├── STORE_LOCAL 0        // Store in local slot 0
├── ALLOC_NAME           // Allocate fresh name for y
├── STORE_LOCAL 1        // Store in local slot 1
├── PUSH_PROC P          // Push process P to stack
└── EXEC                 // Execute P with new scope
```

**Asynchronous Send (send)**
```
chan!(data)  -> BYTECODE
├── LOAD_VAR chan        // Load channel name
├── PUSH_PROC data       // Push data process
├── QUOTE                // Convert process to name (@data)
└── SEND_ASYNC           // Send message asynchronously
```

**Synchronous Send (send_sync)**
```
chan!?(data); P  -> BYTECODE
├── LOAD_VAR chan       // Load channel name
├── PUSH_PROC data      // Push data process
├── QUOTE               // Convert to name
├── SEND_SYNC           // Send and wait for ack
├── PUSH_PROC P         // Push continuation
└── EXEC                // Execute continuation
```

**Input/Receive (for)**
```
// Simple receive without pattern matching
for(x <- chan) P  -> BYTECODE
├── LOAD_VAR chan       // Load channel name
├── ALLOC_LOCAL         // Allocate slot for x
├── RECEIVE_SIMPLE      // Simple receive without pattern matching
├── STORE_LOCAL 0       // Store received value in x
├── PUSH_PROC P         // Push process P
└── EXEC                // Execute P with bound x

// Pattern matching receive
for(pattern <- chan) P  -> BYTECODE
├── LOAD_VAR chan
├── COMPILE_PATTERN pattern
├── RECEIVE_PATTERN         // Receive with pattern matching
├── EXTRACT_BINDINGS        // Extract all bound variables
├── PUSH_PROC P
└── EXEC
```

**Replicated Receive (contract)**
```
contract Name(x) = P  -> BYTECODE
├── LOAD_VAR Name           // Load contract name
├── COMPILE_PATTERN x       // Compile parameter pattern  
├── PUSH_PROC P             // Push contract body
├── RECEIVE_PATTERN true    // Same as for(), but with persist=true flag
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
```
select { x <- chan1 => P1; y <- chan2 => P2 }  -> BYTECODE
├── SELECT_BEGIN        // Start atomic select operation
├── LOAD_VAR chan1      // Load channel 1
├── PREPARE_CHOICE 0    // Prepare choice 0 (but don't commit yet)
├── LOAD_VAR chan2      // Load channel 2  
├── PREPARE_CHOICE 1    // Prepare choice 1 (but don't commit yet)
├── ATOMIC_SELECT       // Atomically wait for ANY channel to be ready
├── CANCEL_OTHERS       // Cancel all non-selected choices
├── BRANCH_CHOICE 0 L1  // If choice 0 was selected, jump to L1
├── BRANCH_CHOICE 1 L2  // If choice 1 was selected, jump to L2
├── L1: ALLOC_LOCAL     // Allocate for x
├── RECEIVE_SELECTED    // Receive from the selected channel
├── STORE_LOCAL 0       // Store received value in x
├── PUSH_PROC P1        // Push process P1
├── EXEC                // Execute P1
├── JUMP L_END          // Jump to end
├── L2: ALLOC_LOCAL     // Allocate for y
├── RECEIVE_SELECTED    // Receive from the selected channel
├── STORE_LOCAL 0       // Store received value in y
├── PUSH_PROC P2        // Push process P2
├── EXEC                // Execute P2
└── L_END: NOP          // Continue
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
├── PUSH_PROC obj       // Push receiver object
├── EVAL                // Evaluate receiver
├── PUSH_PROC args      // Push arguments
├── EVAL                // Evaluate arguments
├── LOAD_METHOD method  // Load method name
└── INVOKE              // Invoke method
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
├── PUSH_CONTEXT LOCAL  // Push local evaluation context
├── PUSH_PROC P         // Push value for x
├── EVAL_WITH_CONTEXT   // Evaluate P with provided context
├── ALLOC_LOCAL         // Allocate slot for x
├── STORE_LOCAL 0       // Store P result in x
├── PUSH_PROC Q         // Push value for y
├── EVAL_WITH_CONTEXT   // Evaluate Q with provided context
├── ALLOC_LOCAL         // Allocate slot for y
├── STORE_LOCAL 1       // Store Q result in y
├── PUSH_PROC R         // Push body process R
├── EVAL_WITH_CONTEXT   // Evaluate R with local bindings
└── POP_CONTEXT         // Restore previous context
```

**Let Binding (Concurrent)**
```
let x = P & y = Q in R  -> BYTECODE
├── CREATE_CONCURRENT_LOCAL_CONTEXT   // Create context for concurrent local eval
├── PUSH_PROC P                       // Push process P
├── PUSH_CONTINUATION x               // Push continuation for x binding
├── SPAWN_IN_LOCAL_CONTEXT            // Spawn P evaluation in local context
├── PUSH_PROC Q                       // Push process Q
├── PUSH_CONTINUATION y               // Push continuation for y binding  
├── SPAWN_IN_LOCAL_CONTEXT            // Spawn Q evaluation in local context
├── WAIT_LOCAL_CONTEXT                // Wait for all local evaluations
├── PUSH_PROC R                       // Push body process R
└── EXEC_WITH_LOCAL_BINDINGS          // Execute R with accumulated bindings
```

### Data Constructs
**List Construction**
```
[P, Q, ...R]  -> BYTECODE
├── LIST_BEGIN          // Start list construction
├── PUSH_PROC P         // Push first element
├── EVAL                // Evaluate P
├── LIST_ADD            // Add to list
├── PUSH_PROC Q         // Push second element
├── EVAL                // Evaluate Q
├── LIST_ADD            // Add to list
├── PUSH_PROC R         // Push remainder
├── EVAL                // Evaluate remainder
├── LIST_SPREAD         // Spread remainder into list
└── LIST_END            // Finish list construction
```

**Map Construction**
```
{key1: val1, key2: val2}  -> BYTECODE
├── MAP_BEGIN           // Start map construction
├── PUSH_PROC key1      // Push first key
├── EVAL                // Evaluate key1
├── PUSH_PROC val1      // Push first value
├── EVAL                // Evaluate val1
├── MAP_PUT             // Add key-value pair
├── PUSH_PROC key2      // Push second key
├── EVAL                // Evaluate key2
├── PUSH_PROC val2      // Push second value
├── EVAL                // Evaluate val2
├── MAP_PUT             // Add key-value pair
└── MAP_END             // Finish map construction
```

**Single Element Tuple**
```
(P,)  -> BYTECODE
├── TUPLE_BEGIN         // Start tuple construction
├── PUSH_PROC P         // Push element
├── EVAL                // Evaluate element
├── TUPLE_ADD           // Add to tuple
└── TUPLE_END           // Finish single-element tuple
```

**Multi-Element Tuple**
```
(P, Q, R)  -> BYTECODE
├── TUPLE_BEGIN         // Start tuple construction
├── PUSH_PROC P         // Push first element
├── EVAL                // Evaluate P
├── TUPLE_ADD           // Add to tuple
├── PUSH_PROC Q         // Push second element
├── EVAL                // Evaluate Q
├── TUPLE_ADD           // Add to tuple
├── PUSH_PROC R         // Push third element
├── EVAL                // Evaluate R
├── TUPLE_ADD           // Add to tuple
└── TUPLE_END           // Finish tuple construction
```

### Advanced Constructs
**Bundle Operations**
```
bundle+ { P }  -> BYTECODE
├── BUNDLE_BEGIN WRITE  // Start write bundle
├── PUSH_PROC P         // Push bundled process
├── EXEC                // Execute in bundle context
└── BUNDLE_END          // End bundle
```

**Quote/Unquote**
```
@P  -> BYTECODE
├── PUSH_PROC P         // Push process P
└── QUOTE               // Convert process to name

*name  -> BYTECODE
├── LOAD_VAR name       // Load name
└── UNQUOTE             // Convert name to process
```

**Variable Reference**
*Requires clarification from Jeff
```
=var  -> BYTECODE
├── LOAD_VAR var        // Load variable
├── COPY                // Create copy
└── REF                 // Create reference to copy

=*var  -> BYTECODE
├── LOAD_VAR var        // Load variable
├── MOVE                // Transfer ownership
└── REF                 // Create reference with move
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
├── RECEIVE_PERSISTENT
├── STORE_LOCAL 0
├── PUSH_PROC P
└── EXEC
```