#!/bin/bash
# find_prompt_instruction.sh

# This function looks for a unique Swift file that contains a
# single instance of a TODO instruction marked by either:
#   - "// TODO: - "  OR
#   - "// TODO: ChatGPT: "
#
# If no such file exists or if there’s more than one, it outputs an error message
# and returns a non-zero exit code.
#
# Usage: find_prompt_instruction <search_directory>
#
# Outputs:
#   On success: prints the file path containing the unique instruction.
#   On failure: prints an error message to stderr and returns with non-zero exit code.
find_prompt_instruction() {
    local search_dir="$1"
    # Match either "// TODO: ChatGPT: " or "// TODO: - " (note the trailing space)
    local grep_pattern='// TODO: (ChatGPT: |- )'
    local matching_lines occurrence_count file_path instruction_content

    # Search for matching lines in Swift files under the provided directory.
    matching_lines=$(grep -rnE "$grep_pattern" --include "*.swift" "$search_dir" || true)
    occurrence_count=$(echo "$matching_lines" | grep -c .)

    if [ "$occurrence_count" -eq 0 ]; then
        echo "Error: No Swift files found containing either '// TODO: - ' or '// TODO: ChatGPT: '" >&2
        return 1
    elif [ "$occurrence_count" -gt 1 ]; then
        echo "Error: More than one instruction found:" >&2
        echo "$matching_lines" | cut -d: -f3- | sed 's/^[[:space:]]*//' >&2
        return 1
    fi

    # Extract the file path and the instruction content.
    file_path=$(echo "$matching_lines" | head -n 1 | cut -d: -f1)
    instruction_content=$(echo "$matching_lines" | cut -d: -f3- | sed 's/^[[:space:]]*//')

    # For now, simply print the file path.
    # (You could also export/store instruction_content in a global variable if needed.)
    echo "$file_path"
}

# Allow running this file directly for a quick manual test.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
        echo "Usage: $0 <search_directory>" >&2
        exit 1
    fi
    find_prompt_instruction "$1"
fi
