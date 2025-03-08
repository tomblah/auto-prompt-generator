SHELL = /bin/bash
export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: all build run clean test

# Build all Rust binaries in release mode from the workspace root.
build:
	@echo "Building all Rust components..."
	cargo build --release

# Run tests for all packages (searching only inside the 'crates' directory).
test:
	echo "Running Rust tests for all packages..."
	@while IFS= read -r -d '' manifest; do \
	    package_dir=$$(dirname "$$manifest"); \
	    echo "Running tests in package: $$package_dir"; \
	    cargo test --manifest-path "$$manifest" -- --test-threads=1; \
	done < <(find crates -name Cargo.toml -print0)

# Clean up the Rust build artifacts.
clean:
	@echo "Cleaning Rust build artifacts..."
	cargo clean
	
# Run the meta-context script with optional arguments.
# Example usage:
#   make context ARGS="--unit-tests crates/assemble_prompt"
context:
	@echo "Running meta-context script with arguments: $(ARGS)"
	./scripts/meta-context.sh $(ARGS)

# Default target.
all: build
