# Rholang Examples: Code, Disassembly, and Results

This document shows Rholang code examples with their compiled bytecode
(disassembly) and execution results.

---

### Nil Literal

**Code:**
```rholang
Nil
```

**Disassembly:**
```
Process: proc_0
Instructions: 2

Bytecode:
  0000: PUSH_NIL  ; Push nil
  0001: HALT  ; Halt execution
```

**Result:** `Nil`

---

### Integer Literal

**Code:**
```rholang
42
```

**Disassembly:**
```
Process: proc_0
Instructions: 2

Bytecode:
  0000: PUSH_INT 0x2a  ; Push integer 42
  0001: HALT  ; Halt execution
```

**Result:** `42`

---

### Negative Integer

**Code:**
```rholang
-123
```

**Disassembly:**
```
Process: proc_0
Instructions: 2

Bytecode:
  0000: PUSH_INT 0xff85  ; Push integer -123
  0001: HALT  ; Halt execution
```

**Result:** `-123`

---

### Boolean True

**Code:**
```rholang
true
```

**Disassembly:**
```
Process: proc_0
Instructions: 2

Bytecode:
  0000: PUSH_BOOL 0x01  ; Push boolean true
  0001: HALT  ; Halt execution
```

**Result:** `true`

---

### Boolean False

**Code:**
```rholang
false
```

**Disassembly:**
```
Process: proc_0
Instructions: 2

Bytecode:
  0000: PUSH_BOOL 0x00  ; Push boolean false
  0001: HALT  ; Halt execution
```

**Result:** `false`

---

### String Literal

**Code:**
```rholang
"hello world"
```

**Disassembly:**
```
Process: proc_0
Instructions: 2

String Pool:
  [0]: Str("hello world")

Bytecode:
  0000: PUSH_STR 0x00  ; Push string at index 0
  0001: HALT  ; Halt execution
```

**Result:** `"hello world"`

---

### Empty String

**Code:**
```rholang
""
```

**Disassembly:**
```
Process: proc_0
Instructions: 2

String Pool:
  [0]: Str("")

Bytecode:
  0000: PUSH_STR 0x00  ; Push string at index 0
  0001: HALT  ; Halt execution
```

**Result:** `""`

---

### Addition

**Code:**
```rholang
1 + 2
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x02  ; Push integer 2
  0002: ADD  ; Add top two stack values
  0003: HALT  ; Halt execution
```

**Result:** `3`

---

### Subtraction

**Code:**
```rholang
10 - 3
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x0a  ; Push integer 10
  0001: PUSH_INT 0x03  ; Push integer 3
  0002: SUB  ; Subtract top two stack values
  0003: HALT  ; Halt execution
```

**Result:** `7`

---

### Multiplication

**Code:**
```rholang
4 * 5
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x04  ; Push integer 4
  0001: PUSH_INT 0x05  ; Push integer 5
  0002: MUL  ; Multiply top two stack values
  0003: HALT  ; Halt execution
```

**Result:** `20`

---

### Division

**Code:**
```rholang
20 / 4
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x14  ; Push integer 20
  0001: PUSH_INT 0x04  ; Push integer 4
  0002: DIV  ; Divide top two stack values
  0003: HALT  ; Halt execution
```

**Result:** `5`

---

### Complex Arithmetic

**Code:**
```rholang
(1 + 2) * (3 + 4)
```

**Disassembly:**
```
Process: proc_0
Instructions: 8

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x02  ; Push integer 2
  0002: ADD  ; Add top two stack values
  0003: PUSH_INT 0x03  ; Push integer 3
  0004: PUSH_INT 0x04  ; Push integer 4
  0005: ADD  ; Add top two stack values
  0006: MUL  ; Multiply top two stack values
  0007: HALT  ; Halt execution
```

**Result:** `21`

---

### Equality

**Code:**
```rholang
1 == 1
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x01  ; Push integer 1
  0002: CMP_EQ  ; Compare equal
  0003: HALT  ; Halt execution
```

**Result:** `true`

---

### Inequality

**Code:**
```rholang
1 != 2
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x02  ; Push integer 2
  0002: CMP_NEQ  ; Compare not equal
  0003: HALT  ; Halt execution
```

**Result:** `true`

---

### Less Than

**Code:**
```rholang
1 < 2
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x02  ; Push integer 2
  0002: CMP_LT  ; Compare less than
  0003: HALT  ; Halt execution
```

**Result:** `true`

---

### Greater Than

**Code:**
```rholang
2 > 1
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x02  ; Push integer 2
  0001: PUSH_INT 0x01  ; Push integer 1
  0002: CMP_GT  ; Compare greater than
  0003: HALT  ; Halt execution
```

**Result:** `true`

---

### Less or Equal

**Code:**
```rholang
1 <= 1
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x01  ; Push integer 1
  0002: CMP_LTE  ; Compare less than or equal
  0003: HALT  ; Halt execution
```

**Result:** `true`

---

### Greater or Equal

**Code:**
```rholang
2 >= 2
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x02  ; Push integer 2
  0001: PUSH_INT 0x02  ; Push integer 2
  0002: CMP_GTE  ; Compare greater than or equal
  0003: HALT  ; Halt execution
```

**Result:** `true`

---

### Boolean And

**Code:**
```rholang
true and true
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_BOOL 0x01  ; Push boolean true
  0001: PUSH_BOOL 0x01  ; Push boolean true
  0002: AND  ; Logical AND
  0003: HALT  ; Halt execution
```

**Result:** `true`

---

### Boolean Or

**Code:**
```rholang
true or false
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_BOOL 0x01  ; Push boolean true
  0001: PUSH_BOOL 0x00  ; Push boolean false
  0002: OR  ; Logical OR
  0003: HALT  ; Halt execution
```

**Result:** `true`

---

### Complex Boolean

**Code:**
```rholang
(1 < 2) and (3 > 2)
```

**Disassembly:**
```
Process: proc_0
Instructions: 8

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x02  ; Push integer 2
  0002: CMP_LT  ; Compare less than
  0003: PUSH_INT 0x03  ; Push integer 3
  0004: PUSH_INT 0x02  ; Push integer 2
  0005: CMP_GT  ; Compare greater than
  0006: AND  ; Logical AND
  0007: HALT  ; Halt execution
```

**Result:** `true`

---

### List

**Code:**
```rholang
[1, 2, 3]
```

