# Rholang Finite State Machine Design

## Introduction

This document proposes a Finite State Machine (FSM) design for Rholang that complements the existing bytecode and pathmap designs. While the bytecode designs focus on instruction-based execution and path-based concurrency, this FSM approach provides a state-based model that can represent the execution of Rholang programs as transitions between well-defined states.

## Design Philosophy

The FSM design follows these core principles:

1. **State-Based Representation**: Each Rholang construct is represented as a set of states and transitions.
2. **Compositional**: Complex processes are composed of simpler state machines.
3. **Concurrent Execution**: Multiple state machines can execute in parallel.
4. **Deterministic Behavior**: Given the same inputs, the FSM will always produce the same outputs.
5. **Compatibility**: The FSM design works with the same grammar as the bytecode designs.

## Core FSM Components

### 1. States

Each state in the FSM represents a specific point in the execution of a Rholang program:

```
STATE TYPES:
├── INITIAL              // Starting state for any process
├── EVALUATING           // Evaluating an expression
├── SENDING              // Sending a message
├── RECEIVING            // Receiving a message
├── WAITING              // Waiting for a condition
├── BRANCHING            // Making a decision
├── FORKING              // Creating parallel processes
├── JOINING              // Synchronizing parallel processes
├── BINDING              // Binding variables
├── MATCHING             // Pattern matching
├── CONSTRUCTING         // Constructing data structures
├── OPERATING            // Performing operations
├── BUNDLING             // Creating a bundle
├── REFERENCING          // Creating a variable reference
├── INTERPOLATING        // Performing string interpolation
├── CONJOINING           // Performing process conjunction
├── DISJOINING           // Performing process disjunction
├── NEGATING             // Performing process negation
├── COLLECTING           // Constructing collections (sets, maps)
├── TERMINATING          // Process is terminating
└── TERMINATED           // Process has terminated
```

### 2. Transitions

Transitions define how the FSM moves from one state to another:

```
TRANSITION TYPES:
├── EVALUATE             // Evaluate an expression
├── SEND                 // Send a message
├── RECEIVE              // Receive a message
├── FORK                 // Create parallel processes
├── JOIN                 // Synchronize parallel processes
├── BIND                 // Bind a variable
├── MATCH                // Perform pattern matching
├── BRANCH               // Make a conditional decision
├── CONSTRUCT            // Construct a data structure
├── OPERATE              // Perform an operation
├── BUNDLE               // Create a bundle
├── REFERENCE            // Create a variable reference
├── INTERPOLATE          // Perform string interpolation
├── CONJOIN              // Perform process conjunction
├── DISJOIN              // Perform process disjunction
├── NEGATE               // Perform process negation
├── COLLECT              // Construct a collection
└── TERMINATE            // Terminate a process
```

### 3. Events

Events trigger transitions between states:

```
EVENT TYPES:
├── MESSAGE_AVAILABLE    // A message is available on a channel
├── CONDITION_MET        // A condition has been satisfied
├── EXPRESSION_EVALUATED // An expression has been evaluated
├── PATTERN_MATCHED      // A pattern has been matched
├── TIMEOUT              // A timeout has occurred
├── ERROR                // An error has occurred
└── SIGNAL               // A signal has been received
```

## FSM Representation of Rholang Constructs

### Core Process Constructs

**Parallel Composition (par)**
```
P | Q -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ FORK    │ FORKING │ FORK    │ JOINING │
│         │────────>│         │────────>│         │
└─────────┘         └─────────┘         └─────────┘
                        │                    │
                        │                    │
                        ▼                    ▼
                    ┌─────────┐         ┌─────────┐
                    │ P:FSM   │         │ Q:FSM   │
                    │         │         │         │
                    └─────────┘         └─────────┘
                        │                    │
                        │                    │
                        ▼                    ▼
                    ┌─────────┐         ┌─────────┐
                    │ P:TERM  │         │ Q:TERM  │
                    │         │         │         │
                    └─────────┘         └─────────┘
                        │                    │
                        │                    │
                        └──────┐     ┌───────┘
                               ▼     ▼
                           ┌─────────┐
                           │TERMINATED│
                           │         │
                           └─────────┘
```

