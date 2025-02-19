#!/bin/bash
# assemble-prompt.sh
#
# This function assembles the final AI prompt by including:
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

# Source our new helper to remove additional TODO markers.
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/remove-other-todo-markers.sh"

# If DIFF_WITH_BRANCH is set, source the diff helper.
if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
    source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/diff-with-branch.sh"
fi

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
            todo_segment=$(extract_first_segment "$TODO_FILE")
            if grep -qE '^[[:space:]]*//[[:space:]]*v' "$TODO_FILE"; then
                file_content=$(filter-substring-markers "$TODO_FILE")
            else
                file_content=$(cat "$TODO_FILE")
            fi
            # For the primary file, use Perl to replace only the first occurrence of the special TODO marker.
            file_content=$(echo "$file_content" | awk 'BEGIN {found=0} { if ($0 ~ /^[ \t]*\/\/[ \t]*TODO: - /) { if (found==0) { print; found=1 } } else { print } }')
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
            candidate_segment=$(extract_first_segment "$file_path")
            
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
            # Scrub any extra special TODO markers in non-primary files.
            if [ "$file_path" != "$TODO_FILE" ]; then
                file_content=$(remove_other_todo_markers "$file_content")
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
            if [ "$file_path" != "$TODO_FILE" ]; then
                file_content=$(remove_other_todo_markers "$file_content")
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
            if [ "$file_path" != "$TODO_FILE" ]; then
                file_content=$(remove_other_todo_markers "$file_content")
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
            if [ "$file_path" != "$TODO_FILE" ]; then
                file_content=$(remove_other_todo_markers "$file_content")
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

            if [ "$file_path" = "$TODO_FILE" ]; then
                # For the primary TODO file, keep the first occurrence of "// TODO: - " unchanged and remove any later ones.
                file_content=$(echo "$file_content" | awk 'BEGIN {found=0} { if ($0 ~ /^[ \t]*\/\/[ \t]*TODO: - /) { if (found==0) { print; found=1 } } else { print } }')
            else
                # For all other files, scrub any special marker lines.
                file_content=$(remove_other_todo_markers "$file_content")
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
    
    # Copy the assembled prompt to the clipboard.
    printf "%b" "$clipboard_content" | pbcopy
    
    # Output the assembled prompt.
    echo "$clipboard_content"
}

# If executed directly, print usage instructions.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    echo "Usage: source assemble-prompt.sh and call assemble-prompt <found_files_file> <instruction_content>" >&2
    exit 1
fi
