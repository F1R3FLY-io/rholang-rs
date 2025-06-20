# Rholang-RS Changelog

All notable changes to the Rholang-RS project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Planned features for future releases as outlined in the ROADMAP.md

## [0.1.1] - 2025-06-19
### Added
- Core Rholang data types with JSON serialization
- JNI bridge for integration with JetBrains plugin
- Documentation updates (ROADMAP.md and BYTECODE.md)

### Changed
- Refactored plugin to use JNI-based Rholang parser, removing CLI dependency
- Consolidated dependency versions in workspace configuration

### Fixed
- Improved error handling and fallback mechanisms in RholangParserJNI
- Fixed markdown formatting issues in ROADMAP.md

### Removed
- Unused bridges

## [0.1.0] - 2025-06-11
### Added
- Project infrastructure setup with workspace configuration
- Shell package (`rhosh`) with rustyline integration
- Basic interpreter trait and implementations:
  - FakeInterpreterProvider (initial implementation)
  - RholangParserInterpreterProvider (tree-sitter based implementation)
- Comprehensive test framework and CI pipeline
- Static analysis pipeline (clippy, fmt, audit)
- Documentation structure and developer guidelines
- Multiline input mode with toggle command
- Command-line arguments for configuration
- Special commands:
  - `.help`: Show help message
  - `.mode`: Toggle between multiline and single line modes
  - `.list`: List all edited lines
  - `.delete` or `.del`: Remove the last edited line
  - `.reset` or Ctrl+C: Interrupt current input
  - `.quit`: Exit the shell
- Process management with kill and ps commands
- Error handling with InterpretationResult and InterpreterError structures
- Bracket-aware processing for improved multiline command handling
- Code quality scripts and Makefile tasks

### Added (Parser Components)
- Rholang Tree-Sitter grammar implementation
- Lexer implementation with token definitions for Rholang syntax
- Grammar definition for Rholang language
- AST node structures
- Expression parsing (arithmetic, logical)
- Pattern matching syntax
- Rholang parser based on tree-sitter grammar

## [0.0.1] - 2025-05-27
### Added
- Initial project setup
- Basic repository structure
- .gitignore for Rust project

## Future Plans

### Phase 1: Core Language Implementation (Q1-Q2 2025)
- Parser and AST (v0.2.0)
- Interpreter Core (v0.3.0)
- Runtime Environment (v0.4.0)
- Bytecode Implementation (v0.4.5)

### Phase 2: Advanced Features (Q2-Q3 2025)
- Security and Safety (v0.5.0)
- Standard Library (v0.6.0)
- Blockchain Integration (v0.7.0)

### Phase 3: Production Readiness (Q3-Q4 2025)
- Performance Optimization (v0.8.0)
- Production Features (v0.9.0)
- Release Preparation (v1.0.0)

### Phase 4: Ecosystem Growth (2026+)
- Advanced Features (v1.1.0+)
- Developer Experience improvements
- Extended Capabilities

For detailed information about future plans, please refer to the [ROADMAP.md](docs/ROADMAP.md) file.