#!/bin/bash
#
# getting-started.sh
#
# This script generates an onboarding prompt for the Auto Prompt Generator project
# and copies it to your clipboard. The prompt instructs the AI to provide exactly
# 5 beginner-friendly steps for getting set up, with each step including a short
# explanation and any command examples enclosed in triple backticks for clarity.
#
# The 5 steps should cover:
#   1. **Install Rust Toolchain:** How to install Rust and Cargo from rustup.rs.
#   2. **Clone & Setup Repository:** How to clone the repository and perform any initial setup.
#   3. **Build the Project:** How to navigate to the project directory and run `make build`.
#   4. **Add to PATH:** How to locate the generated `generate_prompt` binary (typically in `target/release`)
#      and add its directory to your PATH.
#   5. **Play Around:** How to insert a `// TODO: -` comment into a Swift file, run the prompt generator,
#      and paste the output into your favorite AI tool. (This will show how the auto prompt generator
#      converts your TODO into a detailed prompt and how the AI turns that prompt into a helpful answer.)
#
# If you get stuck at any step, feel free to ask followup questions. And if after a few questions
# you're still stuck, please open an issue at:
# https://github.com/tomblah/auto-prompt-generator/issues
#
# The Makefile content is included for context (it is not meant to be copied):
#
# --------------------------------------------------
# SHELL = /bin/bash
# export PATH := $(HOME)/.cargo/bin:$(PATH)
#
# .PHONY: build test tests coverage clean fix-headers mc mmc mmmc mmmmc all
#
# # Ensure Cargo is installed
# ifeq ($(shell command -v cargo 2> /dev/null),)
#   $(error "Cargo is not installed. Please install the Rust toolchain from https://rustup.rs/")
# endif
#
# # Build all Rust binaries in release mode.
# build:
#     @echo "Building all Rust components..."
#     cargo build --release
#
# # Run tests for all packages (inside the 'crates' directory).
# test tests:
#     @echo "Running Rust tests for all packages..."
#     @while IFS= read -r -d '' manifest; do \
#         package_dir=$$(dirname "$$manifest"); \
#         echo "Running tests in package: $$package_dir"; \
#         cargo test --manifest-path "$$manifest" -- --test-threads=1 || exit 1; \
#     done < <(find crates -name Cargo.toml -print0)
#
# # Generate code coverage reports.
# coverage:
#     @echo "Generating code coverage reports with cargo tarpaulin..."
#     cargo tarpaulin --workspace --out Html
#
# # Fix file headers.
# fix-headers:
#     @echo "Fixing headers..."
#     ./scripts/fix-headers.sh
#
# # Clean Rust build artifacts.
# clean:
#     @echo "Cleaning Rust build artifacts..."
#     cargo clean
#
# # Default target: clean, fix headers, build, test, and generate coverage.
# all: clean fix-headers build test coverage
# --------------------------------------------------
#
# Please provide a clear, beginner-friendly, step-by-step guide with exactly 5 steps as described above.
#
# Each step should include a short explanation and any command examples in triple backticks.
# Conclude by inviting followup questions if any details are unclear.

prompt=$(cat << 'EOF'
Auto Prompt Generator is a modular Rust project that transforms your TODO comments into AI-friendly prompts with rich contextual code snippets. The project is composed of multiple crates that work together to extract a TODO instruction from your Swift (and partially supported JavaScript/Objective-C) projects, gather relevant code context (like type definitions and Git diff reports), and assemble a prompt for AI analysis.

For context, here is the project's Makefile (for reference only):

--------------------------------------------------
SHELL = /bin/bash
export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: build test tests coverage clean fix-headers mc mmc mmmc mmmmc all

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

# Fix file headers.
fix-headers:
    @echo "Fixing headers..."
    ./scripts/fix-headers.sh

# Clean Rust build artifacts.
clean:
    @echo "Cleaning Rust build artifacts..."
    cargo clean

# Default target: clean, fix headers, build, test, and generate coverage.
all: clean fix-headers build test coverage
--------------------------------------------------

Please provide a clear, beginner-friendly, step-by-step guide with exactly 5 steps. Each step should include a short explanation and any command examples enclosed in triple backticks, as follows:

1. **Install Rust Toolchain:** Explain how to install Rust and Cargo from [rustup.rs](https://rustup.rs) if not already installed, including a command example.
2. **Clone & Setup Repository:** Instruct how to clone the repository (e.g., using `git clone <repo_url>`) and perform any initial setup.
3. **Build the Project:** Explain how to navigate to the project directory and run `make build` to compile the project in release mode.
4. **Add to PATH:** Guide the user to locate the `generate_prompt` binary (typically in `target/release`) and add its directory to their PATH.
5. **Play Around:** Instruct the user to insert a `// TODO: -` comment in a Swift file, run the prompt generator, and paste the output into their favorite AI tool. Mention that this will demonstrate how the auto prompt generator converts a TODO into a detailed prompt and how the AI turns that prompt into a helpful answer.

If you get stuck at any step, please ask followup questions. And if after a few questions you're still stuck, feel free to open an issue at:
https://github.com/tomblah/auto-prompt-generator/issues
EOF
)

echo "$prompt" | pbcopy

echo "The onboarding prompt (with 5 detailed, friendly steps and a help invitation) has been copied to your clipboard."
echo "Paste it into your favorite AI tool to receive a clear, 5-step guide with formatted code examples and next-step recommendations."
