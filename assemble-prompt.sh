#!/bin/bash
# assemble-prompt.sh
#
# This function assembles the final ChatGPT prompt by including:
#   - The contents of Swift (or other allowed) files where type definitions were found
#     (optionally filtered by substring markers), and
#   - A fixed instruction (ignoring the extracted TODO instruction).
#
# It takes two parameters:
#   1. <found_files_file>: A file (typically temporary) containing a list of file paths.
#   2. <instruction_content>: The TODO instruction content (now ignored).
#
# The function outputs the final assembled prompt to stdout and also copies it
# to the clipboard using pbcopy.
#
# If the environment variable DIFF_WITH_BRANCH is set (for example by running:
#   generate-prompt.sh --diff-with develop
# then for each file that differs from that branch, a diff report is appended after the file's content.

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the helper that filters file content based on substring markers.
source "$SCRIPT_DIR/filter-substring-markers.sh"

## Use the Rust binary for checking prompt size.
# Set the path to the check_prompt_size binary (adjust the relative path if needed)
RUST_CHECK_SIZE="$SCRIPT_DIR/rust/target/release/check_prompt_size"
if [ ! -x "$RUST_CHECK_SIZE" ]; then
    echo "Error: Rust check_prompt_size binary not found. Please build it with 'cargo build --release'." >&2
    exit 1
fi

# If DIFF_WITH_BRANCH is set, source the diff helper.
if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
    source "$SCRIPT_DIR/diff-with-branch.sh"
fi

assemble-prompt() {
    local found_files_file="$1"
    local instruction_content="$2"  # This parameter is now ignored.
    
    # Sort and filter out duplicate file paths.
    local unique_found_files
    unique_found_files=$(sort "$found_files_file" | uniq)
    
    local clipboard_content=""
    
    # Process each file and format its content.
    while IFS= read -r file_path; do
        local file_basename file_content diff_output
        file_basename=$(basename "$file_path")
        
        if grep -qE '^[[:space:]]*//[[:space:]]*v' "$file_path"; then
            file_content=$(filter_substring_markers "$file_path")
        else
            file_content=$(cat "$file_path")
        fi
        
        clipboard_content="${clipboard_content}
The contents of ${file_basename} is as follows:

${file_content}

"
        # If DIFF_WITH_BRANCH is set, append a diff report (if there are changes).
        if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
            diff_output=$(get_diff_with_branch "$file_path")
            if [ -n "$diff_output" ]; then
                clipboard_content="${clipboard_content}
--------------------------------------------------
The diff for ${file_basename} (against branch ${DIFF_WITH_BRANCH}) is as follows:

${diff_output}

"
            fi
        fi
        
        clipboard_content="${clipboard_content}
--------------------------------------------------
"
    done <<< "$unique_found_files"
    
    # Fixed instruction that will be appended.
    local fixed_instruction="Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen"
    
    local final_clipboard_content="${clipboard_content}

${fixed_instruction}"
    
    # Check prompt size using the Rust binary.
    # Use printf with a subshell that ignores SIGPIPE to prevent broken-pipe errors.
    warning_output=$( (trap '' SIGPIPE; printf "%s" "$final_clipboard_content") | "$RUST_CHECK_SIZE" 2>&1 || true )
    if [ -n "$warning_output" ]; then
        echo "$warning_output"
    fi

    # Additional debug logging for prompt size check.
    local prompt_length
    prompt_length=$(printf "%s" "$final_clipboard_content" | wc -m | tr -d ' ')
    local max_length=100000
    if [ "$prompt_length" -gt "$max_length" ]; then
        echo -e "\nConsider excluding files:" >&2
        # Save the list of unique file paths into a temporary file.
        local temp_files
        temp_files=$(mktemp)
        echo "$unique_found_files" > "$temp_files"
        # Source and call the helper to log file sizes.
        source "$SCRIPT_DIR/log-file-sizes.sh"
        # Capture the output from log_file_sizes.
        local sorted_output
        sorted_output=$(log_file_sizes "$temp_files")
        # Transform the output into exclusion suggestions.
        echo "$sorted_output" | awk -v curr="$prompt_length" -v max="$max_length" -v todo="$TODO_FILE_BASENAME" '{
  gsub(/\(/,"", $2);
  gsub(/\)/,"", $2);
  if ($1 == todo) next;  # Skip the TODO file
  file_size = $2;
  projected = curr - file_size;
  percentage = int((projected / max) * 100);
  print " --exclude " $1 " (will get you to " percentage "% of threshold)";
}' >&2

echo "$sorted_output" | awk -v curr="$prompt_length" -v max="$max_length" -v todo="$TODO_FILE_BASENAME" '{
  gsub(/\(/,"", $2);
  gsub(/\)/,"", $2);
  if ($1 == todo) next;  # Skip the TODO file
  file_size = $2;
  projected = curr - file_size;
  percentage = int((projected / max) * 100);
  print " --exclude " $1 " (will get you to " percentage "% of threshold)";
}'
        rm -f "$temp_files"
    fi

    # Copy the assembled prompt to the clipboard and print it.
    echo "$final_clipboard_content" | pbcopy
    echo "$final_clipboard_content"
}

# If executed directly, print usage instructions.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "Usage: source assemble-prompt.sh and call assemble-prompt <found_files_file> <instruction_content>" >&2
    exit 1
fi
