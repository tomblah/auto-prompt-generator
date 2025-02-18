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
        # Source our helper for first-segment extraction.
        source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/extract-first-segment.sh"
        
        local current_length=0
        
        # --- Always include the TODO file first ---
        local todo_basename="" todo_block="" diff_output="" file_content=""
        local todo_root="" todo_segment=""
        if [ -n "${TODO_FILE:-}" ] && [ -f "$TODO_FILE" ]; then
            todo_basename=$(basename "$TODO_FILE")
            todo_root="${todo_basename%.*}"
            todo_segment=$(extract-first-segment "$TODO_FILE")
            if grep -qE '^[[:space:]]*//[[:space:]]*v' "$TODO_FILE"; then
                file_content=$(filter-substring-markers "$TODO_FILE")
            else
                file_content=$(cat "$TODO_FILE")
            fi
            # Replace the TODO marker.
            file_content=$(echo "$file_content" | sed 's,// TODO: - ,// TODO: ChatGPT: ,')
            todo_block=$'\nThe contents of '"$todo_basename"$' is as follows:\n\n'"$file_content"$'\n\n'
            if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
                diff_output=$(get_diff_with_branch "$TODO_FILE")
                if [ -n "$diff_output" ]; then
                    todo_block+="\n--------------------------------------------------\nThe diff for ${todo_basename} (against branch ${DIFF_WITH_BRANCH}) is as follows:\n\n${diff_output}\n\n"
                fi
            fi
            todo_block+="\n--------------------------------------------------\n"
            clipboard_content+="$todo_block"
            current_length=$(echo -n "$todo_block" | wc -c | xargs)
            file_names+=("$todo_basename")
            file_blocks+=("$todo_block")
        fi

        # --- Build grouping arrays from unique_found_files, skipping the TODO file ---
        declare -a first_class_files=()
        declare -a segment_files=()
        declare -a related_files=()
        declare -a other_files=()
        while IFS= read -r file_path; do
            if [ -z "$file_path" ]; then
                continue
            fi
            if [ "$file_path" = "$TODO_FILE" ]; then
                continue
            fi
            local base file_root candidate_segment
            base=$(basename "$file_path")
            file_root="${base%.*}"
            candidate_segment=$(extract-first-segment "$file_path")
            
            # NEW: Only treat as first-class if the TODO file contains the primary marker.
            if grep -q "// TODO: - " "$TODO_FILE"; then
                if echo "$instruction_content" | grep -qw "$file_root"; then
                    first_class_files+=("$file_path")
                    continue
                fi
            fi

            if [[ "$todo_segment" == *"$candidate_segment"* ]] || [[ "$candidate_segment" == *"$todo_segment"* ]]; then
                segment_files+=("$file_path")
            elif [[ "$todo_root" == *"$file_root"* ]] || [[ "$file_root" == *"$todo_root"* ]]; then
                related_files+=("$file_path")
            else
                other_files+=("$file_path")
            fi
        done <<< "$unique_found_files"
        
        # --- Process first-class files (immune to chopping) ---
        for file_path in "${first_class_files[@]:-}"; do
            if [ -z "$file_path" ]; then continue; fi
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
            # Always include first-class files regardless of the CHOP_LIMIT.
            clipboard_content+="$block"
            current_length=$(( current_length + $(echo -n "$block" | wc -c | xargs) ))
            file_names+=("$file_basename")
            file_blocks+=("$block")
            if [ "${VERBOSE:-false}" = true ]; then
                echo "[DEBUG] Including (first-class) $file_basename regardless of CHOP_LIMIT" >&2
            fi
        done
        
        # --- Process segment files ---
        for file_path in "${segment_files[@]:-}"; do
            if [ -z "$file_path" ]; then continue; fi
            local file_basename file_content diff_output block block_length
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
            block_length=$(echo -n "$block" | wc -c | xargs)
            if [ "${VERBOSE:-false}" = true ]; then
                echo "[DEBUG] Processing (segment) $file_basename: block_length=$block_length, current_length=$current_length, CHOP_LIMIT=$CHOP_LIMIT" >&2
            fi
            if [ $((current_length + block_length)) -le "$CHOP_LIMIT" ]; then
                clipboard_content+="$block"
                current_length=$((current_length + block_length))
                file_names+=("$file_basename")
                file_blocks+=("$block")
            else
                chopped_files+=("$file_basename")
                if [ "${VERBOSE:-false}" = true ]; then
                    echo "[DEBUG] Excluded (segment) $file_basename (would exceed CHOP_LIMIT)" >&2
                fi
            fi
        done
        
        # --- Process related files ---
        for file_path in "${related_files[@]:-}"; do
            if [ -z "$file_path" ]; then continue; fi
            local file_basename file_content diff_output block block_length
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
            block_length=$(echo -n "$block" | wc -c | xargs)
            if [ "${VERBOSE:-false}" = true ]; then
                echo "[DEBUG] Processing (related) $file_basename: block_length=$block_length, current_length=$current_length, CHOP_LIMIT=$CHOP_LIMIT" >&2
            fi
            if [ $((current_length + block_length)) -le "$CHOP_LIMIT" ]; then
                clipboard_content+="$block"
                current_length=$((current_length + block_length))
                file_names+=("$file_basename")
                file_blocks+=("$block")
            else
                chopped_files+=("$file_basename")
                if [ "${VERBOSE:-false}" = true ]; then
                    echo "[DEBUG] Excluded (related) $file_basename (would exceed CHOP_LIMIT)" >&2
                fi
            fi
        done
        
        # --- Process other files ---
        for file_path in "${other_files[@]:-}"; do
            if [ -z "$file_path" ]; then continue; fi
            local file_basename file_content diff_output block block_length
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
            block_length=$(echo -n "$block" | wc -c | xargs)
            if [ "${VERBOSE:-false}" = true ]; then
                echo "[DEBUG] Processing (other) $file_basename: block_length=$block_length, current_length=$current_length, CHOP_LIMIT=$CHOP_LIMIT" >&2
            fi
            if [ $((current_length + block_length)) -le "$CHOP_LIMIT" ]; then
                clipboard_content+="$block"
                current_length=$((current_length + block_length))
                file_names+=("$file_basename")
                file_blocks+=("$block")
            else
                chopped_files+=("$file_basename")
                if [ "${VERBOSE:-false}" = true ]; then
                    echo "[DEBUG] Excluded (other) $file_basename (would exceed CHOP_LIMIT)" >&2
                fi
            fi
        done
        
        # Append the fixed instruction.
        clipboard_content+="\n\n${fixed_instruction}"
        
        # Print a dashed separator and then the excluded files.
        echo "--------------------------------------------------" >&2
        if [ "${#chopped_files[@]:-0}" -gt 0 ]; then
            echo "The following files were excluded due to the chop limit of ${CHOP_LIMIT} characters:" >&2
            for f in "${chopped_files[@]:-}"; do
                if [ -z "$f" ]; then continue; fi
                echo "  - $f" >&2
            done
        else
            echo "No files were excluded due to the chop limit of ${CHOP_LIMIT} characters." >&2
        fi
        
        # Print the final file list without an extra separator.
        echo "Files (final list):" >&2
        for f in "${file_names[@]:-}"; do
            if [ -z "$f" ]; then continue; fi
            echo "$f" >&2
        done
        
    else
        # --- ORIGINAL MODE (no chop limit) ---
        while IFS= read -r file_path; do
            if [ -z "$file_path" ]; then
                continue
            fi
            local file_basename file_content diff_output block
            file_basename=$(basename "$file_path")
            
            if grep -qE '^[[:space:]]*//[[:space:]]*v' "$file_path"; then
                file_content=$(filter-substring-markers "$file_path")
            else
                file_content=$(cat "$file_path")
            fi

            # If this is the TODO file, replace the TODO marker.
            if [ "$file_path" = "$TODO_FILE" ]; then
                file_content=$(echo "$file_content" | sed 's,// TODO: - ,// TODO: ChatGPT: ,')
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
        # Iterate over every file path from the sorted unique list (skipping empty lines and the TODO file)
        while IFS= read -r file_path; do
            if [ -z "$file_path" ]; then continue; fi
            if [ "$file_path" = "$TODO_FILE" ]; then continue; fi
            local file_basename
            file_basename=$(basename "$file_path")
            # Build the block for this file (using the same logic as above)
            local file_content
            if grep -qE '^[[:space:]]*//[[:space:]]*v' "$file_path"; then
                file_content=$(filter-substring-markers "$file_path")
            else
                file_content=$(cat "$file_path")
            fi
            local block
            block=$'\nThe contents of '"$file_basename"' is as follows:\n\n'"$file_content"$'\n\n'
            if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
                local diff_output
                diff_output=$(get_diff_with_branch "$file_path")
                if [ -n "$diff_output" ]; then
                    block+="\n--------------------------------------------------\nThe diff for ${file_basename} (against branch ${DIFF_WITH_BRANCH}) is as follows:\n\n${diff_output}\n\n"
                fi
            fi
            block+="\n--------------------------------------------------\n"
            local block_length
            block_length=$(echo -n "$block" | wc -c | xargs)
            local new_length=$((final_length - block_length))
            local percent
            percent=$(awk -v l="$new_length" -v t="$threshold" 'BEGIN { printf "%.0f", (l/t)*100 }')
            suggestions="${suggestions} --exclude ${file_basename} (will get you to ${percent}% of threshold)\n"
        done <<< "$unique_found_files"
        echo -e "\nSuggested exclusions:\n${suggestions}" >&2
    fi
}

# If executed directly, print usage instructions.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "Usage: source assemble-prompt.sh and call assemble-prompt <found_files_file> <instruction_content>" >&2
    exit 1
fi
