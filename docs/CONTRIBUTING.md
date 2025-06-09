# Contributing to Rholang-RS

Welcome to the Rholang-RS project! We're excited to have you contribute to building a high-performance Rholang interpreter in Rust. This guide will help you get started with contributing to the project.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Style and Standards](#code-style-and-standards)
- [Testing Requirements](#testing-requirements)
- [Documentation Guidelines](#documentation-guidelines)
- [Pull Request Process](#pull-request-process)
- [Issue Guidelines](#issue-guidelines)
- [Community Guidelines](#community-guidelines)

## Getting Started

### Prerequisites

- **Rust**: Latest stable version (1.80+)
- **Git**: For version control
- **IDE**: VS Code with rust-analyzer recommended

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/YOUR_USERNAME/rholang-rs.git
   cd rholang-rs
   ```

2. **Build and Test**
   ```bash
   # Build the project
   cargo build
   
   # Run tests
   cargo test
   
   # Run the shell
   cargo run -p shell
   ```

3. **Static Analysis**
   ```bash
   # Format code
   cargo fmt
   
   # Check for issues
   cargo clippy --all-features --all-targets -- -D warnings
   
   # Security audit (requires cargo-audit)
   cargo install cargo-audit
   cargo audit
   ```

### Understanding the Codebase

- **Workspace Structure**: Root workspace with `shell/` package
- **Main Binary**: `rhosh` (Rholang shell) in `shell/src/main.rs`
- **Core Logic**: Interpreter traits and implementations
- **Tests**: Unit tests in `src/` modules, integration tests in `tests/`

## Development Workflow

### Branch Strategy

- **Main Branch**: `main` - stable, deployable code
- **Feature Branches**: `feature/description` - new features
- **Bug Fixes**: `fix/description` - bug fixes
- **Documentation**: `docs/description` - documentation updates

### Commit Message Format

Use conventional commits format:

```
type(scope): description

Optional longer description

Fixes #123
```

**Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

**Examples**:
```
feat(parser): add support for pattern matching syntax
fix(interpreter): resolve memory leak in process cleanup
docs(readme): update installation instructions
test(json): add comprehensive serialization tests
```

### Development Process

1. **Check Issues**: Look for existing issues or create one
2. **Create Branch**: `git checkout -b feature/your-feature`
3. **Implement**: Write code following our standards
4. **Test**: Ensure all tests pass
5. **Document**: Update relevant documentation
6. **Submit PR**: Create a pull request for review

## Code Style and Standards

### Rust Conventions

- **Formatting**: Use `cargo fmt` (enforced in CI)
- **Linting**: Pass `cargo clippy` with no warnings
- **Naming**: Follow Rust naming conventions
  - `snake_case` for functions and variables
  - `PascalCase` for types and traits
  - `SCREAMING_SNAKE_CASE` for constants

### Code Organization

```rust
// 1. Standard library imports
use std::collections::HashMap;

// 2. External crate imports
use anyhow::Result;
use serde::{Deserialize, Serialize};

// 3. Internal imports
use crate::interpreter::Interpreter;

// 4. Module declaration
pub mod submodule;
```

### Error Handling

- Use `Result<T, E>` for fallible operations
- Prefer `anyhow::Result` for application errors
- Use `?` operator for error propagation
- Provide meaningful error messages

```rust
// Good
fn parse_value(input: &str) -> Result<RholangValue> {
    if input.is_empty() {
        return Err(anyhow!("Input cannot be empty"));
    }
    // ... parsing logic
}

// Avoid
fn parse_value(input: &str) -> RholangValue {
    input.parse().unwrap() // Don't use unwrap in library code
}
```

### Documentation

- Use rustdoc comments for public APIs
- Include examples in documentation
- Document safety requirements for unsafe code

```rust
/// Converts a Rholang value to JSON format.
///
/// # Examples
///
/// ```rust
/// use rholang_rs::RholangValue;
/// 
/// let value = RholangValue::Int(42);
/// let json = value.to_json()?;
/// assert_eq!(json, r#"{"type":"Int","value":42}"#);
/// ```
///
/// # Errors
///
/// Returns an error if the value contains unsupported types.
pub fn to_json(&self) -> Result<String> {
    // Implementation
}
```

## Testing Requirements

### Test Categories

1. **Unit Tests**: Test individual functions and methods
2. **Integration Tests**: Test component interactions
3. **End-to-End Tests**: Test complete workflows

### Test Organization

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[test]
    fn test_basic_functionality() {
        // Test implementation
    }

    #[rstest]
    #[case(input1, expected1)]
    #[case(input2, expected2)]
    fn test_parameterized(#[case] input: &str, #[case] expected: &str) {
        // Parameterized test
    }
}
```

### Test Requirements

- **Coverage**: Aim for >90% code coverage
- **Edge Cases**: Test boundary conditions and error cases
- **Performance**: Include benchmarks for critical paths
- **Documentation**: Test examples in documentation

### Running Tests

```bash
# Run all tests
cargo test

# Run specific package tests
cargo test -p shell

# Run with coverage (requires tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# Run benchmarks
cargo bench
```

## Documentation Guidelines

### Types of Documentation

1. **Code Comments**: Explain complex logic
2. **API Documentation**: Rustdoc for public APIs
3. **User Guides**: README and tutorial content
4. **Architecture Docs**: High-level design decisions

### Documentation Standards

- **Clarity**: Write for developers unfamiliar with the code
- **Examples**: Include practical examples
- **Maintenance**: Keep documentation in sync with code
- **Accessibility**: Use clear, inclusive language

### Documentation Updates

When making changes, update:
- [ ] Code comments
- [ ] Rustdoc documentation
- [ ] README.md (if user-facing changes)
- [ ] ROADMAP.md (if affecting project direction)
- [ ] Examples and tutorials

## Pull Request Process

### Before Submitting

1. **Rebase**: Rebase your branch on latest `main`
2. **Test**: Run full test suite
3. **Format**: Run `cargo fmt`
4. **Lint**: Fix all clippy warnings
5. **Document**: Update relevant documentation

### PR Requirements

- [ ] Descriptive title and description
- [ ] Links to related issues
- [ ] Test coverage for changes
- [ ] Documentation updates
- [ ] No merge conflicts

### Review Process

1. **Automated Checks**: CI must pass
2. **Code Review**: At least one reviewer approval
3. **Testing**: Manual testing if needed
4. **Merge**: Squash and merge to `main`

### PR Checklist

- [ ] Code follows project conventions
- [ ] Tests pass and coverage is maintained
- [ ] Documentation is updated
- [ ] No breaking changes (or properly documented)
- [ ] Security considerations addressed

## Issue Guidelines

### Creating Issues

- **Search First**: Check for existing similar issues
- **Clear Title**: Descriptive and specific
- **Details**: Provide reproduction steps for bugs
- **Context**: Explain the use case for features

### Issue Labels

- `bug`: Something isn't working
- `enhancement`: New feature or improvement
- `documentation`: Documentation improvements
- `good-first-issue`: Good for newcomers
- `help-wanted`: Extra attention needed
- `priority-high`: High priority items

### Issue Templates

Use the provided templates for:
- Bug reports
- Feature requests
- Documentation improvements

## Community Guidelines

### Code of Conduct

- **Respectful**: Treat all contributors with respect
- **Inclusive**: Welcome diverse perspectives
- **Constructive**: Provide helpful feedback
- **Professional**: Maintain professional communication

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and general discussion
- **Pull Request Reviews**: Code-specific feedback

### Getting Help

- **Documentation**: Check README and rustdoc
- **Issues**: Search existing issues
- **Discussions**: Ask questions in GitHub Discussions
- **Code Review**: Request feedback on draft PRs

## Recognition

Contributors are recognized through:
- **Commit Attribution**: Co-authored-by tags
- **Release Notes**: Major contributions highlighted
- **Documentation**: Contributor acknowledgments

## License

By contributing to Rholang-RS, you agree that your contributions will be licensed under the same license as the project.

---

Thank you for contributing to Rholang-RS! Your efforts help build a better future for decentralized computing. 🚀