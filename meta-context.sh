#!/bin/bash
set -euo pipefail

##########################################
# meta-context.sh
#
# This script collects the Rust source files (and Cargo.toml files)
# in the repository and copies them to the clipboard.
##########################################

# Define a function to filter out inline Rust test blocks.
filter_rust_tests() {
    awk '
    BEGIN { in_tests=0; brace_count=0 }
    {
        # If we see a #[cfg(test)] attribute and we are not already in a test block, start skipping.
        if (in_tests == 0 && $0 ~ /^[[:space:]]*#\[cfg\(test\)\]/) {
            in_tests = 1;
            next;
        }
        # If we are in a test block and see a module declaration, start counting braces.
        if (in_tests == 1 && $0 ~ /^[[:space:]]*mod[[:space:]]+tests[[:space:]]*\{/) {
            brace_count = 1;
            next;
        }
        # If we are inside a test module block, count braces to know when it ends.
        if (in_tests == 1 && brace_count > 0) {
            n = gsub(/\{/, "{");
            m = gsub(/\}/, "}");
            brace_count += n - m;
            if (brace_count <= 0) {
                in_tests = 0;
                brace_count = 0;
            }
            next;
        }
        print;
    }
    ' "$1"
}

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Optionally, determine the repository root (assumes you're in a Git repository).
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR")
cd "$REPO_ROOT"

echo "Including only Rust source files in the context."
files=$(find rust -type f -iname "*.rs")

# Always include Cargo.toml files across the repository.
cargo_files=$(find . -type f -name "Cargo.toml" -not -path "./.git/*")
if [ -n "$cargo_files" ]; then
    echo "Including all Cargo.toml files in the context."
    files="$files $cargo_files"
fi

# Display the collected files.
echo "--------------------------------------------------"
echo "Files to include in the meta-context prompt:"
for file in $files; do
    echo " - $file"
done
echo "--------------------------------------------------"

# Create a temporary file to accumulate the context.
temp_context=$(mktemp)

# Loop over each file and append a header and its content.
for file in $files; do
    {
      echo "--------------------------------------------------"
      echo "The contents of $file is as follows:"
      echo "--------------------------------------------------"
      if [[ "$file" == *.rs ]]; then
          filter_rust_tests "$file"
          echo -e "\n// Note: rust file unit tests not shown here for brevity."
      else
          cat "$file"
      fi
      echo -e "\n"
    } >> "$temp_context"
done

# Append a horizontal dashed line and a new line.
{
  echo "--------------------------------------------------"
  echo ""
} >> "$temp_context"

# Copy the final context to the clipboard using pbcopy.
pbcopy < "$temp_context"

echo "--------------------------------------------------"
echo "Success: Meta context has been copied to the clipboard."
echo "--------------------------------------------------"

# Clean up the temporary file.
rm "$temp_context"
