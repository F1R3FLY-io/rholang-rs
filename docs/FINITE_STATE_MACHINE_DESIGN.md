# Rholang Finite State Machine Design

## Introduction

This document proposes a Finite State Machine (FSM) design for Rholang that complements the existing bytecode and pathmap designs. This approach bridges the gap between theoretical models of computation and practical implementation concerns, with a particular focus on three key areas:

1. **Concurrency**: Rholang's core strength is its native support for concurrent computation, inspired by the π-calculus. The FSM design provides a formal model for representing and reasoning about concurrent processes.

2. **Functional Programming**: While Rholang incorporates functional programming concepts, the execution of these concepts requires a clear operational model. The FSM design offers a state-based representation that aligns with functional transformations.

3. **Lambda Calculus**: As a foundation of both functional programming and process calculi, lambda calculus principles inform the FSM design, particularly in how it handles variable binding, substitution, and evaluation.

While the bytecode designs focus on instruction-based execution and path-based concurrency, this FSM approach provides a state-based model that can represent the execution of Rholang programs as transitions between well-defined states, making it particularly suitable for formal reasoning about program behavior.

## Design Philosophy

The FSM design follows these core principles, with special attention to concurrency, functional programming, and lambda calculus:

1. **State-Based Representation**: Each Rholang construct is represented as a set of states and transitions, providing a formal model that can be analyzed using techniques from automata theory.

2. **Compositional**: Complex processes are composed of simpler state machines, reflecting the compositional nature of both functional programming and process calculi.

3. **Concurrent Execution**: Multiple state machines can execute in parallel, directly modeling Rholang's concurrent semantics derived from the π-calculus.

4. **Functional Purity**: State transitions are designed as pure functions that take a state and an event and return a new state, aligning with functional programming principles.

5. **Lambda-Inspired Binding**: The handling of name creation and variable binding in the FSM reflects lambda calculus principles, particularly in how it manages scope and substitution.

6. **Deterministic Behavior**: Given the same inputs, the FSM will always produce the same outputs, supporting equational reasoning common in functional programming.

7. **Compatibility**: The FSM design works with the same grammar as the bytecode designs, ensuring a consistent semantic model across different implementation approaches.

## Core FSM Components

The core components of the FSM design are deeply rooted in concurrency theory, functional programming principles, and lambda calculus concepts. These components provide a formal framework for representing the execution of Rholang programs.

### 1. States: Representing Computational Context

Each state in the FSM represents a specific point in the execution of a Rholang program. The state types reflect both the concurrent nature of Rholang and its functional aspects:

```
STATE TYPES:
├── INITIAL              // Starting state for any process
├── EVALUATING           // Evaluating an expression (functional evaluation)
├── SENDING              // Sending a message (concurrent communication)
├── RECEIVING            // Receiving a message (concurrent communication)
├── WAITING              // Waiting for a condition (synchronization primitive)
├── BRANCHING            // Making a decision (functional control flow)
├── FORKING              // Creating parallel processes (concurrent execution)
├── JOINING              // Synchronizing parallel processes (concurrent coordination)
├── BINDING              // Binding variables (lambda calculus substitution)
├── MATCHING             // Pattern matching (functional decomposition)
├── CONSTRUCTING         // Constructing data structures (functional composition)
├── OPERATING            // Performing operations (functional transformation)
├── BUNDLING             // Creating a bundle (capability-based security)
├── REFERENCING          // Creating a variable reference (functional reference)
├── INTERPOLATING        // Performing string interpolation (functional transformation)
├── CONJOINING           // Performing process conjunction (logical composition)
├── DISJOINING           // Performing process disjunction (logical composition)
├── NEGATING             // Performing process negation (logical transformation)
├── COLLECTING           // Constructing collections (functional data structures)
├── TERMINATING          // Process is terminating (lifecycle management)
└── TERMINATED           // Process has terminated (final state)
```

These states capture the essence of both concurrent computation (SENDING, RECEIVING, FORKING, JOINING) and functional evaluation (EVALUATING, BINDING, MATCHING). The BINDING state, in particular, reflects lambda calculus principles of variable binding and substitution.

### 2. Transitions: Pure Functional Transformations

Transitions define how the FSM moves from one state to another. In line with functional programming principles, these transitions are designed as pure functions that take a current state and an event and produce a new state without side effects:

```
TRANSITION TYPES:
├── EVALUATE             // Evaluate an expression (functional evaluation)
├── SEND                 // Send a message (π-calculus output)
├── RECEIVE              // Receive a message (π-calculus input)
├── FORK                 // Create parallel processes (concurrent composition)
├── JOIN                 // Synchronize parallel processes (concurrent coordination)
├── BIND                 // Bind a variable (lambda calculus binding)
├── MATCH                // Perform pattern matching (functional pattern matching)
├── BRANCH               // Make a conditional decision (functional branching)
├── CONSTRUCT            // Construct a data structure (functional construction)
├── OPERATE              // Perform an operation (functional operation)
├── BUNDLE               // Create a bundle (capability restriction)
├── REFERENCE            // Create a variable reference (functional reference)
├── INTERPOLATE          // Perform string interpolation (functional transformation)
├── CONJOIN              // Perform process conjunction (logical AND)
├── DISJOIN              // Perform process disjunction (logical OR)
├── NEGATE               // Perform process negation (logical NOT)
├── COLLECT              // Construct a collection (functional collection)
└── TERMINATE            // Terminate a process (lifecycle completion)
```

These transitions embody both the concurrent communication model of the π-calculus (SEND, RECEIVE, FORK, JOIN) and functional transformations (EVALUATE, BIND, MATCH, CONSTRUCT). The BIND transition, in particular, implements lambda calculus binding semantics.

### 3. Events: Communication and Coordination

Events trigger transitions between states, representing the reactive and message-passing nature of Rholang. These events are central to modeling concurrent computation:

```
EVENT TYPES:
├── MESSAGE_AVAILABLE    // A message is available on a channel (concurrent communication)
├── CONDITION_MET        // A condition has been satisfied (synchronization)
├── EXPRESSION_EVALUATED // An expression has been evaluated (functional completion)
├── PATTERN_MATCHED      // A pattern has been matched (functional pattern matching)
├── TIMEOUT              // A timeout has occurred (temporal behavior)
├── ERROR                // An error has occurred (exception handling)
└── SIGNAL               // A signal has been received (inter-process communication)
```

The event system reflects the asynchronous, message-passing nature of concurrent systems while also accommodating functional programming concepts like expression evaluation and pattern matching. The MESSAGE_AVAILABLE event, in particular, directly models the π-calculus communication primitive.

### 4. Functional Evaluation Model

The FSM design incorporates a functional evaluation model that aligns with lambda calculus principles:

1. **Substitution-based Semantics**: Variable binding in the FSM follows substitution-based semantics from lambda calculus.
2. **Lexical Scoping**: Name creation and variable binding respect lexical scoping rules.
3. **Immutable State Transitions**: Each state transition produces a new state rather than modifying the existing one.
4. **Referential Transparency**: Given the same inputs, state transitions always produce the same outputs.

This functional evaluation model ensures that the FSM design can accurately represent the execution of functional aspects of Rholang while maintaining the concurrent semantics derived from the π-calculus.

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

## Theoretical Foundations: Concurrency, Functional Programming, and Lambda Calculus

The FSM design for Rholang is grounded in three fundamental theoretical areas that together provide a comprehensive framework for understanding and implementing the language:

### Concurrency Theory and the π-Calculus

Rholang's concurrency model is derived from the π-calculus, a process calculus developed by Robin Milner that extends the calculus of communicating systems (CCS) with the ability to communicate channel names. The FSM design captures this concurrency model through:

1. **Message-Passing Semantics**: The SEND and RECEIVE transitions directly model the output and input primitives of the π-calculus.
2. **Channel-Based Communication**: Channels are first-class entities in both the π-calculus and the FSM design.
3. **Parallel Composition**: The FORK and JOIN transitions implement the parallel composition operator of the π-calculus.
4. **Name Restriction**: The BIND transition for name creation corresponds to the name restriction operator (ν) in the π-calculus.
5. **Replication**: The persistent receive state (RECEIVING PERSIST) models the replication operator (!) in the π-calculus.

These elements enable the FSM design to accurately represent the concurrent behavior of Rholang programs, including dynamic creation of processes and channels, message passing, and parallel execution.

### Functional Programming Principles

While Rholang incorporates functional programming concepts, the FSM design provides a formal operational semantics for these concepts:

1. **Immutable State Transitions**: Each state transition produces a new state rather than modifying the existing one, reflecting the immutability principle of functional programming.
2. **Pure Functions**: Transitions are designed as pure functions without side effects.
3. **Pattern Matching**: The MATCHING state implements functional pattern matching for data decomposition.
4. **Higher-Order Functions**: The ability to send processes as messages enables higher-order programming patterns.
5. **Compositional Design**: The FSM design is compositional, allowing complex behaviors to be built from simpler ones.

These functional programming principles make the FSM design more amenable to formal reasoning and verification, while also aligning with Rholang's functional aspects.

### Lambda Calculus Foundations

The lambda calculus, developed by Alonzo Church, provides a formal system for expressing computation based on function abstraction and application. The FSM design incorporates lambda calculus principles in several ways:

1. **Variable Binding**: The BINDING state implements variable binding following lambda calculus substitution rules.
2. **Lexical Scoping**: Name creation and variable binding in the FSM respect lexical scoping rules from lambda calculus.
3. **Evaluation Strategy**: The FSM design implements a specific evaluation strategy (similar to call-by-value) for expressions.
4. **Alpha-Equivalence**: The FSM design respects alpha-equivalence by treating alpha-equivalent processes as semantically identical.
5. **Beta-Reduction**: The application of functions in Rholang corresponds to beta-reduction in lambda calculus.

These lambda calculus foundations provide a theoretical basis for understanding the execution of Rholang programs, particularly in how they handle variables, functions, and evaluation.

## Relationship to Bytecode Designs

The FSM design complements the existing bytecode designs, with each approach emphasizing different aspects of Rholang's execution model:

1. **Bytecode Design**: Focuses on instruction-based execution with a stack-based VM, providing an efficient implementation strategy.
2. **PathMap Design**: Emphasizes path-based concurrency and execution contexts, addressing the practical challenges of implementing concurrent processes.
3. **FSM Design**: Provides a state-based model with explicit transitions and events, offering a formal foundation for reasoning about program behavior.

These designs can be integrated to leverage their respective strengths:
- Bytecode instructions can implement state transitions in the FSM, connecting formal semantics to efficient execution.
- PathMap paths can correspond to concurrent FSM instances, providing a practical implementation of the concurrent semantics.
- FSM states can guide optimization of bytecode generation, using formal properties to improve performance.

## Implementation Considerations

Implementing the FSM design requires careful attention to several aspects:

1. **State Representation**: States should be represented efficiently, possibly as enums or integers, while preserving their semantic meaning.
2. **Transition Functions**: Transitions should be implemented as pure functions that take a state and an event and return a new state, following functional programming principles.
3. **Event Queue**: An event queue is needed to manage events between FSMs, implementing the asynchronous communication model of the π-calculus.
4. **Concurrency Control**: Mechanisms for managing concurrent FSM execution are required, addressing the challenges of implementing true concurrency.
5. **Memory Management**: Efficient memory management for FSM instances is essential, particularly for handling dynamic process creation and termination.
6. **Formal Verification**: The formal nature of the FSM design enables the use of model checking and other verification techniques to ensure correctness.

## Conclusion

The Finite State Machine design for Rholang provides a formal foundation that bridges theoretical models and practical implementation concerns. By integrating concepts from concurrency theory, functional programming, and lambda calculus, the FSM design offers a comprehensive framework for understanding and implementing Rholang's semantics.

The design's emphasis on concurrency reflects Rholang's roots in the π-calculus, enabling accurate modeling of parallel processes, message passing, and channel-based communication. Its incorporation of functional programming principles supports reasoning about program behavior through immutability, pure functions, and compositional design. The lambda calculus foundations provide a theoretical basis for understanding variable binding, scoping, and evaluation.

By representing execution as states and transitions, the FSM design offers significant advantages:
- **Formal Reasoning**: The state-based model facilitates formal verification and analysis.
- **Concurrency Modeling**: The design explicitly represents concurrent execution and communication.
- **Functional Semantics**: The pure functional approach to transitions aligns with functional programming principles.
- **Theoretical Grounding**: The design is firmly rooted in established theoretical frameworks.

The FSM design is compatible with the existing Rholang grammar and can be integrated with the bytecode and pathmap approaches to provide a comprehensive execution model that is both theoretically sound and practically implementable. This integration of theory and practice is essential for a language like Rholang that aims to bring formal concurrency models to mainstream programming.

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
