# Rholang Bytecode Design with PathMap Architecture

This document describes a PathMap-based bytecode design.

## PathMap Runtime System Instructions
```
PATHMAP CORE INSTRUCTIONS:
├── PATH_ALLOC              // Allocate new execution path
├── PATH_FORK n             // Fork current path into n child paths
├── PATH_JOIN paths         // Join multiple paths into current path
├── PATH_SYNC paths         // Synchronize multiple paths at barrier
├── PATH_BIND var path      // Bind variable to specific path
├── PATH_LOAD var path      // Load variable from path context
├── PATH_STORE var path     // Store variable in path context
├── PATH_EXEC path          // Execute process in specific path
├── PATH_SPAWN path         // Spawn process in new path thread
├── PATH_ROUTE chan paths   // Route message across paths
├── PATH_LISTEN chan paths  // Listen on channel across paths
├── PATH_MULTI_LISTEN chans // Listen on multiple channels
├── PATH_BARRIER paths      // Create synchronization barrier
├── PATH_CLEANUP path       // Clean up finished path
├── PATH_PERSIST path       // Make path persistent
├── PATH_REGISTER name path // Register named handler in path
├── PATH_SET_BUNDLE_MODE path mode // Set bundle mode for path
├── PATH_CLEAR_BUNDLE_MODE path // Clear bundle mode
├── PATH_EVAL path          // Evaluate expression in path
├── PATH_EVAL_BOOL path     // Evaluate boolean in path
├── PATH_QUOTE path         // Quote process in path context
├── PATH_UNQUOTE path       // Unquote name in path context
├── PATH_COPY path          // Copy value within path
├── PATH_MOVE path          // Move value between paths
├── PATH_UPDATE_RESULT path // Update path with operation result
├── PATH_UPDATE_STATE path  // Update path state
├── PATH_MATCH_TEST path    // Test pattern match in path
├── PATH_BIND_PATTERN pat path // Bind pattern variables to path
├── PATH_STORE_RECEIVED path // Store received message in path
├── PATH_INVOKE path        // Invoke method in path context
├── PATH_CONJ_BARRIER paths // Conjunction barrier (all must succeed)
├── PATH_DISJ_RACE paths    // Disjunction race (any can succeed)
├── PATH_SELECT_WINNER      // Select winning path from race
├── PATH_CLEANUP_LOSERS     // Clean up non-winning paths
├── PATH_NEGATE path        // Negate path result
└── PATH_MERGE_BINDINGS path // Merge variable bindings into path
```

## Core Process Constructs
**Parallel Composition (par)**
```
par (P | Q)  -> BYTECODE
├── PATH_ALLOC              // Allocate parent path
├── PATH_FORK 2             // Fork into 2 child paths
├── PATH_BIND left_path 0   // Bind left process to path 0
├── PATH_BIND right_path 1  // Bind right process to path 1
├── PUSH_PROC P             // Push left process
├── PATH_SPAWN left_path    // Spawn P in left path
├── PUSH_PROC Q             // Push right process  
├── PATH_SPAWN right_path   // Spawn Q in right path
├── PATH_BARRIER [0,1]      // Create barrier for paths 0,1
├── PATH_JOIN [0,1]         // Join child paths
└── PATH_CLEANUP parent     // Clean up parent path
```

**Name Creation (new)**
```
new x, y in P  -> BYTECODE
├── PATH_ALLOC              // Allocate scope path
├── ALLOC_NAME              // Allocate fresh name for x
├── PATH_STORE x scope_path // Store x in scope path
├── ALLOC_NAME              // Allocate fresh name for y
├── PATH_STORE y scope_path // Store y in scope path
├── PUSH_PROC P             // Push process P
├── PATH_EXEC scope_path    // Execute P within scope path
└── PATH_CLEANUP scope_path // Clean up scope
```

