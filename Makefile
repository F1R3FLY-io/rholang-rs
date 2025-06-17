# Makefile for Rholang Project

# Default target
.PHONY: all
all: build

# Build the project
.PHONY: build
build:
	cargo build

# Build with optimizations
.PHONY: release
release:
	cargo build --release

# Run the shell binary
.PHONY: run
run:
	cargo run -p shell

# Run the shell binary with file history feature
.PHONY: run-with-history
run-with-history:
	cargo run -p shell --features with-file-history

# Run the examples processor
.PHONY: run-examples
run-examples:
	cargo run --example process_examples

# Run all tests
.PHONY: test
test:
	cargo test

# Run tests with ignored tests
.PHONY: test-all
test-all:
	cargo test -- --include-ignored

# Run tests for a specific crate
.PHONY: test-shell
test-shell:
	cargo test -p shell

# Check code quality
.PHONY: check
check:
	./scripts/check_code_quality.sh

# Fix code quality issues
.PHONY: fix
fix:
	./scripts/fix_code_quality.sh

# Run test coverage
.PHONY: coverage
coverage:
	@if command -v cargo-tarpaulin > /dev/null; then \
		./scripts/check_src_coverage.sh Stdout; \
	else \
		echo "ℹ️ cargo-tarpaulin not found, skipping test coverage check"; \
		echo "   Install with: cargo install cargo-tarpaulin"; \
		echo "   Or run: make setup"; \
	fi

# Generate HTML coverage report
.PHONY: coverage-html
coverage-html:
	@if command -v cargo-tarpaulin > /dev/null; then \
		./scripts/check_src_coverage.sh Html coverage; \
	else \
		echo "ℹ️ cargo-tarpaulin not found, skipping HTML coverage report generation"; \
		echo "   Install with: cargo install cargo-tarpaulin"; \
		echo "   Or run: make setup"; \
	fi

# Clean the project
.PHONY: clean
clean:
	cargo clean
	cd rholang-jetbrains-plugin && ./gradlew clean
	rm -rf rholang-jetbrains-plugin/.gradle

# Build the JetBrains plugin
.PHONY: build-plugin
build-plugin: build-rholang-parser
	cd rholang-jetbrains-plugin && ./download-gradle-wrapper.sh
	cd rholang-jetbrains-plugin && ./gradlew buildPlugin

# Build the rholang-parser library with j4rs support (required for the JetBrains plugin)
.PHONY: build-rholang-parser
build-rholang-parser:
	cargo build --release -p rholang-parser

# Install development dependencies
.PHONY: setup
setup:
	cargo install cargo-tarpaulin
	cargo install cargo-audit

# Help target
.PHONY: help
help:
	@echo "Rholang Project Makefile"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  all             Build the project (default)"
	@echo "  build           Build the project"
	@echo "  release         Build with optimizations"
	@echo "  run             Run the shell binary"
	@echo "  run-with-history Run the shell binary with file history feature"
	@echo "  run-examples     Run the examples processor"
	@echo "  test            Run all tests"
	@echo "  test-all        Run all tests including ignored tests"
	@echo "  test-shell      Run tests for the shell crate"
	@echo "  check           Check code quality"
	@echo "  fix             Fix code quality issues"
	@echo "  coverage        Run source-only test coverage (excluding tests)"
	@echo "  coverage-html   Generate source-only HTML coverage report (excluding tests)"
	@echo "  clean           Clean the project (including rholang-jetbrains-plugin)"
	@echo "  build-plugin    Build the JetBrains plugin (includes building rholang-parser)"
	@echo "  build-rholang-parser Build the rholang-parser library with j4rs support (required for the JetBrains plugin)"
	@echo "  setup           Install development dependencies"
	@echo "  help            Show this help message"
