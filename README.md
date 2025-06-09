# rholang-rs

A Rholang interpreter implementation in Rust for the F1r3fly decentralized compute infrastructure.

## Overview

This project implements a Rust-based interpreter for the Rholang programming language, a concurrent process calculus designed for blockchain and distributed computing. Rholang is built on the reflective higher-order calculus (rho-calculus) and is fully asynchronous, making it ideal for decentralized applications and smart contracts.

### Key Features of Rholang

- **Concurrent by Design**: Concurrency is built directly into the language syntax
- **Asynchronous Execution**: Fully asynchronous runtime with message-passing semantics
- **Rho-Calculus Foundation**: Based on reflective higher-order process calculus
- **Blockchain Optimized**: Designed to prevent common smart contract vulnerabilities
- **Scalable Architecture**: Uses Directed Acyclic Graphs (DAGs) for improved scalability

## Project Goals

This Rust implementation aims to provide:
- High-performance Rholang interpreter
- Integration with F1r3fly's decentralized compute infrastructure
- Trustworthy, scalable, and concurrent execution environment
- REPL for interactive development and testing

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo package manager

### Building the Project

```bash
# Clone the repository
git clone <repository-url>
cd rholang-rs

# Build the project
cargo build

# Build for release (optimized)
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with verbose output
cargo test -- --nocapture

# Run specific test module
cargo test <module_name>

# Run tests and show test coverage
cargo test --all-features
```

### Static Analysis and Code Quality

```bash
# Check code formatting
cargo fmt --check

# Format code automatically
cargo fmt

# Run Clippy linter for code quality
cargo clippy

# Run Clippy with all features and strict mode
cargo clippy --all-features --all-targets -- -D warnings

# Check for security vulnerabilities
cargo audit
```

### Running the Interpreter

```bash
# Run the interpreter
cargo run

# Run with specific Rholang file
cargo run -- <file.rho>

# Run in REPL mode
cargo run -- --repl
```

## Development

### Project Structure

```
rholang-rs/
├── src/
│   ├── parser/          # Rholang syntax parser
│   ├── interpreter/     # Core interpreter logic
│   ├── runtime/         # Runtime environment
│   └── repl/           # Interactive REPL
├── tests/              # Integration tests
└── examples/           # Example Rholang programs
```

### Code Style Guidelines

- Follow Rust standard formatting (`cargo fmt`)
- Use meaningful variable and function names
- Write comprehensive unit tests for all modules
- Document public APIs with rustdoc comments
- Handle errors explicitly using `Result<T, E>`

### Testing Strategy

- **Unit Tests**: Test individual components in isolation
- **Integration Tests**: Test complete interpreter workflows
- **Property-Based Tests**: Use fuzzing for parser robustness
- **Benchmark Tests**: Performance testing for critical paths

```bash
# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test '*'

# Run benchmarks
cargo bench

# Generate test coverage report
cargo tarpaulin --out Html
```

### Continuous Integration

The project uses automated testing and static analysis:

```bash
# Full CI pipeline locally
cargo fmt --check && \
cargo clippy --all-features --all-targets -- -D warnings && \
cargo test --all-features && \
cargo build --release
```

## Rholang Language Example

```rholang
new helloworld, stdout(`rho:io:stdout`) in {
  contract helloworld( world ) = {
    for( @msg <- world ) {
      stdout!(msg)
    }
  } |
  helloworld!("Hello, World!")
}
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Run the full test suite and static analysis
4. Submit a pull request with comprehensive tests

## License

This project is part of the F1r3fly open-source ecosystem.

## Resources

- [Rholang Official Documentation](https://rholang.org/)
- [F1r3fly Project](https://github.com/F1R3FLY-io/f1r3fly)
- [RChain Cooperative](https://rchain.coop/)

## Disclaimer

This project is in active development and should not be used for applications involving material value. It is part of the experimental F1r3fly decentralized compute infrastructure.
