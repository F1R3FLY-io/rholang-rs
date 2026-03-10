# Rholang Interpreter Testing Strategy

This document describes the testing strategy, required coverage, test structure, and guidelines for writing tests in the Rholang interpreter project.

## Table of Contents

1. [Testing Philosophy](#testing-philosophy)
2. [Test Coverage Requirements](#test-coverage-requirements)
3. [Test Structure](#test-structure)
4. [Test Categories](#test-categories)
5. [Test Details by Crate](#test-details-by-crate)
6. [Testing Patterns](#testing-patterns)
7. [Running Tests](#running-tests)
8. [LLM Guidelines for Writing Tests](#llm-guidelines-for-writing-tests)

---

## Testing Philosophy

The Rholang interpreter follows a **Test-Driven Development (TDD)** approach where:

1. **Tests are developed in parallel with implementation** - Unit tests are written alongside code changes
2. **Behavior over implementation** - Tests focus on observable behavior, not internal implementation details
3. **Comprehensive coverage** - Both success and error cases are tested
4. **No logging in tests** - Tests use assertions, not log output for validation
5. **Mock external dependencies** - External systems are consistently mocked

---

## Test Coverage Requirements

### Minimum Coverage Standards

| Category | Required Coverage |
|----------|------------------|
| Core VM Operations | 100% |
| RSpace Interface | 100% |
| Process Execution | 95%+ |
| Bytecode Instructions | 100% |
| Error Handling | 90%+ |
| Compiler Codegen | 95%+ |
| Parser Integration | 90%+ |

### What Must Be Tested

1. **All public APIs** - Every public function/method must have tests
2. **All opcodes** - Every bytecode opcode must have execution tests
3. **All value types** - Every Value variant must be tested for storage and operations
4. **State transitions** - All process state transitions (Wait → Ready → Value/Error)
5. **Error conditions** - All error paths with appropriate error messages
6. **Edge cases** - Empty collections, zero values, max/min boundaries
7. **Concurrency** - Thread safety for shared resources (RSpace)

---

## Test Structure

### Directory Layout

```
rholang/
├── rholang-bytecode/
│   ├── src/
│   │   └── core/
│   │       └── *.rs         # Inline #[cfg(test)] modules
│   └── tests/
│       ├── memory_benchmarks.rs
│       └── test_types.rs
├── rholang-compiler/
│   ├── src/
│   │   └── *.rs             # Inline #[cfg(test)] modules
│   └── tests/
│       ├── channels.rs      # Channel operation tests
│       ├── collections.rs   # Collection type tests
│       ├── control_flow.rs  # If/else tests
│       ├── expressions.rs   # Arithmetic/logical tests
│       ├── literals.rs      # Literal value tests
│       ├── variables.rs     # Variable binding tests
│       └── common/mod.rs    # Shared test helpers
├── rholang-process/
│   ├── src/
│   │   ├── parameter.rs     # Inline tests
│   │   └── process.rs
│   └── tests/
│       └── parameter_tests.rs
├── rholang-rspace/
│   ├── src/
│   │   ├── in_memory.rs     # Inline tests
│   │   └── path_map.rs      # Inline tests
│   └── tests/
│       └── rspace_rules_tests.rs
├── rholang-vm/
│   ├── src/
│   │   ├── entry.rs         # Inline tests
│   │   ├── value.rs         # Inline tests
│   │   └── vm.rs
│   └── tests/
│       ├── arithmetic_tests.rs
│       ├── bytecode_contract_tests.rs
│       ├── collection_diff_tests.rs
│       ├── collections_tests.rs
│       ├── comparison_tests.rs
│       ├── control_flow_tests.rs
│       ├── examples_vm_tests.rs
│       ├── minimal_vm_tests.rs
│       ├── parallel_exec_tests.rs
│       └── rspace_operations_tests.rs
└── rholang-shell/
    └── tests/
        ├── interpreter_tests.rs
        ├── compiler_provider_tests.rs
        └── ...
```

### Test File Naming Conventions

| Type | Pattern | Example |
|------|---------|---------|
| Integration tests | `*_tests.rs` | `parameter_tests.rs` |
| Unit tests (inline) | `#[cfg(test)] mod tests` | Inside `parameter.rs` |
| Benchmark tests | `*_benchmarks.rs` | `memory_benchmarks.rs` |
| Golden tests | `golden_*.rs` | `golden_shell.rs` |

---

## Test Categories

### 1. Unit Tests (Inline)

Located inside source files using `#[cfg(test)]` module pattern:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_behavior() {
        // Arrange
        let input = create_test_input();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected_value);
    }
}
```

**Use for:**
- Testing private functions
- Testing simple, self-contained logic
- Testing struct methods

### 2. Integration Tests

Located in `tests/` directory at crate root:

```rust
// tests/feature_tests.rs
use crate_name::{PublicApi, Types};

#[test]
fn test_end_to_end_scenario() {
    // Setup
    let mut system = setup_test_system();

    // Execute workflow
    system.operation_a();
    system.operation_b();

    // Verify final state
    assert!(system.is_valid_state());
}
```

**Use for:**
- Testing public API contracts
- Testing cross-module interactions
- Testing complete workflows

### 3. Property-Based Tests

Using `rstest` for parameterized testing:

```rust
use rstest::rstest;

#[rstest]
#[case(0, 0, 0)]
#[case(1, 2, 3)]
#[case(-1, 1, 0)]
#[case(i64::MAX, 0, i64::MAX)]
fn test_addition(#[case] a: i64, #[case] b: i64, #[case] expected: i64) {
    assert_eq!(add(a, b), expected);
}
```

### 4. Error Case Tests

Always test both success and failure paths:

```rust
#[test]
fn test_operation_fails_with_invalid_input() {
    let result = operation_that_can_fail(invalid_input);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expected error message"));
}
```

---

## Test Details by Crate

### rholang-bytecode (42 tests)

| Module | Tests | Description |
|--------|-------|-------------|
| `constants` | 14 | Constant pool, string interning, serialization |
| `instructions` | 18 | Instruction encoding, compression, labels |
| `module` | 6 | Bytecode module creation and stats |
| `opcodes` | 4 | Opcode properties and flags |
| `types` | 5 | Environment, tagged pointers, RSpace types |

### rholang-compiler (180+ tests)

| Test File | Tests | Description |
|-----------|-------|-------------|
| `channels.rs` | 24 | Channel creation, send, receive |
| `collections.rs` | 18 | List, tuple, map literals |
| `control_flow.rs` | 39 | If/else branching |
| `expressions.rs` | 56 | Arithmetic, logical, comparison |
| `literals.rs` | 17 | Int, bool, string, nil |
| `variables.rs` | 26 | Variable binding and scope |

### rholang-vm (58+ tests)

| Test File | Tests | Description |
|-----------|-------|-------------|
| `arithmetic_tests.rs` | ~10 | ADD, SUB, MUL, DIV operations |
| `comparison_tests.rs` | ~10 | EQ, NE, LT, GT, LE, GE |
| `collections_tests.rs` | ~10 | List, tuple, map operations |
| `control_flow_tests.rs` | ~8 | JMP, JZ, JNZ branching |
| `rspace_operations_tests.rs` | ~10 | TELL, ASK, PEEK, NEW |
| `parallel_exec_tests.rs` | ~5 | Concurrent process execution |

### rholang-process (57 tests)

| Test File | Tests | Description |
|-----------|-------|-------------|
| `parameter.rs` (inline) | 5 | Parameter creation, equality |
| `parameter_tests.rs` | 52 | Parameter solving with RSpace |

### rholang-rspace (114+ tests)

| Test File | Tests | Description |
|-----------|-------|-------------|
| `lib.rs` (inline) | 6 | Default RSpace, basic operations |
| `in_memory.rs` (inline) | ~14 | InMemoryRSpace implementation |
| `path_map.rs` (inline) | ~10 | PathMapRSpace implementation |
| `rspace_rules_tests.rs` | 90 | Comprehensive RSpace rule tests |

---

## Testing Patterns

### Pattern 1: Test Helper Functions

Create helper functions to reduce boilerplate:

```rust
/// Helper to create a simple HALT process
fn halt_process(name: &str) -> Process {
    Process::new(vec![Instruction::nullary(Opcode::HALT)], name)
}

/// Helper to create a shared RSpace
fn shared_rspace() -> SharedRSpace {
    Arc::new(Mutex::new(
        Box::new(DefaultRSpace::default()) as Box<dyn RSpace>
    ))
}

/// Helper to create a VM with shared RSpace
fn vm_with_shared_rspace(rspace: SharedRSpace) -> VM {
    VM::with_shared_rspace(rspace)
}
```

### Pattern 2: Test Organization with Comments

Group related tests with section comments:

```rust
// ============================================================================
// Test: Parameter creation and basic properties
// ============================================================================

#[test]
fn test_parameter_new() { ... }

#[test]
fn test_parameter_new_with_string() { ... }

// ============================================================================
// Test: Parameter is_solved with channel values
// ============================================================================

#[test]
fn test_parameter_solved_with_int_in_channel() { ... }
```

### Pattern 3: Macro-Based Test Generation

Use macros to test multiple implementations:

```rust
macro_rules! rspace_interface_tests {
    ($rspace_type:ty, $mod_name:ident) => {
        mod $mod_name {
            use super::*;

            fn make_rspace() -> Box<dyn RSpace> {
                Box::new(<$rspace_type>::new())
            }

            #[test]
            fn test_tell_stores_value() -> Result<()> {
                let mut rspace = make_rspace();
                rspace.tell("test", Value::Int(42))?;
                assert_eq!(rspace.peek("test")?, Some(Value::Int(42)));
                Ok(())
            }
            // ... more tests
        }
    };
}

// Generate tests for both implementations
rspace_interface_tests!(InMemoryRSpace, in_memory_rspace_tests);
rspace_interface_tests!(PathMapRSpace, path_map_rspace_tests);
```

### Pattern 4: State Transition Testing

Test all state transitions explicitly:

```rust
#[test]
fn test_process_state_transitions() {
    // Ready -> Value (success)
    let mut process = Process::new(code, "test");
    assert!(matches!(process.state, ProcessState::Ready));
    process.execute().unwrap();
    assert!(matches!(process.state, ProcessState::Value(_)));
}

#[test]
fn test_process_state_error() {
    // Ready -> Error (failure)
    let mut process = Process::new(failing_code, "test");
    let _ = process.execute();
    assert!(matches!(process.state, ProcessState::Error(_)));
}
```

### Pattern 5: Async Test Pattern

For async code using tokio:

```rust
#[tokio::test]
async fn test_async_operation() -> Result<()> {
    let provider = Provider::new()?;
    let result = provider.interpret("1 + 2").await;
    assert!(result.is_success());
    Ok(())
}
```

---

## Running Tests

### Commands

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p rholang-vm

# Run specific test file
cargo test -p rholang-process --test parameter_tests

# Run tests matching pattern
cargo test parameter

# Run with output
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored

# Run benchmarks
cargo bench
```

### Pre-Commit Checklist

```bash
# Full validation pipeline
cargo fmt --check && \
cargo clippy --all-features --all-targets -- -D warnings && \
cargo test && \
cargo build
```

---

## LLM Guidelines for Writing Tests

### REQUIRED: Follow These Rules

1. **ALWAYS read existing test files** before writing new tests to match patterns
2. **ALWAYS use descriptive test names** following `test_<what>_<when/condition>` pattern
3. **ALWAYS test both success and error cases** for any function that can fail
4. **ALWAYS use `Result<()>` return type** for tests that use `?` operator
5. **ALWAYS group related tests** with section comment headers
6. **NEVER use logging** (`println!`, `dbg!`) to verify behavior - use assertions
7. **NEVER leave TODO comments** in test code - implement or skip with `#[ignore]`

### Test Structure Template

```rust
// tests/<feature>_tests.rs
//! Tests for <feature description>
//!
//! This test module verifies:
//! - <bullet point 1>
//! - <bullet point 2>
//! - <bullet point 3>

use anyhow::Result;
use crate_name::{Type1, Type2, function_to_test};

// ============================================================================
// Test Helpers
// ============================================================================

fn create_test_fixture() -> TestType {
    // Setup code
}

// ============================================================================
// Test: <Category Name>
// ============================================================================

#[test]
fn test_<behavior>_<condition>() -> Result<()> {
    // Arrange
    let input = create_test_fixture();

    // Act
    let result = function_to_test(input)?;

    // Assert
    assert_eq!(result, expected);
    Ok(())
}

#[test]
fn test_<behavior>_fails_when_<condition>() {
    // Arrange
    let invalid_input = create_invalid_fixture();

    // Act
    let result = function_to_test(invalid_input);

    // Assert
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expected error"));
}
```

### What to Test for New Features

When implementing a new feature, create tests for:

1. **Basic functionality** - Happy path works
2. **Edge cases** - Empty inputs, zero values, max values
3. **Error conditions** - Invalid inputs return proper errors
4. **Type variations** - All supported types work
5. **State changes** - State transitions correctly
6. **Integration** - Works with other components

### Test Naming Convention

```rust
// Good names
test_parameter_new()                              // Basic creation
test_parameter_new_with_string()                  // Variation
test_parameter_solved_when_channel_nonempty()     // State condition
test_parameter_unsolved_when_process_in_ready()   // Negative state
test_process_with_unsolved_parameters_cannot_execute()  // Blocked behavior

// Bad names (avoid)
test1()                    // No description
test_param()               // Too vague
test_it_works()            // Uninformative
testParameterSolving()     // Wrong naming convention
```

### Assertion Best Practices

```rust
// GOOD: Specific assertions
assert_eq!(result, Some(Value::Int(42)));
assert!(result.is_err());
assert!(matches!(state, ProcessState::Value(_)));
assert_eq!(list.len(), 3);

// BAD: Vague assertions
assert!(result.is_some());  // Doesn't verify value
assert!(true);              // Meaningless
```

### When to Use Different Test Types

| Scenario | Test Type | Location |
|----------|-----------|----------|
| Testing private function | Unit (inline) | `src/module.rs` |
| Testing public API | Integration | `tests/api_tests.rs` |
| Testing multiple implementations | Macro-generated | `tests/impl_tests.rs` |
| Testing async code | Tokio test | `tests/async_tests.rs` |
| Testing parameter variations | rstest parametrized | Any test file |

### Coverage Checklist for New Types

When adding a new type (like `Entry`), test:

- [ ] All constructors
- [ ] All accessor methods
- [ ] `Clone`, `Debug`, `PartialEq` implementations
- [ ] Serialization/deserialization (if applicable)
- [ ] All variants/states
- [ ] Conversion to/from other types
- [ ] Error conditions

### Coverage Checklist for New Functions

When adding a new function, test:

- [ ] Normal inputs return expected outputs
- [ ] Empty/zero inputs handled correctly
- [ ] Invalid inputs return proper errors
- [ ] Boundary conditions (min/max values)
- [ ] Integration with dependent systems

---

## Summary

The Rholang project maintains high test quality through:

1. **Comprehensive coverage** - All code paths tested
2. **Consistent patterns** - Helpers, macros, organization
3. **Clear naming** - Self-documenting test names
4. **Separation** - Unit vs integration tests
5. **Automation** - CI/CD validation pipeline

When adding new features, follow existing patterns and ensure all edge cases are covered.