**Asynchronous Send (send)**
```
chan!(data)  -> BYTECODE
├── PATH_LOAD chan path     // Load channel from current path
├── PUSH_PROC data          // Push data process
├── QUOTE                   // Convert process to name (@data)
├── PATH_ROUTE chan paths   // Route message across relevant paths
├── SEND_ASYNC              // Send message asynchronously
└── PATH_UPDATE_STATE       // Update path state after send
```

**Synchronous Send (send_sync)**
```
chan!?(data); P  -> BYTECODE
├── PATH_LOAD chan path     // Load channel from current path
├── PUSH_PROC data          // Push data process
├── QUOTE                   // Convert to name
├── PATH_ALLOC              // Allocate continuation path
├── PATH_ROUTE chan paths   // Route message across paths
├── SEND_SYNC               // Send and wait for ack
├── PATH_SYNC cont_path     // Synchronize on continuation path
├── PUSH_PROC P             // Push continuation
├── PATH_EXEC cont_path     // Execute continuation in path
└── PATH_CLEANUP cont_path  // Clean up continuation path
```

**Input/Receive (for)**
```
for(x <- chan) P  -> BYTECODE
├── PATH_LOAD chan path     // Load channel from current path
├── PATH_ALLOC              // Allocate receive path
├── PATH_BIND x recv_path   // Bind x to receive path
├── PATH_LISTEN chan paths  // Listen on channel across paths
├── RECEIVE                 // Block until message available
├── PATH_STORE x recv_path  // Store received value in path
├── PUSH_PROC P             // Push process P
├── PATH_EXEC recv_path     // Execute P within receive path
└── PATH_CLEANUP recv_path  // Clean up receive path
```

**Replicated Receive (contract)**
```
contract Name(x) = P  -> BYTECODE
├── PATH_LOAD Name path       // Load contract name from path
├── PATH_ALLOC                // Allocate handler path
├── PATH_BIND x handler_path  // Bind parameter to handler path
├── CREATE_HANDLER            // Create persistesnt message handler
├── PATH_PERSIST handler_path // Make handler path persistent
├── PUSH_PROC P               // Push contract body
├── PATH_BIND_HANDLER handler_path // Bind handler to path
└── PATH_REGISTER Name handler_path // Register contract in path map
```

### Control Flow Constructs
**Conditional (if-else)**
```
if (cond) P else Q  -> BYTECODE
├── PUSH_PROC cond          // Push condition
├── EVAL_BOOL               // Evaluate to boolean
├── PATH_ALLOC              // Allocate branch path
├── BRANCH_FALSE L1         // Jump to L1 if false
├── PATH_FORK then_path     // Fork path for then branch
├── PUSH_PROC P             // Push then-branch
├── PATH_EXEC then_path     // Execute P in then path
├── JUMP L2                 // Skip else-branch
├── L1: PATH_FORK else_path // Fork path for else branch
├── PUSH_PROC Q             // Push else-branch
├── PATH_EXEC else_path     // Execute Q in else path
├── L2: PATH_JOIN [then_path, else_path] // Join branch paths
└── PATH_CLEANUP branch_path // Clean up branch path
```

**Pattern Matching (match)**
```
match expr { pat1 => P1; pat2 => P2 }  -> BYTECODE
├── PUSH_PROC expr          // Push expression to match
├── EVAL                    // Evaluate expression
├── PATH_ALLOC              // Allocate match path
├── MATCH_BEGIN             // Start pattern matching
├── PATTERN pat1            // Try pattern 1
├── BRANCH_NOMATCH L1       // Jump if no match
├── PATH_FORK match1_path   // Fork path for match 1
├── PATH_BIND_PATTERN pat1 match1_path // Bind pattern vars to path
├── PUSH_PROC P1            // Push body 1
├── PATH_EXEC match1_path   // Execute P1 in match path
├── JUMP L_END              // Jump to end
├── L1: PATTERN pat2        // Try pattern 2
├── BRANCH_NOMATCH L2       // Jump if no match
├── PATH_FORK match2_path   // Fork path for match 2
├── PATH_BIND_PATTERN pat2 match2_path // Bind pattern vars to path
├── PUSH_PROC P2            // Push body 2
├── PATH_EXEC match2_path   // Execute P2 in match path
├── JUMP L_END              // Jump to end
├── L2: MATCH_FAIL          // No patterns matched
├── L_END: PATH_JOIN [match1_path, match2_path] // Join match paths
└── PATH_CLEANUP match_path // Clean up match path
```

