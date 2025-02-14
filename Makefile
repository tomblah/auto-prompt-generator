export PATH := $(HOME)/.cargo/bin:$(PATH)

.PHONY: all build run clean

# Build the Rust binary in release mode
build:
	@echo "Building Rust components..."
	@cd rust/filter_files_singular && cargo build --release

# Run the generate-prompt.sh script after building
run: build
	@echo "Running generate-prompt.sh..."
	@./generate-prompt.sh

# Clean up the Rust build artifacts
clean:
	@echo "Cleaning Rust build artifacts..."
	@cd rust/filter_files_singular && cargo clean

# Default target
all: build
