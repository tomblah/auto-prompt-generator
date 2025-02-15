#!/bin/bash
# assemble-prompt.sh
#
# This function assembles the final ChatGPT prompt by including:
#   - The contents of files where type definitions were found
#     (optionally filtered by substring markers), and
#   - A fixed instruction (ignoring the extracted TODO instruction).
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

## Use the Rust binary for filtering substring markers.
RUST_FILTER_SUBSTR="$SCRIPT_DIR/rust/target/release/filter_substring_markers"
if [ ! -x "$RUST_FILTER_SUBSTR" ]; then
    echo "Error: Rust filter_substring_markers binary not found. Please build it with 'cargo build --release'." >&2
    exit 1
fi

## Use the Rust binary for checking prompt size.
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
    
    # Sort and remove duplicate file paths.
    local unique_found_files
    unique_found_files=$(sort "$found_files_file" | uniq)
    
    local clipboard_content=""
    
    # Process each file and format its content.
    while IFS= read -r file_path; do
        local file_basename file_content diff_output
        file_basename=$(basename "$file_path")
        
        # If the file contains a line exactly matching the substring marker ("// v"),
        # then process it with the Rust binary.
        if grep -qE '^[[:space:]]*//[[:space:]]*v[[:space:]]*$' "$file_path"; then
            file_content=$("$RUST_FILTER_SUBSTR" "$file_path")
            # If this is the TODO file, attempt to extract its enclosing function context.
            if [ "$file_basename" = "$TODO_FILE_BASENAME" ]; then
                extra_context=$("$SCRIPT_DIR/rust/target/release/extract_enclosing_function" "$file_path")
                if [ -n "$extra_context" ]; then
                    file_content="${file_content}"$'\n\n// Enclosing function context:\n'"${extra_context}"
                fi
            fi
        else
            file_content=$(cat "$file_path")
        fi
        
        clipboard_content="${clipboard_content}"$'\nThe contents of '"${file_basename}"' is as follows:\n\n'"${file_content}"$'\n\n'
        # If DIFF_WITH_BRANCH is set, append a diff report (if there are changes).
        if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
            diff_output=$(get_diff_with_branch "$file_path")
            if [ -n "$diff_output" ]; then
                clipboard_content="${clipboard_content}"$'\n--------------------------------------------------\nThe diff for '"${file_basename}"' (against branch '"${DIFF_WITH_BRANCH}"') is as follows:\n\n'"${diff_output}"$'\n\n'
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

    # Copy the assembled prompt to the clipboard and print it.
    echo "$final_clipboard_content" | pbcopy
    echo "$final_clipboard_content"
}

# If executed directly, print usage instructions.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "Usage: source assemble-prompt.sh and call assemble-prompt <found_files_file> <instruction_content>" >&2
    exit 1
fi
