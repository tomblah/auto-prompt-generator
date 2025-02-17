#!/bin/bash
# assemble-prompt.sh
#
# This function assembles the final ChatGPT prompt by including:
#   - The contents of files where type definitions were found (processed
#     by the Rust binary that encapsulates file processing, diff reporting,
#     and final prompt assembly)
#   - A fixed instruction.
#
# It takes two parameters:
#   1. <found_files_file>: A file (typically temporary) containing a list of file paths.
#   2. <instruction_content>: The TODO instruction content (now ignored).
#
# The final prompt is copied to the clipboard via pbcopy by the Rust binary.
#
# If DIFF_WITH_BRANCH is set (e.g. --diff-with develop), the Rust binary
# will append a diff report for each file that differs.
#
# Additionally, if the final prompt exceeds a maximum length, exclusion suggestions
# are output using the suggest_exclusions binary.
#
# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

## Use the Rust binary for assembling the prompt.
RUST_ASSEMBLE_PROMPT="$SCRIPT_DIR/rust/target/release/assemble_prompt"
if [ ! -x "$RUST_ASSEMBLE_PROMPT" ]; then
    echo "Error: Rust assemble_prompt binary not found. Please build it with 'cargo build --release'." >&2
    exit 1
fi

assemble-prompt() {
    local found_files_file="$1"
    local instruction_content="$2"  # This parameter is now ignored.

    # Call the Rust binary which performs the full prompt assembly.
    # It will process each file, append diff reports if needed, check prompt size,
    # unescape newlines, copy the final prompt to the clipboard, and print it to stdout.
    "$RUST_ASSEMBLE_PROMPT" "$found_files_file" "$instruction_content"
}

# If executed directly, print usage instructions.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "Usage: source assemble-prompt.sh and call assemble-prompt <found_files_file> <instruction_content>" >&2
    exit 1
fi
