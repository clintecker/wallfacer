# Wallfacer - Professional Rust Build System
# ==========================================

CARGO := cargo
RUSTFMT := rustfmt
CLIPPY := cargo clippy

# Build profiles
RELEASE_FLAGS := --release
DEBUG_FLAGS :=

# Cross-compilation targets
PI_TARGET := aarch64-unknown-linux-gnu

.PHONY: all build release debug check test fmt fmt-check lint clippy \
        audit doc clean install-tools pre-commit ci pi help

# Default target
all: fmt-check lint test build

# ============================================================================
# Building
# ============================================================================

## Build debug binary
build: debug

## Build debug binary
debug:
	$(CARGO) build $(DEBUG_FLAGS)

## Build optimized release binary
release:
	$(CARGO) build $(RELEASE_FLAGS)

## Build for Raspberry Pi (requires cross-compilation toolchain)
pi:
	$(CARGO) build $(RELEASE_FLAGS) --target $(PI_TARGET)

## Quick check without building
check:
	$(CARGO) check --all-targets

# ============================================================================
# Testing
# ============================================================================

## Run all tests
test:
	$(CARGO) test --all-targets

## Run tests with output shown
test-verbose:
	$(CARGO) test --all-targets -- --nocapture

## Run tests with coverage (requires cargo-llvm-cov)
coverage:
	cargo llvm-cov --html --open

# ============================================================================
# Code Quality
# ============================================================================

## Format code
fmt:
	$(CARGO) fmt

## Check formatting without modifying files
fmt-check:
	$(CARGO) fmt -- --check

## Run clippy linter with strict settings
lint: clippy

## Run clippy with warnings as errors (lints configured in Cargo.toml)
clippy:
	$(CARGO) clippy --all-targets --all-features -- -D warnings

## Run clippy with auto-fix
clippy-fix:
	$(CARGO) clippy --all-targets --all-features --fix --allow-dirty --allow-staged

## Security audit dependencies (requires cargo-audit)
audit:
	cargo audit

## Check for outdated dependencies (requires cargo-outdated)
outdated:
	cargo outdated

## Check for unused dependencies (requires cargo-udeps, nightly)
udeps:
	cargo +nightly udeps --all-targets

# ============================================================================
# Documentation
# ============================================================================

## Generate documentation
doc:
	$(CARGO) doc --no-deps

## Generate and open documentation
doc-open:
	$(CARGO) doc --no-deps --open

# ============================================================================
# Maintenance
# ============================================================================

## Clean build artifacts
clean:
	$(CARGO) clean

## Update dependencies
update:
	$(CARGO) update

## Install required development tools
install-tools:
	rustup component add rustfmt clippy
	cargo install cargo-audit cargo-outdated cargo-llvm-cov
	@echo "Optional: cargo install cargo-udeps (requires nightly)"

## Install git pre-commit hook
install-hooks:
	cp scripts/pre-commit .git/hooks/pre-commit
	chmod +x .git/hooks/pre-commit
	@echo "Pre-commit hook installed"

# ============================================================================
# CI / Pre-commit
# ============================================================================

## Run all pre-commit checks (fast, for git hook)
pre-commit: fmt-check clippy check test
	@echo "✓ All pre-commit checks passed"

## Run full CI pipeline (thorough, for CI systems)
ci: fmt-check clippy test audit doc
	$(CARGO) build $(RELEASE_FLAGS)
	@echo "✓ Full CI pipeline passed"

## Quick sanity check (fastest, for rapid iteration)
quick: check
	@echo "✓ Quick check passed"

# ============================================================================
# Run
# ============================================================================

## Run in debug mode
run:
	$(CARGO) run

## Run in release mode
run-release:
	$(CARGO) run $(RELEASE_FLAGS)

# ============================================================================
# Help
# ============================================================================

## Show this help
help:
	@echo "Wallfacer Build System"
	@echo "======================"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@grep -E '^## ' $(MAKEFILE_LIST) | sed 's/## /  /'
	@echo ""
	@echo "Common workflows:"
	@echo "  make                 - Format check, lint, test, build"
	@echo "  make quick           - Fast syntax check only"
	@echo "  make pre-commit      - Run all pre-commit validations"
	@echo "  make ci              - Full CI pipeline"
	@echo "  make release         - Build optimized binary"
	@echo "  make pi              - Cross-compile for Raspberry Pi"
