# Rholang Virtual Machine

A bytecode virtual machine for executing Rholang code, based on the design in [BYTECODE_DESIGN.md](../docs/BYTECODE_DESIGN.md).

## Architecture

The Rholang VM consists of several components:

1. **Bytecode Format**: Defines the instruction set for the VM, including computational operations, control flow, and RSpace interactions.

2. **Stack-Based VM**: A stack-based virtual machine that executes bytecode instructions.

3. **RSpace**: Different storage types for data (memory/store, sequential/concurrent).

4. **Compiler**: Translates Rholang code to bytecode.

5. **Interpreter Provider**: Integrates the VM with the shell.

### Component Diagram

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Rholang   │     │   Bytecode  │     │     VM      │
│    Code     │────▶│   Compiler  │────▶│  Execution  │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
                                              ▼
                                        ┌─────────────┐
                                        │   RSpace    │
                                        │   Storage   │
                                        └─────────────┘
```

## Usage

### Basic Usage

```rust
use anyhow::Result;
use rholang_vm::{
    compiler::RholangCompiler,
    vm::VM,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Create a compiler
    let mut compiler = RholangCompiler::new()?;
    
    // Compile Rholang code to bytecode
    let bytecode = compiler.compile("1 + 2")?;
    
    // Create a VM
    let vm = VM::new()?;
    
    // Execute the bytecode
    let result = vm.execute(&bytecode).await?;
    
    println!("Result: {}", result);
    
    Ok(())
}
```

### Using the Interpreter Provider

```rust
use anyhow::Result;
use rholang_vm::interpreter::{InterpreterProvider, RholangVMInterpreterProvider};

#[tokio::main]
async fn main() -> Result<()> {
    // Create an interpreter provider
    let provider = RholangVMInterpreterProvider::new()?;
    
    // Interpret Rholang code
    let result = provider.interpret("new x in { x!(5) | for(y <- x) { y } }").await;
    
    match result {
        rholang_vm::interpreter::InterpretationResult::Success(output) => {
            println!("Result: {}", output);
        }
        rholang_vm::interpreter::InterpretationResult::Error(err) => {
            println!("Error: {}", err);
        }
    }
    
    Ok(())
}
```

## Examples

The crate includes several examples:

### Simple Arithmetic

Demonstrates basic arithmetic operations:

```bash
cargo run --example simple_arithmetic
```

### Shell Integration

Demonstrates how to use the VM with a simple REPL:

```bash
cargo run --example shell_integration
```

## Supported Rholang Features

The VM currently supports the following Rholang features:

- Literals (Nil, Bool, Int, String)
- Variables
- Arithmetic operations (+, -, *, /, %)
- Comparison operations (==, !=, <, <=, >, >=)
- Conditional expressions (if-then-else)
- Parallel composition (|)
- Name creation (new)
- Channel send (!)
- Channel receive (for)

## RSpace Types

The VM supports different RSpace types for data storage:

- `MemorySequential`: In-memory sequential storage (hashmap)
- `MemoryConcurrent`: In-memory concurrent storage (concurrent hashmap)
- `StoreSequential`: On-store sequential storage (not yet implemented)
- `StoreConcurrent`: On-store concurrent storage (not yet implemented)

## Benchmarks

The VM includes benchmarks to measure performance in different areas:

### Compilation Benchmark

Measures the performance of compiling Rholang code to bytecode:

```bash
cargo bench --bench compilation
```

### Execution Benchmark

Measures the performance of executing bytecode (compilation is done outside the benchmark):

```bash
cargo bench --bench execution
```

### End-to-End Benchmark

Measures the performance of the complete process (compilation + execution):

```bash
cargo bench --bench end_to_end
```

All benchmarks use the test corpus from the rholang-parser crate and automatically skip files that can't be compiled.

## Tests

The VM includes tests to verify functionality with real Rholang code examples:

### Corpus Tests

Tests that compile and execute all examples from the rholang-parser test corpus:

```bash
cargo test --test corpus_tests
```

To see detailed output including compilation and execution results:

```bash
cargo test --test corpus_tests -- --nocapture
```

The corpus tests include three test functions:

1. **Compilation Test**: Attempts to compile each corpus file and reports success/failure counts
2. **Compile and Execute Test**: Attempts to compile and then execute each corpus file, reporting detailed success/failure counts for both steps
3. **End-to-End Test**: Tests the complete process using the `compile_and_execute` method

These tests help ensure that the VM can handle a wide variety of Rholang code examples and provide visibility into which features are working correctly and which need further implementation.

## Future Work

- Implement more Rholang features
- Optimize bytecode generation
- Implement store-based RSpace types
- Add support for concurrency and parallelism
- Implement bundle operations