**Disassembly:**
```
Process: proc_0
Instructions: 5

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x02  ; Push integer 2
  0002: PUSH_INT 0x03  ; Push integer 3
  0003: CREATE_LIST 0x03  ; Create list with 3 elements
  0004: HALT  ; Halt execution
```

**Result:** `[1, 2, 3]`

---

### Empty List

**Code:**
```rholang
[]
```

**Disassembly:**
```
Process: proc_0
Instructions: 2

Bytecode:
  0000: CREATE_LIST 0x00  ; Create list with 0 elements
  0001: HALT  ; Halt execution
```

**Result:** `[]`

---

### Nested List

**Code:**
```rholang
[[1, 2], [3, 4]]
```

**Disassembly:**
```
Process: proc_0
Instructions: 8

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x02  ; Push integer 2
  0002: CREATE_LIST 0x02  ; Create list with 2 elements
  0003: PUSH_INT 0x03  ; Push integer 3
  0004: PUSH_INT 0x04  ; Push integer 4
  0005: CREATE_LIST 0x02  ; Create list with 2 elements
  0006: CREATE_LIST 0x02  ; Create list with 2 elements
  0007: HALT  ; Halt execution
```

**Result:** `[[1, 2], [3, 4]]`

---

### Tuple

**Code:**
```rholang
(1, 2, 3)
```

**Disassembly:**
```
Process: proc_0
Instructions: 5

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x02  ; Push integer 2
  0002: PUSH_INT 0x03  ; Push integer 3
  0003: CREATE_TUPLE 0x03  ; Create tuple with 3 elements
  0004: HALT  ; Halt execution
```

**Result:** `(1, 2, 3)`

---

### Empty Tuple

**Code:**
```rholang
()
```

**Disassembly:**
```
Process: proc_0
Instructions: 2

Bytecode:
  0000: CREATE_TUPLE 0x00  ; Create tuple with 0 elements
  0001: HALT  ; Halt execution
```

**Result:** `()`

---

### Mixed Tuple

**Code:**
```rholang
(true, 42, "hello")
```

**Disassembly:**
```
Process: proc_0
Instructions: 5

String Pool:
  [0]: Str("hello")

Bytecode:
  0000: PUSH_BOOL 0x01  ; Push boolean true
  0001: PUSH_INT 0x2a  ; Push integer 42
  0002: PUSH_STR 0x00  ; Push string at index 0
  0003: CREATE_TUPLE 0x03  ; Create tuple with 3 elements
  0004: HALT  ; Halt execution
```

**Result:** `(true, 42, "hello")`

---

### If True Branch

**Code:**
```rholang
if (true) { 1 } else { 2 }
```

**Disassembly:**
```
Process: proc_0
Instructions: 6

Bytecode:
  0000: PUSH_BOOL 0x01  ; Push boolean true
  0001: BRANCH_FALSE 0x04  ; Branch to 4 if false
  0002: PUSH_INT 0x01  ; Push integer 1
  0003: JUMP 0x05  ; Jump to instruction 5
  0004: PUSH_INT 0x02  ; Push integer 2
  0005: HALT  ; Halt execution
```

**Result:** `1`

---

### If False Branch

**Code:**
```rholang
if (false) { 1 } else { 2 }
```

**Disassembly:**
```
Process: proc_0
Instructions: 6

Bytecode:
  0000: PUSH_BOOL 0x00  ; Push boolean false
  0001: BRANCH_FALSE 0x04  ; Branch to 4 if false
  0002: PUSH_INT 0x01  ; Push integer 1
  0003: JUMP 0x05  ; Jump to instruction 5
  0004: PUSH_INT 0x02  ; Push integer 2
  0005: HALT  ; Halt execution
```

**Result:** `2`

---

### If With Comparison

**Code:**
```rholang
if (1 < 2) { "yes" } else { "no" }
```

**Disassembly:**
```
Process: proc_0
Instructions: 8

String Pool:
  [0]: Str("yes")
  [1]: Str("no")

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: PUSH_INT 0x02  ; Push integer 2
  0002: CMP_LT  ; Compare less than
  0003: BRANCH_FALSE 0x06  ; Branch to 6 if false
  0004: PUSH_STR 0x00  ; Push string at index 0
  0005: JUMP 0x07  ; Jump to instruction 7
  0006: PUSH_STR 0x01  ; Push string at index 1
  0007: HALT  ; Halt execution
```

**Result:** `"yes"`

---

### Parallel

**Code:**
```rholang
1 | 2
```

**Disassembly:**
```
Process: proc_0
Instructions: 4

Bytecode:
  0000: PUSH_INT 0x01  ; Push integer 1
  0001: POP  ; Pop top of stack
  0002: PUSH_INT 0x02  ; Push integer 2
  0003: HALT  ; Halt execution
```

**Result:** `2`

---

### Multiple Parallel

**Code:**
```rholang
Nil | Nil | 42
```

**Disassembly:**
```
Process: proc_0
Instructions: 6

Bytecode:
  0000: PUSH_NIL  ; Push nil
  0001: POP  ; Pop top of stack
  0002: PUSH_NIL  ; Push nil
  0003: POP  ; Pop top of stack
  0004: PUSH_INT 0x2a  ; Push integer 42
  0005: HALT  ; Halt execution
```

**Result:** `42`

---

### New Channel

**Code:**
```rholang
new x in { Nil }
```

**Disassembly:**
```
Process: proc_0
Instructions: 5

Bytecode:
  0000: NAME_CREATE 0x03  ; Create name with kind 3
  0001: ALLOC_LOCAL 0x00  ; Allocate local variable
  0002: STORE_LOCAL 0x00  ; Store to local variable #0
  0003: PUSH_NIL  ; Push nil
  0004: HALT  ; Halt execution
```

**Result:** `Nil`

---

### Send and Receive

**Code:**
```rholang
new x in { x!(42) | for (y <- x) { y } }
```

**Disassembly:**
```
Process: proc_0
Instructions: 14

Bytecode:
  0000: NAME_CREATE 0x03  ; Create name with kind 3
  0001: ALLOC_LOCAL 0x00  ; Allocate local variable
  0002: STORE_LOCAL 0x00  ; Store to local variable #0
  0003: LOAD_LOCAL 0x00  ; Load local variable #0
  0004: PUSH_INT 0x2a  ; Push integer 42
  0005: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0006: POP  ; Pop top of stack
  0007: LOAD_LOCAL 0x00  ; Load local variable #0
  0008: ASK 0x3, 0x0  ; Receive from channel (kind: 3)
  0009: ALLOC_LOCAL 0x00  ; Allocate local variable
  0010: STORE_LOCAL 0x01  ; Store to local variable #1
  0011: LOAD_LOCAL 0x01  ; Load local variable #1
  0012: EVAL  ; Evaluate (unquote) name
  0013: HALT  ; Halt execution
```

