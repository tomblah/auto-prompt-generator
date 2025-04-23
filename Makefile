SHELL = /bin/bash
export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: build test tests coverage clean fix-headers mc mmc mmmc mmmmc all check review

# Ensure Cargo is installed
ifeq ($(shell command -v cargo 2> /dev/null),)
  $(error "Cargo is not installed. Please install the Rust toolchain from https://rustup.rs/")
endif

# Build all Rust binaries in release mode.
build:
	@echo "Building all Rust components..."
	cargo build --release

# Run tests for all packages (inside the 'crates' directory).
test tests:
	@echo "Running Rust tests for all packages..."
	@while IFS= read -r -d '' manifest; do \
	    package_dir=$$(dirname "$$manifest"); \
	    echo "Running tests in package: $$package_dir"; \
	    cargo test --manifest-path "$$manifest" -- --test-threads=1 || exit 1; \
	done < <(find crates -name Cargo.toml -print0)

# Generate code coverage reports.
coverage:
	@echo "Generating code coverage reports with cargo tarpaulin..."
	cargo tarpaulin --workspace --out Html

coverage-compare:
	./scripts/coverage-compare.sh
	
# Fix file headers.
fix-headers:
	@echo "Fixing headers..."
	./scripts/fix-headers.sh

# Clean Rust build artifacts.
clean:
	@echo "Cleaning Rust build artifacts..."
	cargo clean

# Run the meta-context script with optional arguments.
# Example: make context ARGS="--unit-tests crates/assemble_prompt"
mc meta-context context:
	@echo "Running meta-context script with arguments: $(ARGS)"
	./scripts/meta-context.sh $(ARGS)

# Copy the meta-context script to the clipboard.
mmc:
	@echo "Copying scripts/meta-context.sh to clipboard..."
	@if command -v pbcopy >/dev/null; then \
	  cat scripts/meta-context.sh | pbcopy && echo "Copied using pbcopy."; \
	elif command -v xclip >/dev/null; then \
	  cat scripts/meta-context.sh | xclip -selection clipboard && echo "Copied using xclip."; \
	else \
	  echo "Error: No clipboard tool found (requires pbcopy or xclip)"; exit 1; \
	fi

# Copy the Makefile to the clipboard.
mmmc:
	@echo "Copying Makefile to clipboard..."
	@if command -v pbcopy >/dev/null; then \
	  cat Makefile | pbcopy && echo "Makefile copied using pbcopy."; \
	elif command -v xclip >/dev/null; then \
	  cat Makefile | xclip -selection clipboard && echo "Makefile copied using xclip."; \
	else \
	  echo "Error: No clipboard tool found (requires pbcopy or xclip)"; exit 1; \
	fi

# Copy an AI prompt to the clipboard.
mmmmc:
	@echo "Copying AI prompt 'how do I use make?' to clipboard..."
	@if command -v pbcopy >/dev/null; then \
	  echo "how do I use make?" | pbcopy && echo "AI prompt copied using pbcopy."; \
	elif command -v xclip >/dev/null; then \
	  echo "how do I use make?" | xclip -selection clipboard && echo "AI prompt copied using xclip."; \
	else \
	  echo "Error: No clipboard tool found (requires pbcopy or xclip)"; exit 1; \
	fi

# Default target: clean, fix headers, build, test, and generate coverage.
all: clean fix-headers build test coverage

# Run the diff‑and‑copy check script
check:
	@echo "Running diff‑and‑copy check…"
	./scripts/is-this-right.sh

# Copy diff-vs-main bundle to clipboard for code review
review:
	./scripts/code-review.sh
