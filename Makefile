export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: all build run clean

# Build all Rust binaries in release mode from the workspace root.
build:
	@echo "Building all Rust components..."
	@cd rust && cargo build --release

# Run the generate-prompt.sh script after building.
run: build
	@echo "Running generate-prompt.sh..."
	@./generate-prompt.sh

# Clean up the Rust build artifacts.
clean:
	@echo "Cleaning Rust build artifacts..."
	@cd rust && cargo clean

# Default target.
all: build