**Name Creation (new)**
```
new x, y in P -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ BIND    │ BINDING │ BIND    │ P:FSM   │
│         │────────>│ x       │────────>│         │
└─────────┘         └─────────┘         └─────────┘
                        │                    │
                        │                    │
                        ▼                    ▼
                    ┌─────────┐         ┌─────────┐
                    │ BINDING │         │ P:TERM  │
                    │ y       │         │         │
                    └─────────┘         └─────────┘
                        │                    │
                        │                    │
                        └──────┐     ┌───────┘
                               ▼     ▼
                           ┌─────────┐
                           │TERMINATED│
                           │         │
                           └─────────┘
```

**Asynchronous Send (send)**
```
chan!(data) -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│ SENDING │TERMINATE│TERMINATED│
│         │────────>│ chan    │────────>│         │────────>│         │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                        │
                        │
                        ▼
                    ┌─────────┐
                    │EVALUATING│
                    │ data    │
                    └─────────┘
```

**Synchronous Send (send_sync)**
```
chan!?(data); P -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│ SENDING │ WAIT    │ WAITING │
│         │────────>│ chan    │────────>│         │────────>│         │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                        │                                        │
                        │                                        │
                        ▼                                        ▼
                    ┌─────────┐                             ┌─────────┐
                    │EVALUATING│                            │ P:FSM   │
                    │ data    │                            │         │
                    └─────────┘                            └─────────┘
                                                               │
                                                               │
                                                               ▼
                                                           ┌─────────┐
                                                           │ P:TERM  │
                                                           │         │
                                                           └─────────┘
                                                               │
                                                               │
                                                               ▼
                                                           ┌─────────┐
                                                           │TERMINATED│
                                                           │         │
                                                           └─────────┘
```

**Input/Receive (for)**
```
for(x <- chan) P -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ RECEIVE │RECEIVING │ BIND    │ BINDING │
│         │────────>│ chan    │────────>│         │────────>│ x       │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │ P:FSM   │
                                                            │         │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │ P:TERM  │
                                                            │         │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```

**Replicated Receive (contract)**
```
contract Name(x) = P -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ BIND    │ BINDING │ RECEIVE │RECEIVING │
│         │────────>│ Name    │────────>│ PERSIST │
└─────────┘         └─────────┘         └─────────┘
                                            │
                                            │ MESSAGE_AVAILABLE
                                            ▼
                                        ┌─────────┐         ┌─────────┐
                                        │ BINDING │ FORK    │ P:FSM   │
                                        │ x       │────────>│         │
                                        └─────────┘         └─────────┘
                                            │                    │
                                            │                    │
                                            │                    ▼
                                            │                ┌─────────┐
                                            │                │ P:TERM  │
                                            │                │         │
                                            │                └─────────┘
                                            │                    │
                                            │                    │
                                            └────────────────────┘
                                                     │
                                                     ▼
                                                 ┌─────────┐
                                                 │RECEIVING │
                                                 │ PERSIST │
                                                 └─────────┘
```

### Control Flow Constructs

**Conditional (if-else)**
```
if (cond) P else Q -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ BRANCH  │BRANCHING │
│         │────────>│ cond    │────────>│         │
└─────────┘         └─────────┘         └─────────┘
                                            │
                                            │
                        ┌──────────────────┴──────────────────┐
                        │                                     │
                        ▼                                     ▼
                    ┌─────────┐                          ┌─────────┐
                    │ P:FSM   │                          │ Q:FSM   │
                    │         │                          │         │
                    └─────────┘                          └─────────┘
                        │                                     │
                        │                                     │
                        ▼                                     ▼
                    ┌─────────┐                          ┌─────────┐
                    │ P:TERM  │                          │ Q:TERM  │
                    │         │                          │         │
                    └─────────┘                          └─────────┘
                        │                                     │
                        │                                     │
                        └──────────────┐       ┌─────────────┘
                                       ▼       ▼
                                   ┌─────────┐
                                   │TERMINATED│
                                   │         │
                                   └─────────┘
```

