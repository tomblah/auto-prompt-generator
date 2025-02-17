#!/bin/bash
# assemble-prompt.sh
#
# This function assembles the final ChatGPT prompt by including:
#   - The contents of files where type definitions were found
#     (processed by prompt_file_processor to filter substring markers
#      and, if applicable, append extra context), and
#   - A fixed instruction.
#
# It takes two parameters:
#   1. <found_files_file>: A file (typically temporary) containing a list of file paths.
#   2. <instruction_content>: The TODO instruction content (now ignored).
#
# The final prompt is copied to the clipboard via pbcopy.
#
# If DIFF_WITH_BRANCH is set (e.g. --diff-with develop),
# for each file that differs from that branch a diff report is appended.

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

## Use the new Rust binary for processing individual files.
RUST_PROMPT_FILE_PROCESSOR="$SCRIPT_DIR/rust/target/release/prompt_file_processor"
if [ ! -x "$RUST_PROMPT_FILE_PROCESSOR" ]; then
    echo "Error: Rust prompt_file_processor binary not found. Please build it with 'cargo build --release'." >&2
    exit 1
fi

## Use the Rust binary for checking prompt size.
RUST_CHECK_SIZE="$SCRIPT_DIR/rust/target/release/check_prompt_size"
if [ ! -x "$RUST_CHECK_SIZE" ]; then
    echo "Error: Rust check_prompt_size binary not found. Please build it with 'cargo build --release'." >&2
    exit 1
fi

## Use the Rust binary for diffing.
RUST_DIFF_WITH_BRANCH="$SCRIPT_DIR/rust/target/release/diff_with_branch"
if [ ! -x "$RUST_DIFF_WITH_BRANCH" ]; then
    echo "Error: Rust diff_with_branch binary not found. Please build it with 'cargo build --release'." >&2
    exit 1
fi

## Use the Rust binary for unescaping newlines.
RUST_UNESCAPE_NEWLINES="$SCRIPT_DIR/rust/target/release/unescape_newlines"
if [ ! -x "$RUST_UNESCAPE_NEWLINES" ]; then
    echo "Error: Rust unescape_newlines binary not found. Please build it with 'cargo build --release'." >&2
    exit 1
fi

assemble-prompt() {
    local found_files_file="$1"
    local instruction_content="$2"  # This parameter is now ignored.
    
    # Sort and remove duplicate file paths.
    local unique_found_files
    unique_found_files=$(sort "$found_files_file" | uniq)
    
    local clipboard_content=""
    
    # Process each file and format its content.
    while IFS= read -r file_path; do
        local file_basename file_content raw_diff_output
        file_basename=$(basename "$file_path")
        
        # Use the new Rust binary to process the file.
        # It handles substring marker filtering and, if applicable,
        # appends extra context if the file matches TODO_FILE_BASENAME.
        file_content=$("$RUST_PROMPT_FILE_PROCESSOR" "$file_path" "$TODO_FILE_BASENAME")
        
        clipboard_content="${clipboard_content}"$'\nThe contents of '"${file_basename}"' is as follows:\n\n'"${file_content}"$'\n\n'
        
        # If DIFF_WITH_BRANCH is set, append a diff report (if there are changes).
        if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
            raw_diff_output=$("$RUST_DIFF_WITH_BRANCH" "$file_path")
            # If the diff output, with all whitespace removed, equals the file's basename,
            # then treat it as if there were no diff.
            if [ "$(echo -n "$raw_diff_output" | tr -d '[:space:]')" = "$file_basename" ]; then
                raw_diff_output=""
            fi
            if [ -n "$raw_diff_output" ]; then
                clipboard_content="${clipboard_content}"$'\n--------------------------------------------------\nThe diff for '"${file_basename}"' (against branch '"${DIFF_WITH_BRANCH}"') is as follows:\n\n'"${raw_diff_output}"$'\n\n'
            fi
        fi
        
        clipboard_content="${clipboard_content}"$'\n--------------------------------------------------\n'
    done <<< "$unique_found_files"
    
    # Fixed instruction to be appended.
    local fixed_instruction="Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen"
    
    local final_clipboard_content="${clipboard_content}"$'\n\n'"${fixed_instruction}"
    
    # Check prompt size using the Rust binary.
    warning_output=$( (trap '' SIGPIPE; printf "%s" "$final_clipboard_content") | "$RUST_CHECK_SIZE" 2>&1 || true )
    if [ -n "$warning_output" ]; then
        echo "$warning_output"
    fi

    # Additional debug logging for prompt size.
    local prompt_length
    prompt_length=$(printf "%s" "$final_clipboard_content" | wc -m | tr -d ' ')
    local max_length=100000
    if [ "$prompt_length" -gt "$max_length" ]; then
        echo -e "\nConsider excluding files:" >&2
        local temp_files
        temp_files=$(mktemp)
        echo "$unique_found_files" > "$temp_files"
        local sorted_output
        sorted_output=$("$SCRIPT_DIR/rust/target/release/log_file_sizes" "$temp_files")
        echo "$sorted_output" | awk -v curr="$prompt_length" -v max="$max_length" -v todo="$TODO_FILE_BASENAME" '{
  gsub(/\(/,"", $2);
  gsub(/\)/,"", $2);
  if ($1 == todo) next;
  file_size = $2;
  projected = curr - file_size;
  percentage = int((projected / max) * 100);
  print " --exclude " $1 " (will get you to " percentage "% of threshold)";
}' >&2
        echo "$sorted_output" | awk -v curr="$prompt_length" -v max="$max_length" -v todo="$TODO_FILE_BASENAME" '{
  gsub(/\(/,"", $2);
  gsub(/\)/,"", $2);
  if ($1 == todo) next;
  file_size = $2;
  projected = curr - file_size;
  percentage = int((projected / max) * 100);
  print " --exclude " $1 " (will get you to " percentage "% of threshold)";
}'
        rm -f "$temp_files"
    fi

    # Unescape literal "\n" sequences using the Rust binary before copying.
    if [ -x "$RUST_UNESCAPE_NEWLINES" ]; then
        echo "$final_clipboard_content" | "$RUST_UNESCAPE_NEWLINES" | pbcopy
        echo "$final_clipboard_content" | "$RUST_UNESCAPE_NEWLINES"
    else
        echo "$final_clipboard_content" | pbcopy
        echo "$final_clipboard_content"
    fi
}

# If executed directly, print usage instructions.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "Usage: source assemble-prompt.sh and call assemble-prompt <found_files_file> <instruction_content>" >&2
    exit 1
fi
