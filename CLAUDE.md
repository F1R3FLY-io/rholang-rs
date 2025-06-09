# Rholang Interpreter in Rust

## Project Context
- This is a F1r3fly.io repository for the Rholang rust interpreter
- If the user does not provide enough information with their prompts, ask the user to clarify before executing the task. This should be included in all tasks including writing unit tests, scaffolding the project, as well as implementation of individual modules. In general, follow a test driven development approach whereby unit tests are developed in parallel with the individual components, and features.

## Commands
- Development: 
- Build: 
- Lint: 
- Start production: 
- DO NOT ever `git add`, `git rm` or `git commit` code. Allow the Claude user to always manually review git changes. `git mv` is permiitted and inform the developer.
- DO NOT ever remove tests from linting, type checks, or limit static analysis checking.
- Run `test && build` commands to test code changes before proceeding to a prompt for more instructions or the next task.

## Code Style
- Follow existing component patterns with clear proper interfaces and traits.
- Follow existing error handling patterns with optional chaining and fallbacks.
- When adding source code or new files, enhance, update, and provide new unit tests using the existing patterns.
- If unused variables are required, deliberately prefix them with an _, underscore and set lint preferences appropriately.

## Best Practices
- Keep console logging to minimum.

## Testing Best Practices
- DO NOT use logging with expects in unit tests to check.
- Always mock external dependencies consistently
- Write tests that focus on behavior over implementation details

## Common Tasks
- If connected to a `mcp-shell-server` also known as just a "shell", run all shell commands through that mcp server. This approach will automatically restrict which commands can be run and properly configure the shell environment. 
- Review `git history` to determine how code base evolved or history for particular files and functions.

## Project Specifics
- DO NOT change language or narratives when synthesizing code. Ask if you think other changes are necessary.
- Observe the lint rules when writing code. 
- Make use of [https://browsertools.agentdesk.ai/](https://browsertools.agentdesk.ai/) if installed in MCP subsystem.
