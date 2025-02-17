#!/usr/bin/env bash
set -e

# List all the bin targets you want to combine into universal binaries.
bins=(
  "assemble_prompt"
  "check_prompt_size"
  "diff_with_branch"
  "extract_enclosing_function"
  "extract_enclosing_type"
  "extract_instruction_content"
  "extract_types"
  "filter_excluded_files"
  "filter_files"
  "filter_files_singular"
  "filter_substring_markers"
  "find_definition_files"
  "find_prompt_instruction"
  "find_referencing_files"
  "generate_prompt"
  "get_git_root"
  "get_package_root"
  "get_search_roots"
  "log_file_sizes"
  "prompt_file_processor"
  "suggest_exclusions"
  "unescape_newlines"
)

echo "Building for x86_64-apple-darwin..."
cargo build --release --target x86_64-apple-darwin

echo "Building for aarch64-apple-darwin..."
cargo build --release --target aarch64-apple-darwin

# Create a universal output directory if it doesn't already exist
mkdir -p target/universal/release

echo "Combining binaries with lipo..."
for bin in "${bins[@]}"; do
    echo "  -> $bin"
    lipo -create \
      "target/x86_64-apple-darwin/release/$bin" \
      "target/aarch64-apple-darwin/release/$bin" \
      -output "target/universal/release/$bin"
done

echo "All universal binaries placed in target/universal/release/"