**Select/Choice (select)**
```
select { x <- chan1 => P1; y <- chan2 => P2 }  -> BYTECODE
├── PATH_ALLOC              // Allocate select path
├── SELECT_BEGIN            // Start select operation
├── PATH_LOAD chan1 path    // Load channel 1 from path
├── ADD_CHOICE 0            // Add to choice set with index 0
├── PATH_LOAD chan2 path    // Load channel 2 from path
├── ADD_CHOICE 1            // Add to choice set with index 1
├── PATH_MULTI_LISTEN [chan1, chan2] // Listen on multiple channels
├── SELECT_WAIT             // Wait for any channel to be ready
├── BRANCH_CHOICE 0 L1      // If choice 0, jump to L1
├── BRANCH_CHOICE 1 L2      // If choice 1, jump to L2
├── L1: PATH_FORK choice1_path // Fork path for choice 1
├── PATH_BIND x choice1_path // Bind x to choice path
├── PATH_STORE_RECEIVED choice1_path // Store received value
├── PUSH_PROC P1            // Push process P1
├── PATH_EXEC choice1_path  // Execute P1 in choice path
├── JUMP L_END              // Jump to end
├── L2: PATH_FORK choice2_path // Fork path for choice 2
├── PATH_BIND y choice2_path // Bind y to choice path
├── PATH_STORE_RECEIVED choice2_path // Store received value
├── PUSH_PROC P2            // Push process P2
├── PATH_EXEC choice2_path  // Execute P2 in choice path
├── L_END: PATH_JOIN [choice1_path, choice2_path] // Join choice paths
└── PATH_CLEANUP select_path // Clean up select path
```

### Expression Constructs
**Arithmetic Operations (only addition)**
```
P + Q  -> BYTECODE
├── PUSH_PROC P             // Push left operand
├── PATH_EVAL current_path  // Evaluate P in current path
├── PUSH_PROC Q             // Push right operand
├── PATH_EVAL current_path  // Evaluate Q in current path
├── ADD                     // Perform addition
└── PATH_UPDATE_RESULT current_path // Update path with result
```

**Logical Operations**
```
P and Q  -> BYTECODE
├── PUSH_PROC P             // Push left operand
├── PATH_EVAL_BOOL current_path // Evaluate to boolean in path
├── DUP                     // Duplicate result
├── BRANCH_FALSE L1         // Short-circuit if false
├── POP                     // Remove duplicate
├── PUSH_PROC Q             // Push right operand
├── PATH_EVAL_BOOL current_path // Evaluate to boolean in path
├── L1: PATH_UPDATE_RESULT current_path // Update path with result
└── NOP                     // Result is on stack
```

**Method Call**
```
obj.method(args)  -> BYTECODE
├── PUSH_PROC obj           // Push receiver object
├── PATH_EVAL current_path  // Evaluate receiver in path
├── PUSH_PROC args          // Push arguments
├── PATH_EVAL current_path  // Evaluate arguments in path
├── LOAD_METHOD method      // Load method name
├── PATH_INVOKE current_path // Invoke method in path context
└── PATH_UPDATE_RESULT current_path // Update path with result
```

