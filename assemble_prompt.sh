#!/bin/bash
# assemble_prompt.sh
#
# This function assembles the final ChatGPT prompt by including:
#   - The contents of Swift files where type definitions were found, and
#   - The extracted TODO instruction.
#
# It takes two parameters:
#   1. <found_files_file>: A file (typically temporary) containing a list of Swift file paths.
#   2. <instruction_content>: The TODO instruction content to be appended.
#
# Usage: assemble_prompt <found_files_file> <instruction_content>
#
# The function outputs the final assembled prompt to stdout and also copies it to the clipboard using pbcopy.
assemble_prompt() {
    local found_files_file="$1"
    local instruction_content="$2"
    
    # Sort and filter out duplicate file paths.
    local unique_found_files
    unique_found_files=$(sort "$found_files_file" | uniq)
    
    local clipboard_content=""
    
    # Process each Swift file and format its content.
    while IFS= read -r file_path; do
        local file_basename
        file_basename=$(basename "$file_path")
        local file_content
        file_content=$(cat "$file_path")
        
        # Append a header and the file content, followed by a separator.
        clipboard_content+=$(printf "The contents of %s is as follows:\n\n%s\n\n--------------------------------------------------\n" "$file_basename" "$file_content")
    done <<< "$unique_found_files"
    
    # Replace "// TODO: - " with "// TODO: ChatGPT: " for consistency.
    local modified_clipboard_content
    modified_clipboard_content=$(echo -e "$clipboard_content" | sed 's/\/\/ TODO: - /\/\/ TODO: ChatGPT: /g')
    
    # Append the instruction content.
    local final_clipboard_content
    final_clipboard_content=$(printf "%s\n\n%s" "$modified_clipboard_content" "$instruction_content")
    
    # Copy the assembled prompt to the clipboard.
    echo -e "$final_clipboard_content" | pbcopy
    
    # Optionally, print the final content (for logging or debugging).
    echo "$final_clipboard_content"
}

# If this file is executed directly, print usage instructions.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "Usage: source assemble_prompt.sh and call assemble_prompt <found_files_file> <instruction_content>" >&2
    exit 1
fi