**Result:** `42`

---

### Multiple Channels

**Code:**
```rholang
new a, b in { a!(1) | b!(2) | for (x <- a) { x } }
```

**Disassembly:**
```
Process: proc_0
Instructions: 21

Bytecode:
  0000: NAME_CREATE 0x03  ; Create name with kind 3
  0001: ALLOC_LOCAL 0x00  ; Allocate local variable
  0002: STORE_LOCAL 0x00  ; Store to local variable #0
  0003: NAME_CREATE 0x03  ; Create name with kind 3
  0004: ALLOC_LOCAL 0x00  ; Allocate local variable
  0005: STORE_LOCAL 0x01  ; Store to local variable #1
  0006: LOAD_LOCAL 0x00  ; Load local variable #0
  0007: PUSH_INT 0x01  ; Push integer 1
  0008: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0009: POP  ; Pop top of stack
  0010: LOAD_LOCAL 0x01  ; Load local variable #1
  0011: PUSH_INT 0x02  ; Push integer 2
  0012: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0013: POP  ; Pop top of stack
  0014: LOAD_LOCAL 0x00  ; Load local variable #0
  0015: ASK 0x3, 0x0  ; Receive from channel (kind: 3)
  0016: ALLOC_LOCAL 0x00  ; Allocate local variable
  0017: STORE_LOCAL 0x02  ; Store to local variable #2
  0018: LOAD_LOCAL 0x02  ; Load local variable #2
  0019: EVAL  ; Evaluate (unquote) name
  0020: HALT  ; Halt execution
```

**Result:** `1`

---

### Complex Example (from shell tests)

**Code:**
```rholang
// Complex example using supported constructs in the MVP compiler:
// - new channels
// - send operations
// - for comprehension (receive)
// - parallel composition
// - arithmetic expressions
// - conditionals
// - collections (lists, tuples)
// - string literals
// - boolean literals

new channel1, channel2, result in {
    channel1!(42) |
    channel1!("hello") |
    channel2!(true) |
    for (x <- channel1) {
        result!(100)
    } |
    if (1 + 2 == 3) {
        result!([1, 2, 3])
    } else {
        result!(false)
    }
}

```

**Disassembly:**
```
Process: proc_0
Instructions: 46

String Pool:
  [0]: Str("hello")

Bytecode:
  0000: NAME_CREATE 0x03  ; Create name with kind 3
  0001: ALLOC_LOCAL 0x00  ; Allocate local variable
  0002: STORE_LOCAL 0x00  ; Store to local variable #0
  0003: NAME_CREATE 0x03  ; Create name with kind 3
  0004: ALLOC_LOCAL 0x00  ; Allocate local variable
  0005: STORE_LOCAL 0x01  ; Store to local variable #1
  0006: NAME_CREATE 0x03  ; Create name with kind 3
  0007: ALLOC_LOCAL 0x00  ; Allocate local variable
  0008: STORE_LOCAL 0x02  ; Store to local variable #2
  0009: LOAD_LOCAL 0x00  ; Load local variable #0
  0010: PUSH_INT 0x2a  ; Push integer 42
  0011: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0012: POP  ; Pop top of stack
  0013: LOAD_LOCAL 0x00  ; Load local variable #0
  0014: PUSH_STR 0x00  ; Push string at index 0
  0015: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0016: POP  ; Pop top of stack
  0017: LOAD_LOCAL 0x01  ; Load local variable #1
  0018: PUSH_BOOL 0x01  ; Push boolean true
  0019: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0020: POP  ; Pop top of stack
  0021: LOAD_LOCAL 0x00  ; Load local variable #0
  0022: ASK 0x3, 0x0  ; Receive from channel (kind: 3)
  0023: ALLOC_LOCAL 0x00  ; Allocate local variable
  0024: STORE_LOCAL 0x03  ; Store to local variable #3
  0025: LOAD_LOCAL 0x02  ; Load local variable #2
  0026: PUSH_INT 0x64  ; Push integer 100
  0027: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0028: POP  ; Pop top of stack
  0029: PUSH_INT 0x01  ; Push integer 1
  0030: PUSH_INT 0x02  ; Push integer 2
  0031: ADD  ; Add top two stack values
  0032: PUSH_INT 0x03  ; Push integer 3
  0033: CMP_EQ  ; Compare equal
  0034: BRANCH_FALSE 0x2a  ; Branch to 42 if false
  0035: LOAD_LOCAL 0x02  ; Load local variable #2
  0036: PUSH_INT 0x01  ; Push integer 1
  0037: PUSH_INT 0x02  ; Push integer 2
  0038: PUSH_INT 0x03  ; Push integer 3
  0039: CREATE_LIST 0x03  ; Create list with 3 elements
  0040: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0041: JUMP 0x2d  ; Jump to instruction 45
  0042: LOAD_LOCAL 0x02  ; Load local variable #2
  0043: PUSH_BOOL 0x00  ; Push boolean false
  0044: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0045: HALT  ; Halt execution
```

**Result:** `true`

---

### Maximum Complexity (from shell tests)