### Comparison Expression Constructs
**Equality Comparison**
```
P == Q  -> BYTECODE
├── PUSH_PROC P             // Push left operand
├── PATH_EVAL current_path  // Evaluate P in current path
├── PUSH_PROC Q             // Push right operand
├── PATH_EVAL current_path  // Evaluate Q in current path
├── CMP_EQ                  // Compare for equality
└── PATH_UPDATE_RESULT current_path // Update path with boolean result
```

**All Other Comparisons (!=, <, <=, >, >=)**
```
P <op> Q  -> BYTECODE
├── PUSH_PROC P             // Push left operand
├── PATH_EVAL current_path  // Evaluate P in current path
├── PUSH_PROC Q             // Push right operand
├── PATH_EVAL current_path  // Evaluate Q in current path
├── CMP_<OP>                // Perform comparison operation
└── PATH_UPDATE_RESULT current_path // Update path with boolean result
```

## Variable binding constructs
**Let Binding (Linear)**
```
let x = P; y = Q in R  -> BYTECODE
├── PATH_ALLOC              // Allocate binding path
├── PUSH_PROC P             // Push value for x
├── PATH_EVAL binding_path  // Evaluate P in binding path
├── PATH_BIND x binding_path // Bind x to binding path
├── PATH_STORE x binding_path // Store P result in path
├── PUSH_PROC Q             // Push value for y
├── PATH_EVAL binding_path  // Evaluate Q in binding path
├── PATH_BIND y binding_path // Bind y to binding path
├── PATH_STORE y binding_path // Store Q result in path
├── PUSH_PROC R             // Push body process R
├── PATH_EXEC binding_path  // Execute R within binding path
└── PATH_CLEANUP binding_path // Clean up binding path
```

**Let Binding (Concurrent)**
```
let x = P & y = Q in R  -> BYTECODE
├── PATH_ALLOC              // Allocate parent binding path
├── PATH_FORK 2             // Fork into 2 child paths for concurrent bindings
├── PATH_BIND x_path 0      // Bind x binding to path 0
├── PATH_BIND y_path 1      // Bind y binding to path 1
├── PUSH_PROC P             // Push value for x
├── PATH_SPAWN x_path       // Spawn P evaluation in x path
├── PATH_EVAL x_path        // Evaluate P in x path
├── PATH_STORE x x_path     // Store result in x path
├── PUSH_PROC Q             // Push value for y
├── PATH_SPAWN y_path       // Spawn Q evaluation in y path
├── PATH_EVAL y_path        // Evaluate Q in y path
├── PATH_STORE y y_path     // Store result in y path
├── PATH_BARRIER [x_path, y_path] // Wait for both bindings
├── PATH_JOIN [x_path, y_path] // Join binding paths
├── PATH_MERGE_BINDINGS binding_path // Merge all bindings to main path
├── PUSH_PROC R             // Push body process R
├── PATH_EXEC binding_path  // Execute R with all bindings
└── PATH_CLEANUP binding_path // Clean up binding path
```

### Data Constructs
**List Construction**
```
[P, Q, ...R]  -> BYTECODE
├── PATH_ALLOC              // Allocate list construction path
├── LIST_BEGIN              // Start list construction
├── PUSH_PROC P             // Push first element
├── PATH_EVAL list_path     // Evaluate P in list path
├── LIST_ADD                // Add to list
├── PUSH_PROC Q             // Push second element
├── PATH_EVAL list_path     // Evaluate Q in list path
├── LIST_ADD                // Add to list
├── PUSH_PROC R             // Push remainder
├── PATH_EVAL list_path     // Evaluate remainder in list path
├── LIST_SPREAD             // Spread remainder into list
├── LIST_END                // Finish list construction
├── PATH_UPDATE_RESULT list_path // Update path with list result
└── PATH_CLEANUP list_path  // Clean up list construction path
```