**Pattern Matching (match)**
```
match expr { pat1 => P1; pat2 => P2 } -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ MATCH   │MATCHING │
│         │────────>│ expr    │────────>│ pat1    │
└─────────┘         └─────────┘         └─────────┘
                                            │
                                            │
                        ┌──────────────────┴──────────────────┐
                        │ PATTERN_MATCHED                     │ !PATTERN_MATCHED
                        ▼                                     ▼
                    ┌─────────┐                          ┌─────────┐
                    │ P1:FSM  │                          │MATCHING │
                    │         │                          │ pat2    │
                    └─────────┘                          └─────────┘
                        │                                     │
                        │                                     │
                        ▼                                     ▼
                    ┌─────────┐                          ┌─────────┐
                    │ P1:TERM │                          │ P2:FSM  │
                    │         │                          │         │
                    └─────────┘                          └─────────┘
                        │                                     │
                        │                                     │
                        │                                     ▼
                        │                                 ┌─────────┐
                        │                                 │ P2:TERM │
                        │                                 │         │
                        │                                 └─────────┘
                        │                                     │
                        │                                     │
                        └──────────────┐       ┌─────────────┘
                                       ▼       ▼
                                   ┌─────────┐
                                   │TERMINATED│
                                   │         │
                                   └─────────┘
```

**Select/Choice (select)**
```
select { x <- chan1 => P1; y <- chan2 => P2 } -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ RECEIVE │RECEIVING │
│         │────────>│ chan1,  │────────>│ RACE    │
│         │         │ chan2   │         │         │
└─────────┘         └─────────┘         └─────────┘
                                            │
                                            │
                        ┌──────────────────┴──────────────────┐
                        │ chan1 MESSAGE_AVAILABLE             │ chan2 MESSAGE_AVAILABLE
                        ▼                                     ▼
                    ┌─────────┐                          ┌─────────┐
                    │ BINDING │                          │ BINDING │
                    │ x       │                          │ y       │
                    └─────────┘                          └─────────┘
                        │                                     │
                        │                                     │
                        ▼                                     ▼
                    ┌─────────┐                          ┌─────────┐
                    │ P1:FSM  │                          │ P2:FSM  │
                    │         │                          │         │
                    └─────────┘                          └─────────┘
                        │                                     │
                        │                                     │
                        ▼                                     ▼
                    ┌─────────┐                          ┌─────────┐
                    │ P1:TERM │                          │ P2:TERM │
                    │         │                          │         │
                    └─────────┘                          └─────────┘
                        │                                     │
                        │                                     │
                        └──────────────┐       ┌─────────────┘
                                       ▼       ▼
                                   ┌─────────┐
                                   │TERMINATED│
                                   │         │
                                   └─────────┘
```

### Expression Constructs

**Arithmetic Operations**
```
P + Q -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│ OPERATE │OPERATING │
│         │────────>│ P       │────────>│ Q       │────────>│ ADD     │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```

**Logical Operations**
```
P and Q -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ BRANCH  │BRANCHING │ EVALUATE│EVALUATING│
│         │────────>│ P       │────────>│         │────────>│ Q       │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                            │                    │
                                            │ P=false            │
                                            ▼                    ▼
                                        ┌─────────┐         ┌─────────┐
                                        │TERMINATED│         │TERMINATED│
                                        │ false   │         │ Q result │
                                        └─────────┘         └─────────┘
```

**Method Call**
```
obj.method(args) -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│ OPERATE │OPERATING │
│         │────────>│ obj     │────────>│ args    │────────>│ INVOKE  │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```

### Data Constructs

**List Construction**
```
[P, Q, ...R] -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│CONSTRUCT│CONSTRUCTING
│         │────────>│ P       │────────>│ Q       │────────>│ LIST    │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                            │                    │
                                            │                    │
                                            ▼                    ▼
                                        ┌─────────┐         ┌─────────┐
                                        │EVALUATING│         │TERMINATED│
                                        │ R       │         │         │
                                        └─────────┘         └─────────┘
```
### Additional Rholang Constructs

This section covers additional Rholang constructs that are part of the language grammar but were not explicitly represented in the original FSM design.

#### Bundle Operations

**Bundle Creation (bundle+, bundle-, bundle0, bundle)**
```
bundle+ { P } -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ BUNDLE  │ BUNDLING │ EVALUATE│ P:FSM   │TERMINATE│TERMINATED│
│         │────────>│ WRITE   │────────>│         │────────>│         │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                            │
                                            │
                                            ▼
                                        ┌─────────┐
                                        │ P:TERM  │
                                        │         │
                                        └─────────┘
```

