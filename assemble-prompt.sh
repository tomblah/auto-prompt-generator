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
# If DIFF_WITH_BRANCH is set, a diff report is appended.
#
# NOTE: The file that contains the TODO instruction should be stored in the
#       environment variable TODO_FILE. Exclusion suggestions will not include this file.
#       If TODO_FILE is not set, it defaults to an empty string.
#
# New Option:
#   --chop <character_limit>
#       When supplied, only file blocks that keep the prompt under the given
#       character limit are added. Files that would cause the limit to be exceeded
#       are skipped and reported to the user.
#
# Debugging:
#   If VERBOSE is set to true, additional debug logs will be printed to stderr.

# Source the helper that filters file content based on substring markers.
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/filter-substring-markers.sh"

# If DIFF_WITH_BRANCH is set, source the diff helper.
if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
    source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/diff-with-branch.sh"
fi

# Source the helper to check prompt length.
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/check-prompt-length.sh"

assemble-prompt() {
    local found_files_file="$1"
    local instruction_content="$2"  # This parameter is now ignored.
    
    # Sort and deduplicate the file list.
    local unique_found_files
    unique_found_files=$(sort "$found_files_file" | uniq)
    
    local fixed_instruction="Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen"
    
    local clipboard_content=""
    
    # Declare arrays to store file names, their corresponding prompt blocks,
    # and any files that are skipped because adding them would exceed the chop limit.
    declare -a file_names
    declare -a file_blocks
    declare -a chopped_files

    if [ -n "${CHOP_LIMIT:-}" ]; then
        # --- CHOP MODE: Respect the user-supplied character limit ---
        local current_length=0
        while IFS= read -r file_path; do
            local file_basename file_content diff_output block block_length
            file_basename=$(basename "$file_path")
            
            if grep -qE '^[[:space:]]*//[[:space:]]*v' "$file_path"; then
                file_content=$(filter-substring-markers "$file_path")
            else
                file_content=$(cat "$file_path")
            fi
            
            # Build the block for this file.
            block=$'\nThe contents of '"$file_basename"$' is as follows:\n\n'"$file_content"$'\n\n'
            
            # If DIFF_WITH_BRANCH is set, append a diff report if applicable.
            if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
                diff_output=$(get_diff_with_branch "$file_path")
                if [ -n "$diff_output" ]; then
                    block+="\n--------------------------------------------------\nThe diff for ${file_basename} (against branch ${DIFF_WITH_BRANCH}) is as follows:\n\n${diff_output}\n\n"
                fi
            fi
            
            block+="\n--------------------------------------------------\n"
            
            block_length=$(echo -n "$block" | wc -c | xargs)
            if [ "${VERBOSE:-false}" = true ]; then
                echo "[DEBUG] Processing $file_basename: block_length=$block_length, current_length=$current_length, CHOP_LIMIT=$CHOP_LIMIT" >&2
            fi
            if [ $((current_length + block_length)) -le "$CHOP_LIMIT" ]; then
                clipboard_content+="$block"
                current_length=$((current_length + block_length))
                file_names+=("$file_basename")
                file_blocks+=("$block")
                if [ "${VERBOSE:-false}" = true ]; then
                    echo "[DEBUG] Accepted $file_basename; new current_length=$current_length" >&2
                fi
            else
                chopped_files+=("$file_basename")
                if [ "${VERBOSE:-false}" = true ]; then
                    echo "[DEBUG] Excluded $file_basename (would exceed CHOP_LIMIT)" >&2
                fi
            fi
        done <<< "$unique_found_files"
        # Append the fixed instruction.
        clipboard_content+="\n\n${fixed_instruction}"
        
        # Print a dashed separator and then the excluded files.
        echo "--------------------------------------------------" >&2
        if [ "${#chopped_files[@]}" -gt 0 ]; then
            echo "The following files were excluded due to the chop limit of ${CHOP_LIMIT} characters:" >&2
            for f in "${chopped_files[@]}"; do
                echo "  - $f" >&2
            done
        else
            echo "No files were excluded due to the chop limit of ${CHOP_LIMIT} characters." >&2
        fi
        
        # Print the final file list without an extra separator.
        echo "Files (final list):" >&2
        for f in "${file_names[@]}"; do
            echo "$f" >&2
        done
    else
        # --- ORIGINAL MODE (no chop limit) ---
        while IFS= read -r file_path; do
            local file_basename file_content diff_output block
            file_basename=$(basename "$file_path")
            
            if grep -qE '^[[:space:]]*//[[:space:]]*v' "$file_path"; then
                file_content=$(filter-substring-markers "$file_path")
            else
                file_content=$(cat "$file_path")
            fi
            
            block=$'\nThe contents of '"$file_basename"$' is as follows:\n\n'"$file_content"$'\n\n'
            
            if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
                diff_output=$(get_diff_with_branch "$file_path")
                if [ -n "$diff_output" ]; then
                    block+="\n--------------------------------------------------\nThe diff for ${file_basename} (against branch ${DIFF_WITH_BRANCH}) is as follows:\n\n${diff_output}\n\n"
                fi
            fi
            block+="\n--------------------------------------------------\n"
            
            clipboard_content+="$block"
            file_names+=("$file_basename")
            file_blocks+=("$block")
        done <<< "$unique_found_files"
        clipboard_content+="\n\n${fixed_instruction}"
    fi
    
    # Compute the final prompt length.
    local final_length
    final_length=$(echo -n "$clipboard_content" | wc -c | xargs)
    local threshold=${PROMPT_LENGTH_THRESHOLD:-600000}
    
    # Copy the assembled prompt to the clipboard.
    printf "%b" "$clipboard_content" | pbcopy
    
    # Output the assembled prompt.
    echo "$clipboard_content"
    
    # If the prompt is too long relative to a preset threshold, print exclusion suggestions.
    if [ "$final_length" -gt "$threshold" ]; then
        local suggestions=""
        for i in "${!file_blocks[@]}"; do
            # Skip this file if TODO_FILE is set and its basename matches the current file.
            if [ -n "${TODO_FILE:-}" ] && [ "$(basename "$TODO_FILE")" = "${file_names[$i]}" ]; then
                continue
            fi
            local block_length new_length percent
            block_length=$(echo -n "${file_blocks[$i]}" | wc -c | xargs)
            new_length=$((final_length - block_length))
            percent=$(awk -v l="$new_length" -v t="$threshold" 'BEGIN { printf "%.0f", (l/t)*100 }')
            suggestions="${suggestions} --exclude ${file_names[$i]} (will get you to ${percent}% of threshold)\n"
        done
        echo -e "\nSuggested exclusions:\n${suggestions}" >&2
    fi
}

# If executed directly, print usage instructions.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "Usage: source assemble-prompt.sh and call assemble-prompt <found_files_file> <instruction_content>" >&2
    exit 1
fi
