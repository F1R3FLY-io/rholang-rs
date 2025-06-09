# Rholang Interpreter in Rust

## Project Context
- This is a F1r3fly.io repository for the Rholang rust interpreter
- Uses workspace structure with `shell/` package containing the main interpreter (`rhosh` binary)
- Currently uses Rust Edition 2021 with resolver v2 for stable compatibility
- If the user does not provide enough information with their prompts, ask the user to clarify before executing the task. This should be included in all tasks including writing unit tests, scaffolding the project, as well as implementation of individual modules. In general, follow a test driven development approach whereby unit tests are developed in parallel with the individual components, and features.

## Commands
- Development: `cargo run -p shell` (runs rhosh interpreter)
- Build: `cargo build` (workspace), `cargo build -p shell` (specific package)
- Test: `cargo test` (all), `cargo test -p shell` (package-specific)
- Lint: `cargo clippy --all-features --all-targets -- -D warnings`
- Format: `cargo fmt`
- Fix: `cargo fix --bin "rhosh"`
- Static Analysis: `cargo audit` (requires `cargo install cargo-audit`)
- Start production: `./target/release/rhosh` (after `cargo build --release`)
- DO NOT ever `git add`, `git rm` or `git commit` code. Allow the Claude user to always manually review git changes. `git mv` is permitted and inform the developer.
- DO NOT ever remove tests from linting, type checks, or limit static analysis checking.
- Run `cargo test && cargo build` commands to test code changes before proceeding to a prompt for more instructions or the next task.

## Code Style
- Follow Rust standard formatting: run `cargo fmt` before commits
- Use `cargo clippy` to catch common issues and improve code quality
- Follow existing component patterns with clear proper interfaces and traits
- Follow existing error handling patterns with `Result<T, E>` and `anyhow` for error handling
- When adding source code or new files, enhance, update, and provide new unit tests using the existing patterns
- If unused variables are required, deliberately prefix them with `_` underscore
- Use `#[allow(dead_code)]` sparingly and only for legitimate cases like alternative implementations
- Prefer explicit error handling over unwrap/expect except in tests
- Use workspace dependencies for consistent versioning across packages

## Best Practices
- Keep console logging to minimum
- Run static analysis pipeline before completing tasks: `cargo fmt --check && cargo clippy --all-features --all-targets -- -D warnings && cargo test && cargo build`
- Fix all clippy warnings and compiler warnings before considering a task complete
- Use features in Cargo.toml to enable optional functionality (e.g., `with-file-history`)
- Maintain clean build with zero warnings

## Static Analysis and Quality Assurance
- **Formatting**: Always run `cargo fmt` to maintain consistent code style
- **Linting**: Use `cargo clippy` with strict settings to catch issues early
- **Type Checking**: Ensure all code compiles without warnings
- **Security**: Run `cargo audit` to check for known vulnerabilities in dependencies
- **Testing**: Maintain comprehensive test coverage with `cargo test`
- **Documentation**: Use rustdoc comments for public APIs

## Testing Best Practices
- DO NOT use logging with expects in unit tests to check
- Always mock external dependencies consistently  
- Write tests that focus on behavior over implementation details
- Use `rstest` for parameterized testing (already in workspace dependencies)
- Test both success and error cases
- Use `#[cfg(test)]` modules for test-only code

## Workflow for Code Changes
1. **Planning**: Use TodoWrite tool for complex multi-step tasks
2. **Implementation**: Write code following established patterns
3. **Testing**: Add/update unit tests alongside implementation
4. **Static Analysis**: Run the complete pipeline:
   ```bash
   cargo fmt --check && \
   cargo clippy --all-features --all-targets -- -D warnings && \
   cargo test && \
   cargo build
   ```
5. **Documentation**: Update relevant documentation and comments
6. **Review**: Present changes for user review before git operations

## Common Tasks
- If connected to a `mcp-shell-server` also known as just a "shell", run all shell commands through that mcp server. This approach will automatically restrict which commands can be run and properly configure the shell environment
- Review `git history` to determine how code base evolved or history for particular files and functions
- When adding new dependencies, add them to workspace dependencies in root Cargo.toml
- Use package-specific commands when working on shell: `cargo test -p shell`, `cargo run -p shell`

## Project Specifics
- DO NOT change language or narratives when synthesizing code. Ask if you think other changes are necessary
- Observe the lint rules when writing code and fix all warnings
- Make use of [https://browsertools.agentdesk.ai/](https://browsertools.agentdesk.ai/) if installed in MCP subsystem
- Current workspace structure: root with `shell/` package containing the Rholang interpreter
- Main binary: `rhosh` (Rholang shell) in `shell/src/main.rs`