**Map Construction**
```
{key1: val1, key2: val2}  -> BYTECODE
├── PATH_ALLOC              // Allocate map construction path
├── MAP_BEGIN               // Start map construction
├── PUSH_PROC key1          // Push first key
├── PATH_EVAL map_path      // Evaluate key1 in map path
├── PUSH_PROC val1          // Push first value
├── PATH_EVAL map_path      // Evaluate val1 in map path
├── MAP_PUT                 // Add key-value pair
├── PUSH_PROC key2          // Push second key
├── PATH_EVAL map_path      // Evaluate key2 in map path
├── PUSH_PROC val2          // Push second value
├── PATH_EVAL map_path      // Evaluate val2 in map path
├── MAP_PUT                 // Add key-value pair
├── MAP_END                 // Finish map construction
├── PATH_UPDATE_RESULT map_path // Update path with map result
└── PATH_CLEANUP map_path   // Clean up map construction path
```

**Multi-Element Tuple**
```
(P, Q, R)  -> BYTECODE
├── PATH_ALLOC              // Allocate tuple construction path
├── TUPLE_BEGIN             // Start tuple construction
├── PUSH_PROC P             // Push first element
├── PATH_EVAL tuple_path    // Evaluate P in tuple path
├── TUPLE_ADD               // Add to tuple
├── PUSH_PROC Q             // Push second element
├── PATH_EVAL tuple_path    // Evaluate Q in tuple path
├── TUPLE_ADD               // Add to tuple
├── PUSH_PROC R             // Push third element
├── PATH_EVAL tuple_path    // Evaluate R in tuple path
├── TUPLE_ADD               // Add to tuple
├── TUPLE_END               // Finish tuple construction
├── PATH_UPDATE_RESULT tuple_path // Update path with tuple result
└── PATH_CLEANUP tuple_path // Clean up tuple construction path
```

### Advanced Constructs
**Bundle Operations**
```
bundle+ { P }  -> BYTECODE
├── PATH_ALLOC              // Allocate bundle path
├── BUNDLE_BEGIN WRITE      // Start write bundle
├── PATH_SET_BUNDLE_MODE bundle_path WRITE // Set path to write mode
├── PUSH_PROC P             // Push bundled process
├── PATH_EXEC bundle_path   // Execute in bundle context
├── BUNDLE_END              // End bundle
├── PATH_CLEAR_BUNDLE_MODE bundle_path // Clear bundle mode
└── PATH_CLEANUP bundle_path // Clean up bundle path
```

**Quote/Unquote**
```
@P  -> BYTECODE
├── PUSH_PROC P             // Push process P
├── PATH_QUOTE current_path // Quote process in current path context
└── PATH_UPDATE_RESULT current_path // Update path with quoted name

*name  -> BYTECODE
├── PATH_LOAD name path     // Load name from path
├── PATH_UNQUOTE current_path // Unquote name in current path context
└── PATH_UPDATE_RESULT current_path // Update path with process result
```

**Variable Reference**
```
=var  -> BYTECODE
├── PATH_LOAD var path      // Load variable from its path
├── PATH_COPY current_path  // Create copy in current path
├── REF                     // Create reference to copy
└── PATH_UPDATE_RESULT current_path // Update current path with reference

=*var  -> BYTECODE
├── PATH_LOAD var path      // Load variable from its path
├── PATH_MOVE current_path  // Transfer ownership to current path
├── REF                     // Create reference with move
└── PATH_UPDATE_RESULT current_path // Update current path with reference
```

### String/Collection Operations
**String Concatenation**
```
P ++ Q  -> BYTECODE
├── PUSH_PROC P             // Push left operand
├── PATH_EVAL current_path  // Evaluate P in current path
├── PUSH_PROC Q             // Push right operand
├── PATH_EVAL current_path  // Evaluate Q in current path
├── CONCAT                  // Concatenate strings/collections
└── PATH_UPDATE_RESULT current_path // Update path with result
```

**Collection Difference**
```
P -- Q  -> BYTECODE
├── PUSH_PROC P             // Push left operand
├── PATH_EVAL current_path  // Evaluate P in current path
├── PUSH_PROC Q             // Push right operand
├── PATH_EVAL current_path  // Evaluate Q in current path
├── DIFF                    // Collection difference operation
└── PATH_UPDATE_RESULT current_path // Update path with result
```