**Code:**
```rholang
// Maximum complexity disassembly example - exercises all supported MVP compiler features
// Comprehensive example exercising ALL supported MVP compiler constructs:
// 1. Multiple nested new declarations
// 2. Multiple send operations with different data types
// 3. Multiple for comprehensions (receives)
// 4. Parallel composition at multiple levels
// 5. All arithmetic operators: +, -, *, /
// 6. All comparison operators: ==, !=, <, >, <=, >=
// 7. Boolean operators: and, or
// 8. Nested if-then-else conditionals
// 9. Collections: lists, tuples
// 10. Various literal types: integers, strings, booleans

new main, worker1, worker2, worker3, collector, logger in {
    // Section 1: Send various data types
    main!(0) |
    main!(1) |
    main!(2) |
    main!(42) |
    main!(-100) |
    main!(32767) |
    main!("hello") |
    main!("world") |
    main!("rholang") |
    main!("disassembly") |
    main!("test") |
    main!(true) |
    main!(false) |
    
    // Section 2: Multiple workers with receives
    for (a <- worker1) {
        logger!(1)
    } |
    for (b <- worker2) {
        logger!(2)
    } |
    for (c <- worker3) {
        logger!(3)
    } |
    
    // Section 3: Arithmetic expressions (Note: % modulo not supported in MVP)
    collector!(1 + 2) |
    collector!(10 - 5) |
    collector!(3 * 4) |
    collector!(20 / 4) |
    collector!((1 + 2) * (3 + 4)) |
    collector!(((10 - 2) * 3) + 1) |
    collector!(100 / 10 / 2) |
    
    // Section 4: Comparison operations
    collector!(1 == 1) |
    collector!(1 != 2) |
    collector!(1 < 2) |
    collector!(2 > 1) |
    collector!(1 <= 1) |
    collector!(2 >= 2) |
    collector!(5 <= 10) |
    collector!(10 >= 5) |
    
    // Section 5: Boolean operations (Note: 'not' unary operator is not supported in MVP)
    collector!(true and true) |
    collector!(true or false) |
    collector!((1 < 2) and (3 > 2)) |
    collector!((1 == 1) or (2 == 3)) |
    
    // Section 6: Nested conditionals
    if (true) {
        if (1 < 2) {
            logger!("nested-true-true")
        } else {
            logger!("nested-true-false")
        }
    } else {
        if (3 > 4) {
            logger!("nested-false-true")
        } else {
            logger!("nested-false-false")
        }
    } |
    
    // Section 7: Collections - lists
    collector!([1, 2, 3]) |
    collector!([4, 5, 6, 7, 8]) |
    collector!(["a", "b", "c"]) |
    collector!([true, false, true]) |
    collector!([1, "mixed", true]) |
    collector!([[1, 2], [3, 4]]) |
    
    // Section 8: Collections - tuples
    collector!((1, 2)) |
    collector!((1, 2, 3)) |
    collector!(("x", "y", "z")) |
    collector!((true, 42, "hello")) |
    collector!(((1, 2), (3, 4))) |
    
    // Section 9: Complex nested expressions
    if ((1 + 2) == 3) {
        if ((4 * 5) > 15) {
            collector!([(1 + 1), (2 + 2), (3 + 3)])
        } else {
            collector!((100 - 50, 200 / 4))
        }
    } else {
        collector!(0)
    } |
    
    // Section 10: Nested new with more operations
    new inner1, inner2 in {
        inner1!(1) |
        inner2!(2) |
        for (i <- inner1) {
            collector!(999)
        } |
        new deepNested in {
            deepNested!("deep") |
            for (d <- deepNested) {
                logger!("received-deep")
            }
        }
    } |
    
    // Section 11: Long chain of parallel compositions
    Nil | Nil | Nil | Nil | Nil |
    main!(100) | main!(101) | main!(102) | main!(103) |
    
    // Section 12: More complex boolean chains
    if ((1 < 2) and (2 < 3) and (3 < 4)) {
        collector!("chain-true")
    } else {
        collector!("chain-false")
    } |
    
    if ((1 > 2) or (2 > 3) or (3 < 4)) {
        collector!("or-chain-true")
    } else {
        collector!("or-chain-false")
    } |
    
    // Section 13: Edge cases (integers limited to i16 range: -32768 to 32767)
    collector!(0) |
    collector!(-1) |
    collector!(-32768) |
    collector!(32767) |
    collector!("") |
    collector!([]) |
    
    // Final marker
    logger!("complete")
}

```

