SHELL = /bin/bash
export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: build test tests clean mc mmc mmmc mmmmc mc-ut-% mc-uts-% mc-it-% mc-its-% mc-it-s-% mc-it-js-% mc-its-js-% all

# Check if Cargo is installed
ifeq ($(shell command -v cargo 2> /dev/null),)
  $(error "Cargo is not installed. Please install the Rust toolchain from https://rustup.rs/")
endif

# Build all Rust binaries in release mode from the workspace root.
build:
	@echo "Building all Rust components..."
	cargo build --release

# Run tests for all packages (searching only inside the 'crates' directory).
# Use either "make test" or "make tests".
test tests:
	@echo "Running Rust tests for all packages..."
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
# Example: make context ARGS="--unit-tests crates/assemble_prompt"
mc meta-context context:
	@echo "Running meta-context script with arguments: $(ARGS)"
	./scripts/meta-context.sh $(ARGS)

# Unit Test Targets:
# Run meta-context for a specific crate's unit tests.
# Usage: make mc-ut-<crate> or make mc-uts-<crate>
mc-ut-% mc-uts-%:
	./scripts/meta-context.sh --unit-tests crates/$*

# Integration Test Target:
# Run meta-context for a specific crate's integration tests.
# Usage: make mc-it-<crate>
mc-it-%:
	./scripts/meta-context.sh --integration-tests crates/$*

# Swift Integration Test Targets:
# Run meta-context for a specific crate's Swift integration tests.
# Usage: make mc-its-<crate> or make mc-it-s-<crate>
mc-its-% mc-it-s-%:
	./scripts/meta-context.sh --integration-tests-swift crates/$*

# Javascript Integration Test Targets:
# Run meta-context for a specific crate's Javascript integration tests.
# Usage: make mc-it-js-<crate> or make mc-its-js-<crate>
mc-it-js-% mc-its-js-%:
	./scripts/meta-context.sh --integration-tests-js crates/$*

# Clipboard Targets:
# Copy the meta-context script to the clipboard.
mmc:
	@echo "Copying scripts/meta-context.sh to clipboard..."
	@if command -v pbcopy >/dev/null; then \
	  cat scripts/meta-context.sh | pbcopy && echo "Copied to clipboard successfully using pbcopy."; \
	elif command -v xclip >/dev/null; then \
	  cat scripts/meta-context.sh | xclip -selection clipboard && echo "Copied to clipboard successfully using xclip."; \
	else \
	  echo "Error: No clipboard tool found (requires pbcopy or xclip)"; exit 1; \
	fi
	
# Copy the Makefile to the clipboard and report success.
mmmc:
	@echo "Copying Makefile to clipboard..."
	@if command -v pbcopy >/dev/null; then \
	  cat Makefile | pbcopy && echo "Makefile copied successfully using pbcopy."; \
	elif command -v xclip >/dev/null; then \
	  cat Makefile | xclip -selection clipboard && echo "Makefile copied successfully using xclip."; \
	else \
	  echo "Error: No clipboard tool found (requires pbcopy or xclip)"; exit 1; \
	fi

# Copy a helpful AI prompt
mmmmc:
	@echo "Copying AI prompt 'how do I use make?' to clipboard..."
	@if command -v pbcopy >/dev/null; then \
	  echo "how do I use make?" | pbcopy && echo "AI prompt copied successfully using pbcopy."; \
	elif command -v xclip >/dev/null; then \
	  echo "how do I use make?" | xclip -selection clipboard && echo "AI prompt copied successfully using xclip."; \
	else \
	  echo "Error: No clipboard tool found (requires pbcopy or xclip)"; exit 1; \
	fi

# Default target: cleans artifacts, builds all Rust components, and runs tests.
all: clean build test