**String Interpolation**
```
P %% Q  -> BYTECODE
├── PUSH_PROC P             // Push format string
├── PATH_EVAL current_path  // Evaluate P in current path
├── PUSH_PROC Q             // Push value to interpolate
├── PATH_EVAL current_path  // Evaluate Q in current path
├── INTERPOLATE             // Perform string interpolation
└── PATH_UPDATE_RESULT current_path // Update path with result
```

## Logical/Pattern Constructs
**Matches Expression**
```
P matches Q  -> BYTECODE
├── PUSH_PROC P             // Push value to match
├── PATH_EVAL current_path  // Evaluate P in current path
├── PATTERN Q               // Load pattern Q
├── PATH_MATCH_TEST current_path // Test if P matches Q in path context
├── PUSH_BOOL               // Push boolean result
└── PATH_UPDATE_RESULT current_path // Update path with match result
```

**Logical NOT**
```
not P  -> BYTECODE
├── PUSH_PROC P             // Push operand
├── PATH_EVAL_BOOL current_path // Evaluate to boolean in current path
├── NOT                     // Logical negation
└── PATH_UPDATE_RESULT current_path // Update path with negated result
```

**Logical OR**
```
P or Q  -> BYTECODE
├── PUSH_PROC P             // Push left operand
├── PATH_EVAL_BOOL current_path // Evaluate to boolean in current path
├── DUP                     // Duplicate result
├── BRANCH_TRUE L1          // Short-circuit if true
├── POP                     // Remove duplicate
├── PUSH_PROC Q             // Push right operand
├── PATH_EVAL_BOOL current_path // Evaluate to boolean in current path
├── L1: PATH_UPDATE_RESULT current_path // Update path with result
└── NOP                     // Result is on stack
```

### Process Logic Constructs
**Conjunction (Process AND)**
```
P /\ Q  -> BYTECODE
├── PATH_ALLOC              // Allocate conjunction path
├── PATH_FORK 2             // Fork into 2 paths for conjunction
├── PUSH_PROC P             // Push left process
├── PATH_SPAWN conj_path_1  // Spawn P in path 1
├── PUSH_PROC Q             // Push right process
├── PATH_SPAWN conj_path_2  // Spawn Q in path 2
├── PATH_CONJ_BARRIER [conj_path_1, conj_path_2] // Both must succeed
├── CONJ                    // Process conjunction
├── PATH_JOIN [conj_path_1, conj_path_2] // Join conjunction paths
└── PATH_CLEANUP conj_path  // Clean up conjunction path
```

**Disjunction (Process OR)**
```
P \/ Q  -> BYTECODE
├── PATH_ALLOC              // Allocate disjunction path
├── PATH_FORK 2             // Fork into 2 paths for disjunction
├── PUSH_PROC P             // Push left process
├── PATH_SPAWN disj_path_1  // Spawn P in path 1
├── PUSH_PROC Q             // Push right process
├── PATH_SPAWN disj_path_2  // Spawn Q in path 2
├── PATH_DISJ_RACE [disj_path_1, disj_path_2] // Either can succeed
├── DISJ                    // Process disjunction
├── PATH_SELECT_WINNER      // Select winning path
└── PATH_CLEANUP_LOSERS     // Clean up non-winning paths
```

**Process Negation**
```
~P  -> BYTECODE
├── PATH_ALLOC              // Allocate negation path
├── PUSH_PROC P             // Push process
├── PATH_EXEC neg_path      // Execute P in negation path
├── PATH_NEGATE neg_path    // Negate the path result
├── PROC_NEG                // Process negation
├── PATH_UPDATE_RESULT current_path // Update current path
└── PATH_CLEANUP neg_path   // Clean up negation path
```
