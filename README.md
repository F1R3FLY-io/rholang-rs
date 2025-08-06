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

### Rust Setup

This project currently uses Rust Edition 2021 for maximum compatibility. To prepare for future 2024 edition migration:

```bash
# Update to the latest stable Rust version
rustup update stable

# Set stable as default (if not already)
rustup default stable

# Verify your Rust version
rustc --version
cargo --version

# For future 2024 edition support, you may need nightly:
# rustup install nightly
# rustup default nightly
```

**Note**: Edition 2024 requires Cargo 1.85+ with the feature stabilized. Currently using Edition 2021 and resolver v2 for stable compatibility.

### Building the Project

```bash
# Clone the repository
git clone <repository-url>
cd rholang-rs

# Build the entire workspace
cargo build

# Build for release (optimized)
cargo build --release

# Build specific workspace member
cargo build -p shell
```

### Running Tests

```bash
# Run all tests in workspace
cargo test

# Run tests with verbose output
cargo test -- --nocapture

# Run tests for specific workspace member
cargo test -p shell

# Run specific test module
cargo test <module_name>

# Run tests and show test coverage
cargo test --all-features
```

### Development Container

For a consistent development environment, this project provides a Docker-based development container with all necessary tools pre-installed.

```bash
# Start an interactive shell in the container
make container-shell

# Build the project in the container
make container-build

# Run tests in the container
make container-test

# Check code quality in the container
make container-check

# Fix code quality issues in the container
make container-fix
```

For detailed instructions on using the development container, see [DOCKER.md](DOCKER.md).

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

# Fix code style issues automatically
cargo fix --bin "rhosh"

# Check for security vulnerabilities (requires cargo-audit)
cargo install cargo-audit
cargo audit
```

### Running the Interpreter

```bash
# Run the Rholang shell (rhosh)
cargo run -p shell

# Run with specific arguments
cargo run -p shell -- --help

# Run the interpreter binary directly after building
./target/debug/rhosh

# Run the release version
./target/release/rhosh
```

## Development

### Project Structure

```
rholang-rs/
├── Cargo.toml           # Workspace configuration
├── shell/               # Rholang interpreter shell (rhosh)
│   ├── Cargo.toml       # Shell package configuration
│   ├── src/
│   │   ├── main.rs      # Shell entry point
│   │   ├── lib.rs       # Library modules
│   │   ├── interpreter.rs    # Core interpreter logic
│   │   ├── rh_interpreter.rs # Rholang-specific interpreter
│   │   └── main_sync.rs # Synchronous main alternative
│   └── tests/           # Shell integration tests
├── README.md
└── CLAUDE.md           # Project instructions for Claude
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

# Run tests for shell package
cargo test -p shell

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

## 📄 License

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)


## Resources

- [Rholang Official Documentation](https://rholang.org/)
- [F1r3fly Project](https://github.com/F1R3FLY-io/f1r3fly)
- [RChain Cooperative](https://rchain.coop/)

## Disclaimer

This project is in active development and should not be used for applications involving material value. It is part of the experimental F1r3fly decentralized compute infrastructure.
