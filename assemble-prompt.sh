#!/bin/bash
# assemble-prompt.sh
#
# This function assembles the final ChatGPT prompt by including:
#   - The contents of Swift files where type definitions were found (optionally filtered by substring markers), and
#   - A fixed instruction (instead of the extracted TODO).
#
# It takes two parameters:
#   1. <found_files_file>: A file (typically temporary) containing a list of Swift file paths.
#   2. <instruction_content>: The TODO instruction content that is now ignored.
#
# The function outputs the final assembled prompt to stdout and also copies it
# to the clipboard using pbcopy.

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the helper that filters file content based on substring markers.
# IMPORTANT: Ensure that filter-substring-markers.sh uses regexes without "\b".
source "$SCRIPT_DIR/filter-substring-markers.sh"

assemble-prompt() {
    local found_files_file="$1"
    local instruction_content="$2"  # This parameter is no longer used.
    
    # Sort and filter out duplicate file paths.
    local unique_found_files
    unique_found_files=$(sort "$found_files_file" | uniq)
    
    local clipboard_content=""
    
    # Process each file and format its content.
    while IFS= read -r file_path; do
        local file_basename
        file_basename=$(basename "$file_path")
        local file_content
        # Check if the file contains substring markers (an opening marker "// v").
        if grep -qE '^[[:space:]]*//[[:space:]]*v' "$file_path"; then
            file_content=$(filter_substring_markers "$file_path")
        else
            file_content=$(cat "$file_path")
        fi
    
        clipboard_content+=$(printf "\nThe contents of %s is as follows:\n\n%s\n\n--------------------------------------------------\n" "$file_basename" "$file_content")
    done <<< "$unique_found_files"
    
    # (The extracted TODO instruction is now ignored.)
    local modified_clipboard_content="$clipboard_content"
    
    # Use the fixed instruction instead.
    local fixed_instruction="Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen"
    
    local final_clipboard_content
    final_clipboard_content=$(printf "%s\n\n%s" "$modified_clipboard_content" "$fixed_instruction")
    
    # Copy the assembled prompt to the clipboard.
    echo -e "$final_clipboard_content" | pbcopy
    
    # Optionally, print the final content (for logging or debugging).
    echo "$final_clipboard_content"
}

# If this file is executed directly, print usage instructions.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "Usage: source assemble-prompt.sh and call assemble-prompt <found_files_file> <instruction_content>" >&2
    exit 1
fi