```
bundle- { P } -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ BUNDLE  │ BUNDLING │ EVALUATE│ P:FSM   │TERMINATE│TERMINATED│
│         │────────>│ READ    │────────>│         │────────>│         │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                            │
                                            │
                                            ▼
                                        ┌─────────┐
                                        │ P:TERM  │
                                        │         │
                                        └─────────┘
```

```
bundle0 { P } -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ BUNDLE  │ BUNDLING │ EVALUATE│ P:FSM   │TERMINATE│TERMINATED│
│         │────────>│ EQUIV   │────────>│         │────────>│         │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                            │
                                            │
                                            ▼
                                        ┌─────────┐
                                        │ P:TERM  │
                                        │         │
                                        └─────────┘
```

```
bundle { P } -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ BUNDLE  │ BUNDLING │ EVALUATE│ P:FSM   │TERMINATE│TERMINATED│
│         │────────>│ RW      │────────>│         │────────>│         │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                            │
                                            │
                                            ▼
                                        ┌─────────┐
                                        │ P:TERM  │
                                        │         │
                                        └─────────┘
```

#### String Operations

**String Interpolation**
```
P %% Q -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│INTERPOLATE│INTERPOLATING│
│         │────────>│ P       │────────>│ Q       │────────>│           │
└─────────┘         └─────────┘         └─────────┘         └───────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```

#### Variable Reference Operations

**Variable Reference (=var)**
```
=var -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│REFERENCE│REFERENCING│
│         │────────>│ var     │────────>│ COPY     │
└─────────┘         └─────────┘         └──────────┘
                                            │
                                            │
                                            ▼
                                        ┌─────────┐
                                        │TERMINATED│
                                        │         │
                                        └─────────┘
```

**Variable Reference with Move (=*var)**
```
=*var -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│REFERENCE│REFERENCING│
│         │────────>│ var     │────────>│ MOVE     │
└─────────┘         └─────────┘         └──────────┘
                                            │
                                            │
                                            ▼
                                        ┌─────────┐
                                        │TERMINATED│
                                        │         │
                                        └─────────┘
```

#### Process Logic Operations

**Process Conjunction (P /\ Q)**
```
P /\ Q -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│ CONJOIN │CONJOINING│
│         │────────>│ P       │────────>│ Q       │────────>│         │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```

**Process Disjunction (P \/ Q)**
```
P \/ Q -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│ DISJOIN │DISJOINING│
│         │────────>│ P       │────────>│ Q       │────────>│         │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```

**Process Negation (~P)**
```
~P -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ NEGATE  │NEGATING │
│         │────────>│ P       │────────>│         │
└─────────┘         └─────────┘         └─────────┘
                                            │
                                            │
                                            ▼
                                        ┌─────────┐
                                        │TERMINATED│
                                        │         │
                                        └─────────┘
```

#### Collection Operations

**Set Construction**
```
Set(P, Q, R) -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│ COLLECT │COLLECTING│
│         │────────>│ P       │────────>│ Q       │────────>│ SET     │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                            │                    │
                                            │                    │
                                            ▼                    ▼
                                        ┌─────────┐         ┌─────────┐
                                        │EVALUATING│         │TERMINATED│
                                        │ R       │         │         │
                                        └─────────┘         └─────────┘
```

**Map Construction**
```
{key1: val1, key2: val2} -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│
│         │────────>│ key1    │────────>│ val1    │────────>│ key2    │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │EVALUATING│
                                                            │ val2    │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │COLLECTING│
                                                            │ MAP     │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```

#### Enhanced Receipt Types

**Repeated Bind (<=)**
```
for(x <= chan) P -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ RECEIVE │RECEIVING │ BIND    │ BINDING │
│         │────────>│ chan    │────────>│ PERSIST │────────>│ x       │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                            │                    │
                                            │                    │
                                            │                    ▼
                                            │                ┌─────────┐
                                            │                │ P:FSM   │
                                            │                │         │
                                            │                └─────────┘
                                            │                    │
                                            │                    │
                                            │                    ▼
                                            │                ┌─────────┐
                                            │                │ P:TERM  │
                                            │                │         │
                                            │                └─────────┘
                                            │                    │
                                            │                    │
                                            └────────────────────┘
                                                     │
                                                     ▼
                                                 ┌─────────┐
                                                 │RECEIVING │
                                                 │ PERSIST │
                                                 └─────────┘
```