**Disassembly:**
```
Process: proc_0
Instructions: 506

String Pool:
  [0]: Str("hello")
  [1]: Str("world")
  [2]: Str("rholang")
  [3]: Str("disassembly")
  [4]: Str("test")
  [5]: Str("nested-true-true")
  [6]: Str("nested-true-false")
  [7]: Str("nested-false-true")
  [8]: Str("nested-false-false")
  [9]: Str("a")
  [10]: Str("b")
  [11]: Str("c")
  [12]: Str("mixed")
  [13]: Str("x")
  [14]: Str("y")
  [15]: Str("z")
  [16]: Str("deep")
  [17]: Str("received-deep")
  [18]: Str("chain-true")
  [19]: Str("chain-false")
  [20]: Str("or-chain-true")
  [21]: Str("or-chain-false")
  [22]: Str("")
  [23]: Str("complete")

Bytecode:
  0000: NAME_CREATE 0x03  ; Create name with kind 3
  0001: ALLOC_LOCAL 0x00  ; Allocate local variable
  0002: STORE_LOCAL 0x00  ; Store to local variable #0
  0003: NAME_CREATE 0x03  ; Create name with kind 3
  0004: ALLOC_LOCAL 0x00  ; Allocate local variable
  0005: STORE_LOCAL 0x01  ; Store to local variable #1
  0006: NAME_CREATE 0x03  ; Create name with kind 3
  0007: ALLOC_LOCAL 0x00  ; Allocate local variable
  0008: STORE_LOCAL 0x02  ; Store to local variable #2
  0009: NAME_CREATE 0x03  ; Create name with kind 3
  0010: ALLOC_LOCAL 0x00  ; Allocate local variable
  0011: STORE_LOCAL 0x03  ; Store to local variable #3
  0012: NAME_CREATE 0x03  ; Create name with kind 3
  0013: ALLOC_LOCAL 0x00  ; Allocate local variable
  0014: STORE_LOCAL 0x04  ; Store to local variable #4
  0015: NAME_CREATE 0x03  ; Create name with kind 3
  0016: ALLOC_LOCAL 0x00  ; Allocate local variable
  0017: STORE_LOCAL 0x05  ; Store to local variable #5
  0018: LOAD_LOCAL 0x02  ; Load local variable #2
  0019: PUSH_INT 0x00  ; Push integer 0
  0020: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0021: POP  ; Pop top of stack
  0022: LOAD_LOCAL 0x02  ; Load local variable #2
  0023: PUSH_INT 0x01  ; Push integer 1
  0024: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0025: POP  ; Pop top of stack
  0026: LOAD_LOCAL 0x02  ; Load local variable #2
  0027: PUSH_INT 0x02  ; Push integer 2
  0028: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0029: POP  ; Pop top of stack
  0030: LOAD_LOCAL 0x02  ; Load local variable #2
  0031: PUSH_INT 0x2a  ; Push integer 42
  0032: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0033: POP  ; Pop top of stack
  0034: LOAD_LOCAL 0x02  ; Load local variable #2
  0035: PUSH_INT 0xff9c  ; Push integer -100
  0036: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0037: POP  ; Pop top of stack
  0038: LOAD_LOCAL 0x02  ; Load local variable #2
  0039: PUSH_INT 0x7fff  ; Push integer 32767
  0040: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0041: POP  ; Pop top of stack
  0042: LOAD_LOCAL 0x02  ; Load local variable #2
  0043: PUSH_STR 0x00  ; Push string at index 0
  0044: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0045: POP  ; Pop top of stack
  0046: LOAD_LOCAL 0x02  ; Load local variable #2
  0047: PUSH_STR 0x01  ; Push string at index 1
  0048: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0049: POP  ; Pop top of stack
  0050: LOAD_LOCAL 0x02  ; Load local variable #2
  0051: PUSH_STR 0x02  ; Push string at index 2
  0052: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0053: POP  ; Pop top of stack
  0054: LOAD_LOCAL 0x02  ; Load local variable #2
  0055: PUSH_STR 0x03  ; Push string at index 3
  0056: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0057: POP  ; Pop top of stack
  0058: LOAD_LOCAL 0x02  ; Load local variable #2
  0059: PUSH_STR 0x04  ; Push string at index 4
  0060: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0061: POP  ; Pop top of stack
  0062: LOAD_LOCAL 0x02  ; Load local variable #2
  0063: PUSH_BOOL 0x01  ; Push boolean true
  0064: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0065: POP  ; Pop top of stack
  0066: LOAD_LOCAL 0x02  ; Load local variable #2
  0067: PUSH_BOOL 0x00  ; Push boolean false
  0068: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0069: POP  ; Pop top of stack
  0070: LOAD_LOCAL 0x03  ; Load local variable #3
  0071: ASK 0x3, 0x0  ; Receive from channel (kind: 3)
  0072: ALLOC_LOCAL 0x00  ; Allocate local variable
  0073: STORE_LOCAL 0x06  ; Store to local variable #6
  0074: LOAD_LOCAL 0x01  ; Load local variable #1
  0075: PUSH_INT 0x01  ; Push integer 1
  0076: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0077: POP  ; Pop top of stack
  0078: LOAD_LOCAL 0x04  ; Load local variable #4
  0079: ASK 0x3, 0x0  ; Receive from channel (kind: 3)
  0080: ALLOC_LOCAL 0x00  ; Allocate local variable
  0081: STORE_LOCAL 0x07  ; Store to local variable #7
  0082: LOAD_LOCAL 0x01  ; Load local variable #1
  0083: PUSH_INT 0x02  ; Push integer 2
  0084: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0085: POP  ; Pop top of stack
  0086: LOAD_LOCAL 0x05  ; Load local variable #5
  0087: ASK 0x3, 0x0  ; Receive from channel (kind: 3)
  0088: ALLOC_LOCAL 0x00  ; Allocate local variable
  0089: STORE_LOCAL 0x08  ; Store to local variable #8
  0090: LOAD_LOCAL 0x01  ; Load local variable #1
  0091: PUSH_INT 0x03  ; Push integer 3
  0092: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0093: POP  ; Pop top of stack
  0094: LOAD_LOCAL 0x00  ; Load local variable #0
  0095: PUSH_INT 0x01  ; Push integer 1
  0096: PUSH_INT 0x02  ; Push integer 2
  0097: ADD  ; Add top two stack values
  0098: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0099: POP  ; Pop top of stack
  0100: LOAD_LOCAL 0x00  ; Load local variable #0
  0101: PUSH_INT 0x0a  ; Push integer 10
  0102: PUSH_INT 0x05  ; Push integer 5
  0103: SUB  ; Subtract top two stack values
  0104: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0105: POP  ; Pop top of stack
  0106: LOAD_LOCAL 0x00  ; Load local variable #0
  0107: PUSH_INT 0x03  ; Push integer 3
  0108: PUSH_INT 0x04  ; Push integer 4
  0109: MUL  ; Multiply top two stack values
  0110: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0111: POP  ; Pop top of stack
  0112: LOAD_LOCAL 0x00  ; Load local variable #0
  0113: PUSH_INT 0x14  ; Push integer 20
  0114: PUSH_INT 0x04  ; Push integer 4
  0115: DIV  ; Divide top two stack values
  0116: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0117: POP  ; Pop top of stack
  0118: LOAD_LOCAL 0x00  ; Load local variable #0
  0119: PUSH_INT 0x01  ; Push integer 1
  0120: PUSH_INT 0x02  ; Push integer 2
  0121: ADD  ; Add top two stack values
  0122: PUSH_INT 0x03  ; Push integer 3
  0123: PUSH_INT 0x04  ; Push integer 4
  0124: ADD  ; Add top two stack values
  0125: MUL  ; Multiply top two stack values
  0126: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0127: POP  ; Pop top of stack
  0128: LOAD_LOCAL 0x00  ; Load local variable #0
  0129: PUSH_INT 0x0a  ; Push integer 10
  0130: PUSH_INT 0x02  ; Push integer 2
  0131: SUB  ; Subtract top two stack values
  0132: PUSH_INT 0x03  ; Push integer 3
  0133: MUL  ; Multiply top two stack values
  0134: PUSH_INT 0x01  ; Push integer 1
  0135: ADD  ; Add top two stack values
  0136: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0137: POP  ; Pop top of stack
  0138: LOAD_LOCAL 0x00  ; Load local variable #0
  0139: PUSH_INT 0x64  ; Push integer 100
  0140: PUSH_INT 0x0a  ; Push integer 10
  0141: DIV  ; Divide top two stack values
  0142: PUSH_INT 0x02  ; Push integer 2
  0143: DIV  ; Divide top two stack values
  0144: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0145: POP  ; Pop top of stack
  0146: LOAD_LOCAL 0x00  ; Load local variable #0
  0147: PUSH_INT 0x01  ; Push integer 1
  0148: PUSH_INT 0x01  ; Push integer 1
  0149: CMP_EQ  ; Compare equal
  0150: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0151: POP  ; Pop top of stack
  0152: LOAD_LOCAL 0x00  ; Load local variable #0
  0153: PUSH_INT 0x01  ; Push integer 1
  0154: PUSH_INT 0x02  ; Push integer 2
  0155: CMP_NEQ  ; Compare not equal
  0156: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0157: POP  ; Pop top of stack
  0158: LOAD_LOCAL 0x00  ; Load local variable #0
  0159: PUSH_INT 0x01  ; Push integer 1
  0160: PUSH_INT 0x02  ; Push integer 2
  0161: CMP_LT  ; Compare less than
  0162: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0163: POP  ; Pop top of stack
  0164: LOAD_LOCAL 0x00  ; Load local variable #0
  0165: PUSH_INT 0x02  ; Push integer 2
  0166: PUSH_INT 0x01  ; Push integer 1
  0167: CMP_GT  ; Compare greater than
  0168: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0169: POP  ; Pop top of stack
  0170: LOAD_LOCAL 0x00  ; Load local variable #0
  0171: PUSH_INT 0x01  ; Push integer 1
  0172: PUSH_INT 0x01  ; Push integer 1
  0173: CMP_LTE  ; Compare less than or equal
  0174: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0175: POP  ; Pop top of stack
  0176: LOAD_LOCAL 0x00  ; Load local variable #0
  0177: PUSH_INT 0x02  ; Push integer 2
  0178: PUSH_INT 0x02  ; Push integer 2
  0179: CMP_GTE  ; Compare greater than or equal
  0180: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0181: POP  ; Pop top of stack
  0182: LOAD_LOCAL 0x00  ; Load local variable #0
  0183: PUSH_INT 0x05  ; Push integer 5
  0184: PUSH_INT 0x0a  ; Push integer 10
  0185: CMP_LTE  ; Compare less than or equal
  0186: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0187: POP  ; Pop top of stack
  0188: LOAD_LOCAL 0x00  ; Load local variable #0
  0189: PUSH_INT 0x0a  ; Push integer 10
  0190: PUSH_INT 0x05  ; Push integer 5
  0191: CMP_GTE  ; Compare greater than or equal
  0192: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0193: POP  ; Pop top of stack
  0194: LOAD_LOCAL 0x00  ; Load local variable #0
  0195: PUSH_BOOL 0x01  ; Push boolean true
  0196: PUSH_BOOL 0x01  ; Push boolean true
  0197: AND  ; Logical AND
  0198: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0199: POP  ; Pop top of stack
  0200: LOAD_LOCAL 0x00  ; Load local variable #0
  0201: PUSH_BOOL 0x01  ; Push boolean true
  0202: PUSH_BOOL 0x00  ; Push boolean false
  0203: OR  ; Logical OR
  0204: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0205: POP  ; Pop top of stack
  0206: LOAD_LOCAL 0x00  ; Load local variable #0
  0207: PUSH_INT 0x01  ; Push integer 1
  0208: PUSH_INT 0x02  ; Push integer 2
  0209: CMP_LT  ; Compare less than
  0210: PUSH_INT 0x03  ; Push integer 3
  0211: PUSH_INT 0x02  ; Push integer 2
  0212: CMP_GT  ; Compare greater than
  0213: AND  ; Logical AND
  0214: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0215: POP  ; Pop top of stack
  0216: LOAD_LOCAL 0x00  ; Load local variable #0
  0217: PUSH_INT 0x01  ; Push integer 1
  0218: PUSH_INT 0x01  ; Push integer 1
  0219: CMP_EQ  ; Compare equal
  0220: PUSH_INT 0x02  ; Push integer 2
  0221: PUSH_INT 0x03  ; Push integer 3
  0222: CMP_EQ  ; Compare equal
  0223: OR  ; Logical OR
  0224: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0225: POP  ; Pop top of stack
  0226: PUSH_BOOL 0x01  ; Push boolean true
  0227: BRANCH_FALSE 0xf0  ; Branch to 240 if false
  0228: PUSH_INT 0x01  ; Push integer 1
  0229: PUSH_INT 0x02  ; Push integer 2
  0230: CMP_LT  ; Compare less than
  0231: BRANCH_FALSE 0xec  ; Branch to 236 if false
  0232: LOAD_LOCAL 0x01  ; Load local variable #1
  0233: PUSH_STR 0x05  ; Push string at index 5
  0234: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0235: JUMP 0xef  ; Jump to instruction 239
  0236: LOAD_LOCAL 0x01  ; Load local variable #1
  0237: PUSH_STR 0x06  ; Push string at index 6
  0238: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0239: JUMP 0xfb  ; Jump to instruction 251
  0240: PUSH_INT 0x03  ; Push integer 3
  0241: PUSH_INT 0x04  ; Push integer 4
  0242: CMP_GT  ; Compare greater than
  0243: BRANCH_FALSE 0xf8  ; Branch to 248 if false
  0244: LOAD_LOCAL 0x01  ; Load local variable #1
  0245: PUSH_STR 0x07  ; Push string at index 7
  0246: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0247: JUMP 0xfb  ; Jump to instruction 251
  0248: LOAD_LOCAL 0x01  ; Load local variable #1
  0249: PUSH_STR 0x08  ; Push string at index 8
  0250: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0251: POP  ; Pop top of stack
  0252: LOAD_LOCAL 0x00  ; Load local variable #0
  0253: PUSH_INT 0x01  ; Push integer 1
  0254: PUSH_INT 0x02  ; Push integer 2
  0255: PUSH_INT 0x03  ; Push integer 3
  0256: CREATE_LIST 0x03  ; Create list with 3 elements
  0257: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0258: POP  ; Pop top of stack
  0259: LOAD_LOCAL 0x00  ; Load local variable #0
  0260: PUSH_INT 0x04  ; Push integer 4
  0261: PUSH_INT 0x05  ; Push integer 5
  0262: PUSH_INT 0x06  ; Push integer 6
  0263: PUSH_INT 0x07  ; Push integer 7
  0264: PUSH_INT 0x08  ; Push integer 8
  0265: CREATE_LIST 0x05  ; Create list with 5 elements
  0266: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0267: POP  ; Pop top of stack
  0268: LOAD_LOCAL 0x00  ; Load local variable #0
  0269: PUSH_STR 0x09  ; Push string at index 9
  0270: PUSH_STR 0x0a  ; Push string at index 10
  0271: PUSH_STR 0x0b  ; Push string at index 11
  0272: CREATE_LIST 0x03  ; Create list with 3 elements
  0273: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0274: POP  ; Pop top of stack
  0275: LOAD_LOCAL 0x00  ; Load local variable #0
  0276: PUSH_BOOL 0x01  ; Push boolean true
  0277: PUSH_BOOL 0x00  ; Push boolean false
  0278: PUSH_BOOL 0x01  ; Push boolean true
  0279: CREATE_LIST 0x03  ; Create list with 3 elements
  0280: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0281: POP  ; Pop top of stack
  0282: LOAD_LOCAL 0x00  ; Load local variable #0
  0283: PUSH_INT 0x01  ; Push integer 1
  0284: PUSH_STR 0x0c  ; Push string at index 12
  0285: PUSH_BOOL 0x01  ; Push boolean true
  0286: CREATE_LIST 0x03  ; Create list with 3 elements
  0287: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0288: POP  ; Pop top of stack
  0289: LOAD_LOCAL 0x00  ; Load local variable #0
  0290: PUSH_INT 0x01  ; Push integer 1
  0291: PUSH_INT 0x02  ; Push integer 2
  0292: CREATE_LIST 0x02  ; Create list with 2 elements
  0293: PUSH_INT 0x03  ; Push integer 3
  0294: PUSH_INT 0x04  ; Push integer 4
  0295: CREATE_LIST 0x02  ; Create list with 2 elements
  0296: CREATE_LIST 0x02  ; Create list with 2 elements
  0297: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0298: POP  ; Pop top of stack
  0299: LOAD_LOCAL 0x00  ; Load local variable #0
  0300: PUSH_INT 0x01  ; Push integer 1
  0301: PUSH_INT 0x02  ; Push integer 2
  0302: CREATE_TUPLE 0x02  ; Create tuple with 2 elements
  0303: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0304: POP  ; Pop top of stack
  0305: LOAD_LOCAL 0x00  ; Load local variable #0
  0306: PUSH_INT 0x01  ; Push integer 1
  0307: PUSH_INT 0x02  ; Push integer 2
  0308: PUSH_INT 0x03  ; Push integer 3
  0309: CREATE_TUPLE 0x03  ; Create tuple with 3 elements
  0310: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0311: POP  ; Pop top of stack
  0312: LOAD_LOCAL 0x00  ; Load local variable #0
  0313: PUSH_STR 0x0d  ; Push string at index 13
  0314: PUSH_STR 0x0e  ; Push string at index 14
  0315: PUSH_STR 0x0f  ; Push string at index 15
  0316: CREATE_TUPLE 0x03  ; Create tuple with 3 elements
  0317: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0318: POP  ; Pop top of stack
  0319: LOAD_LOCAL 0x00  ; Load local variable #0
  0320: PUSH_BOOL 0x01  ; Push boolean true
  0321: PUSH_INT 0x2a  ; Push integer 42
  0322: PUSH_STR 0x00  ; Push string at index 0
  0323: CREATE_TUPLE 0x03  ; Create tuple with 3 elements
  0324: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0325: POP  ; Pop top of stack
  0326: LOAD_LOCAL 0x00  ; Load local variable #0
  0327: PUSH_INT 0x01  ; Push integer 1
  0328: PUSH_INT 0x02  ; Push integer 2
  0329: CREATE_TUPLE 0x02  ; Create tuple with 2 elements
  0330: PUSH_INT 0x03  ; Push integer 3
  0331: PUSH_INT 0x04  ; Push integer 4
  0332: CREATE_TUPLE 0x02  ; Create tuple with 2 elements
  0333: CREATE_TUPLE 0x02  ; Create tuple with 2 elements
  0334: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0335: POP  ; Pop top of stack
  0336: PUSH_INT 0x01  ; Push integer 1
  0337: PUSH_INT 0x02  ; Push integer 2
  0338: ADD  ; Add top two stack values
  0339: PUSH_INT 0x03  ; Push integer 3
  0340: CMP_EQ  ; Compare equal
  0341: BRANCH_FALSE 0x173  ; Branch to 371 if false
  0342: PUSH_INT 0x04  ; Push integer 4
  0343: PUSH_INT 0x05  ; Push integer 5
  0344: MUL  ; Multiply top two stack values
  0345: PUSH_INT 0x0f  ; Push integer 15
  0346: CMP_GT  ; Compare greater than
  0347: BRANCH_FALSE 0x169  ; Branch to 361 if false
  0348: LOAD_LOCAL 0x00  ; Load local variable #0
  0349: PUSH_INT 0x01  ; Push integer 1
  0350: PUSH_INT 0x01  ; Push integer 1
  0351: ADD  ; Add top two stack values
  0352: PUSH_INT 0x02  ; Push integer 2
  0353: PUSH_INT 0x02  ; Push integer 2
  0354: ADD  ; Add top two stack values
  0355: PUSH_INT 0x03  ; Push integer 3
  0356: PUSH_INT 0x03  ; Push integer 3
  0357: ADD  ; Add top two stack values
  0358: CREATE_LIST 0x03  ; Create list with 3 elements
  0359: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0360: JUMP 0x172  ; Jump to instruction 370
  0361: LOAD_LOCAL 0x00  ; Load local variable #0
  0362: PUSH_INT 0x64  ; Push integer 100
  0363: PUSH_INT 0x32  ; Push integer 50
  0364: SUB  ; Subtract top two stack values
  0365: PUSH_INT 0xc8  ; Push integer 200
  0366: PUSH_INT 0x04  ; Push integer 4
  0367: DIV  ; Divide top two stack values
  0368: CREATE_TUPLE 0x02  ; Create tuple with 2 elements
  0369: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0370: JUMP 0x176  ; Jump to instruction 374
  0371: LOAD_LOCAL 0x00  ; Load local variable #0
  0372: PUSH_INT 0x00  ; Push integer 0
  0373: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0374: POP  ; Pop top of stack
  0375: NAME_CREATE 0x03  ; Create name with kind 3
  0376: ALLOC_LOCAL 0x00  ; Allocate local variable
  0377: STORE_LOCAL 0x09  ; Store to local variable #9
  0378: NAME_CREATE 0x03  ; Create name with kind 3
  0379: ALLOC_LOCAL 0x00  ; Allocate local variable
  0380: STORE_LOCAL 0x0a  ; Store to local variable #10
  0381: LOAD_LOCAL 0x09  ; Load local variable #9
  0382: PUSH_INT 0x01  ; Push integer 1
  0383: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0384: POP  ; Pop top of stack
  0385: LOAD_LOCAL 0x0a  ; Load local variable #10
  0386: PUSH_INT 0x02  ; Push integer 2
  0387: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0388: POP  ; Pop top of stack
  0389: LOAD_LOCAL 0x09  ; Load local variable #9
  0390: ASK 0x3, 0x0  ; Receive from channel (kind: 3)
  0391: ALLOC_LOCAL 0x00  ; Allocate local variable
  0392: STORE_LOCAL 0x0b  ; Store to local variable #11
  0393: LOAD_LOCAL 0x00  ; Load local variable #0
  0394: PUSH_INT 0x3e7  ; Push integer 999
  0395: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0396: POP  ; Pop top of stack
  0397: NAME_CREATE 0x03  ; Create name with kind 3
  0398: ALLOC_LOCAL 0x00  ; Allocate local variable
  0399: STORE_LOCAL 0x0c  ; Store to local variable #12
  0400: LOAD_LOCAL 0x0c  ; Load local variable #12
  0401: PUSH_STR 0x10  ; Push string at index 16
  0402: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0403: POP  ; Pop top of stack
  0404: LOAD_LOCAL 0x0c  ; Load local variable #12
  0405: ASK 0x3, 0x0  ; Receive from channel (kind: 3)
  0406: ALLOC_LOCAL 0x00  ; Allocate local variable
  0407: STORE_LOCAL 0x0d  ; Store to local variable #13
  0408: LOAD_LOCAL 0x01  ; Load local variable #1
  0409: PUSH_STR 0x11  ; Push string at index 17
  0410: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0411: POP  ; Pop top of stack
  0412: PUSH_NIL  ; Push nil
  0413: POP  ; Pop top of stack
  0414: PUSH_NIL  ; Push nil
  0415: POP  ; Pop top of stack
  0416: PUSH_NIL  ; Push nil
  0417: POP  ; Pop top of stack
  0418: PUSH_NIL  ; Push nil
  0419: POP  ; Pop top of stack
  0420: PUSH_NIL  ; Push nil
  0421: POP  ; Pop top of stack
  0422: LOAD_LOCAL 0x02  ; Load local variable #2
  0423: PUSH_INT 0x64  ; Push integer 100
  0424: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0425: POP  ; Pop top of stack
  0426: LOAD_LOCAL 0x02  ; Load local variable #2
  0427: PUSH_INT 0x65  ; Push integer 101
  0428: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0429: POP  ; Pop top of stack
  0430: LOAD_LOCAL 0x02  ; Load local variable #2
  0431: PUSH_INT 0x66  ; Push integer 102
  0432: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0433: POP  ; Pop top of stack
  0434: LOAD_LOCAL 0x02  ; Load local variable #2
  0435: PUSH_INT 0x67  ; Push integer 103
  0436: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0437: POP  ; Pop top of stack
  0438: PUSH_INT 0x01  ; Push integer 1
  0439: PUSH_INT 0x02  ; Push integer 2
  0440: CMP_LT  ; Compare less than
  0441: PUSH_INT 0x02  ; Push integer 2
  0442: PUSH_INT 0x03  ; Push integer 3
  0443: CMP_LT  ; Compare less than
  0444: AND  ; Logical AND
  0445: PUSH_INT 0x03  ; Push integer 3
  0446: PUSH_INT 0x04  ; Push integer 4
  0447: CMP_LT  ; Compare less than
  0448: AND  ; Logical AND
  0449: BRANCH_FALSE 0x1c6  ; Branch to 454 if false
  0450: LOAD_LOCAL 0x00  ; Load local variable #0
  0451: PUSH_STR 0x12  ; Push string at index 18
  0452: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0453: JUMP 0x1c9  ; Jump to instruction 457
  0454: LOAD_LOCAL 0x00  ; Load local variable #0
  0455: PUSH_STR 0x13  ; Push string at index 19
  0456: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0457: POP  ; Pop top of stack
  0458: PUSH_INT 0x01  ; Push integer 1
  0459: PUSH_INT 0x02  ; Push integer 2
  0460: CMP_GT  ; Compare greater than
  0461: PUSH_INT 0x02  ; Push integer 2
  0462: PUSH_INT 0x03  ; Push integer 3
  0463: CMP_GT  ; Compare greater than
  0464: OR  ; Logical OR
  0465: PUSH_INT 0x03  ; Push integer 3
  0466: PUSH_INT 0x04  ; Push integer 4
  0467: CMP_LT  ; Compare less than
  0468: OR  ; Logical OR
  0469: BRANCH_FALSE 0x1da  ; Branch to 474 if false
  0470: LOAD_LOCAL 0x00  ; Load local variable #0
  0471: PUSH_STR 0x14  ; Push string at index 20
  0472: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0473: JUMP 0x1dd  ; Jump to instruction 477
  0474: LOAD_LOCAL 0x00  ; Load local variable #0
  0475: PUSH_STR 0x15  ; Push string at index 21
  0476: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0477: POP  ; Pop top of stack
  0478: LOAD_LOCAL 0x00  ; Load local variable #0
  0479: PUSH_INT 0x00  ; Push integer 0
  0480: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0481: POP  ; Pop top of stack
  0482: LOAD_LOCAL 0x00  ; Load local variable #0
  0483: PUSH_INT 0xffff  ; Push integer -1
  0484: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0485: POP  ; Pop top of stack
  0486: LOAD_LOCAL 0x00  ; Load local variable #0
  0487: PUSH_INT 0x8000  ; Push integer -32768
  0488: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0489: POP  ; Pop top of stack
  0490: LOAD_LOCAL 0x00  ; Load local variable #0
  0491: PUSH_INT 0x7fff  ; Push integer 32767
  0492: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0493: POP  ; Pop top of stack
  0494: LOAD_LOCAL 0x00  ; Load local variable #0
  0495: PUSH_STR 0x16  ; Push string at index 22
  0496: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0497: POP  ; Pop top of stack
  0498: LOAD_LOCAL 0x00  ; Load local variable #0
  0499: CREATE_LIST 0x00  ; Create list with 0 elements
  0500: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0501: POP  ; Pop top of stack
  0502: LOAD_LOCAL 0x01  ; Load local variable #1
  0503: PUSH_STR 0x17  ; Push string at index 23
  0504: TELL 0x3, 0x0  ; Send on channel (kind: 3)
  0505: HALT  ; Halt execution
```

**Result:** `true`

---

