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

# Run the rholang-shell binary
.PHONY: run
run:
	cargo run -p shell

# Run the rholang-shell binary with file history feature
.PHONY: run-with-history
run-with-history:
	cargo run -p shell --features with-file-history

# Run the examples processor
.PHONY: run-examples
run-examples:
	cargo run --example process_examples

# Run the rholang-tree-sitter-proc-macro examples
.PHONY: macro-examples
macro-examples:
	cargo run --example parse_rholang --features proc_macros
	cargo run --example advanced_usage --features proc_macros
	cargo run --example match_node_example --features proc_macros

# Run all examples in all crates
.PHONY: examples
examples:
	./scripts/run_all_examples.sh

# Run all tests
.PHONY: test
test:
	./scripts/run_all_tests.sh

# Run tests with ignored tests
.PHONY: test-all
test-all:
	cargo test -- --include-ignored

# Run tests for a specific crate
.PHONY: test-shell
test-shell:
	cargo test -p shell

# Run tests for the VM crate
.PHONY: test-vm
test-vm:
	cargo test -p rholang-vm

# Run all VM tests including ignored ones
.PHONY: test-vm-all
test-vm-all:
	cargo test -p rholang-vm -- --include-ignored

# Run a specific VM integration test binary (e.g., BIN=bytecode_examples_tests)
.PHONY: test-vm-bin
test-vm-bin:
	@if [ -z "$(BIN)" ]; then \
		echo "Usage: make test-vm-bin BIN=<integration_test_name> [ARGS='-- --nocapture']"; \
		echo "Example: make test-vm-bin BIN=bytecode_examples_tests ARGS='-- --nocapture'"; \
		exit 1; \
	fi; \
	cargo test -p rholang-vm --test $(BIN) $(ARGS)

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
build-plugin: build-rholang-jni-bridge
	cd rholang-jetbrains-plugin && ./download-gradle-wrapper.sh
	cd rholang-jetbrains-plugin && ./gradlew buildPlugin

# Build the rholang-parser library (required for the JetBrains plugin)
.PHONY: build-rholang-parser
build-rholang-parser:
	cargo build --release -p rholang-parser

# Build the rholang-jni-bridge library with JNI support (required for the JetBrains plugin)
.PHONY: build-rholang-jni-bridge
build-rholang-jni-bridge: build-rholang-parser
	cargo build --release -p rholang-jni-bridge

# Install development dependencies
.PHONY: setup
setup:
	cargo install cargo-tarpaulin
	cargo install cargo-audit

# Container targets
.PHONY: container-build
container-build:
	./scripts/run-in-container.sh make build

.PHONY: container-release
container-release:
	./scripts/run-in-container.sh make release

.PHONY: container-run
container-run:
	./scripts/run-in-container.sh make run

.PHONY: container-test
container-test:
	./scripts/run-in-container.sh make test

.PHONY: container-check
container-check:
	./scripts/run-in-container.sh make check

.PHONY: container-fix
container-fix:
	./scripts/run-in-container.sh make fix

.PHONY: container-shell
container-shell:
	./scripts/run-in-container.sh

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
	@echo "  macro-examples    Run the rholang-tree-sitter-proc-macro examples"
	@echo "  examples         Run all examples in all crates"
	@echo "  test            Run all tests"
	@echo "  test-all        Run all tests including ignored tests"
	@echo "  test-shell      Run tests for the shell crate"
	@echo "  test-vm         Run tests for the rholang-vm crate"
	@echo "  test-vm-all     Run VM tests including ignored tests"
	@echo "  test-vm-bin     Run a specific VM integration test binary (BIN=...)"
	@echo "  check           Check code quality"
	@echo "  fix             Fix code quality issues"
	@echo "  coverage        Run source-only test coverage (excluding tests)"
	@echo "  coverage-html   Generate source-only HTML coverage report (excluding tests)"
	@echo "  clean           Clean the project (including rholang-jetbrains-plugin)"
	@echo "  build-plugin    Build the JetBrains plugin (includes building rholang-jni-bridge)"
	@echo "  build-rholang-parser Build the rholang-parser library (required for the JetBrains plugin)"
	@echo "  build-rholang-jni-bridge Build the rholang-jni-bridge library with JNI support (required for the JetBrains plugin)"
	@echo "  wasm-build      Build the rholang-wasm package (RELEASE=1 TARGET=web OUT_DIR=pkg FEATURES=...)"
	@echo "  wasm-serve      Build and serve rholang-wasm demo (PORT=8000 RELEASE=1 OPEN=1)"
	@echo "  setup           Install development dependencies"
	@echo "  help            Show this help message"
	@echo ""
	@echo "Container Targets:"
	@echo "  container-build  Build the project in a container"
	@echo "  container-release Build with optimizations in a container"
	@echo "  container-run    Run the shell binary in a container"
	@echo "  container-test   Run all tests in a container"
	@echo "  container-check  Check code quality in a container"
	@echo "  container-fix    Fix code quality issues in a container"
	@echo "  container-shell  Start an interactive shell in the container"


# WASM build and serve targets
.PHONY: wasm-build
wasm-build:
	@bash scripts/build_wasm.sh $(if $(RELEASE),--release,) $(if $(TARGET),--target $(TARGET),) $(if $(OUT_DIR),--out-dir $(OUT_DIR),) $(if $(FEATURES),--features $(FEATURES),) $(EXTRA)

.PHONY: wasm-serve
wasm-serve:
	@bash scripts/serve_wasm.sh $(if $(PORT),--port $(PORT),) $(if $(RELEASE),--release,) $(if $(OPEN),--open,)