**Peek Bind (<<-)**
```
for(x <<- chan) P -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ RECEIVE │RECEIVING │ BIND    │ BINDING │
│         │────────>│ chan    │────────>│ PEEK    │────────>│ x       │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │ P:FSM   │
                                                            │         │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │ P:TERM  │
                                                            │         │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```

#### Enhanced Source Types

**Receive-Send Source (?!)**
```
for(x <- chan?!) P -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ RECEIVE │RECEIVING │ BIND    │ BINDING │
│         │────────>│ chan?!  │────────>│ RECV_SEND│────────>│ x       │
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │ P:FSM   │
                                                            │         │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │ P:TERM  │
                                                            │         │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```

**Send-Receive Source (!?)**
```
for(x <- chan!?(data)) P -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│ RECEIVE │RECEIVING │
│         │────────>│ chan    │────────>│ data    │────────>│ SEND_RECV│
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │ BINDING │
                                                            │ x       │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │ P:FSM   │
                                                            │         │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │ P:TERM  │
                                                            │         │
                                                            └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```

#### Enhanced Send Types

**Multiple Send (!!)**
```
chan!!(data) -> FSM
┌─────────┐         ┌─────────┐         ┌─────────┐         ┌─────────┐
│ INITIAL │ EVALUATE│EVALUATING│ EVALUATE│EVALUATING│ SEND    │ SENDING │
│         │────────>│ chan    │────────>│ data    │────────>│ MULTIPLE│
└─────────┘         └─────────┘         └─────────┘         └─────────┘
                                                                │
                                                                │
                                                                ▼
                                                            ┌─────────┐
                                                            │TERMINATED│
                                                            │         │
                                                            └─────────┘
```
## State Machine Execution Model

The FSM execution model follows these principles:

1. **State Transitions**: Execution progresses through state transitions triggered by events.
2. **Concurrent Execution**: Multiple FSMs can execute concurrently, each representing a separate process.
3. **Communication**: FSMs communicate through events, particularly MESSAGE_AVAILABLE events.
4. **Composition**: Complex FSMs are composed of simpler FSMs.

### Execution Algorithm

1. Initialize the FSM for the main process in the INITIAL state.
2. Process events and perform transitions until all FSMs reach TERMINATED state:
   a. For each active FSM, check if any transitions are enabled.
   b. Execute enabled transitions, potentially creating new FSMs.
   c. Process any events generated by transitions.
3. When all FSMs reach TERMINATED state, execution is complete.

## Advantages of the FSM Design

1. **Explicit State Representation**: The FSM design makes the execution state explicit, which can simplify reasoning about program behavior.
2. **Formal Verification**: FSMs are amenable to formal verification techniques.
3. **Visual Representation**: FSMs can be visualized as state diagrams, aiding in understanding and debugging.
4. **Event-Driven Model**: The event-driven nature of FSMs aligns well with Rholang's concurrent and reactive programming model.
5. **Compositional Reasoning**: FSMs support compositional reasoning about program behavior.

## Relationship to Bytecode Designs

The FSM design complements the existing bytecode designs:

1. **Bytecode Design**: Focuses on instruction-based execution with a stack-based VM.
2. **PathMap Design**: Emphasizes path-based concurrency and execution contexts.
3. **FSM Design**: Provides a state-based model with explicit transitions and events.

These designs can be integrated:
- Bytecode instructions can implement state transitions in the FSM.
- PathMap paths can correspond to concurrent FSM instances.
- FSM states can guide optimization of bytecode generation.

## Implementation Considerations

1. **State Representation**: States should be represented efficiently, possibly as enums or integers.
2. **Transition Functions**: Transitions should be implemented as pure functions that take a state and an event and return a new state.
3. **Event Queue**: An event queue is needed to manage events between FSMs.
4. **Concurrency Control**: Mechanisms for managing concurrent FSM execution are required.
5. **Memory Management**: Efficient memory management for FSM instances is essential.

## Conclusion

The proposed Finite State Machine design provides a complementary approach to the existing bytecode designs for Rholang. By representing execution as states and transitions, the FSM design offers advantages in terms of reasoning about program behavior, formal verification, and visualization. The design is compatible with the existing Rholang grammar and can be integrated with the bytecode and pathmap approaches to provide a comprehensive execution model for Rholang programs.

## Bibliography

This bibliography provides resources for further exploration of finite state machines, particularly in the context of concurrency, lambda calculus, and functional programming.

### Books and Textbooks

#### Finite State Machines and Automata Theory
- Hopcroft, J. E., Motwani, R., & Ullman, J. D. (2006). *Introduction to Automata Theory, Languages, and Computation* (3rd ed.). Pearson.
- Sipser, M. (2012). *Introduction to the Theory of Computation* (3rd ed.). Cengage Learning.
- Cassandras, C. G., & Lafortune, S. (2008). *Introduction to Discrete Event Systems* (2nd ed.). Springer.
- Alur, R., & Dill, D. L. (1994). *A Theory of Timed Automata*. Theoretical Computer Science, 126(2), 183-235.

#### Concurrency and Finite State Machines
- Baier, C., & Katoen, J. P. (2008). *Principles of Model Checking*. MIT Press.
- Milner, R. (1999). *Communicating and Mobile Systems: The π-Calculus*. Cambridge University Press.

### Research Papers and Articles

#### Finite State Machines for Concurrency
- Harel, D. (1987). "Statecharts: A Visual Formalism for Complex Systems." *Science of Computer Programming*, 8(3), 231-274.
- Vardi, M. Y., & Wolper, P. (1986). "An Automata-Theoretic Approach to Automatic Program Verification." *Proceedings of the First Annual IEEE Symposium on Logic in Computer Science*.
- Pnueli, A. (1977). "The Temporal Logic of Programs." *18th Annual Symposium on Foundations of Computer Science*, 46-57.
- Clarke, E. M., Emerson, E. A., & Sistla, A. P. (1986). "Automatic Verification of Finite-State Concurrent Systems Using Temporal Logic Specifications." *ACM Transactions on Programming Languages and Systems*, 8(2), 244-263.

#### Functional Programming and State Machines
- Wadler, P. (1997). "How to Declare an Imperative." *ACM Computing Surveys*, 29(3), 240-263.
- Peyton Jones, S. L. (2001). "Tackling the Awkward Squad: Monadic Input/Output, Concurrency, Exceptions, and Foreign-Language Calls in Haskell." *Engineering Theories of Software Construction*, 47-96.

### Online Resources

#### Tutorials and Guides
- [Stanford's Automata Theory Course](https://lagunita.stanford.edu/courses/course-v1:ComputerScience+Automata+Fall2016/about)
- [Finite State Machines in Functional Programming](https://medium.com/@DzoQiEuoi/finite-state-machines-in-functional-programming-5f8c5d30442)
- [The Haskell Wiki: State Machine](https://wiki.haskell.org/State_machine)
- [Implementing State Machines in Scala](https://www.baeldung.com/scala/state-machine)

#### Blogs and Articles
- [Finite State Machines for Concurrent Programming](https://blog.nelhage.com/2010/05/using-finite-state-machines-in-concurrent-code/)
- [Functional Programming and State Machines](https://www.schoolofhaskell.com/user/pbv/an-introduction-to-state-machines)

#### Tools and Libraries
- [XState: State Machines and Statecharts for the Modern Web](https://xstate.js.org/)
- [SPIN Model Checker](http://spinroot.com/spin/whatispin.html)

### Video Resources

#### Lectures and Courses
- [Automata Theory - Stanford](https://www.youtube.com/playlist?list=PL6EF0274BD849A7D5) - Professor Jeffrey Ullman's course on automata theory.
- [Introduction to Finite State Machines](https://www.youtube.com/watch?v=Qa6csfkK7_I) - MIT OpenCourseWare.

#### Conference Talks
- [Finite State Machines in Functional Programming](https://www.youtube.com/watch?v=UVQ7N1o6Mhk) - Lambda Days 2019.
- [Communicating Sequential Processes](https://www.youtube.com/watch?v=3gXWA6WEvOM) - Rob Pike at Gopherfest 2015.

#### Tutorials and Demonstrations
- [Finite State Machines Explained](https://www.youtube.com/watch?v=Qa6csfkK7_I) - Computerphile.
- [Functional State Machines in Haskell](https://www.youtube.com/watch?v=l3JIxYKV0Ys) - Haskell eXchange 2018.
