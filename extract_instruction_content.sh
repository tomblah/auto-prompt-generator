#!/bin/bash
# extract_instruction_content.sh
#
# This function extracts the TODO instruction content from a given Swift file.
# It looks for a line that matches either "// TODO: - " or "// TODO: ChatGPT: ".
#
# Usage: extract_instruction_content <swift_file>
#
# On success: prints the extracted instruction line (trimmed).
# On failure: prints an error message and returns a non-zero exit code.
extract_instruction_content() {
    local swift_file="$1"
    local instruction_line

    # Search for the matching TODO instruction.
    instruction_line=$(grep -E '// TODO: (ChatGPT: |- )' "$swift_file" | head -n 1)
    
    if [ -z "$instruction_line" ]; then
        echo "Error: No valid TODO instruction found in $swift_file" >&2
        return 1
    fi

    # Trim leading whitespace and output the result.
    echo "$instruction_line" | sed 's/^[[:space:]]*//'
}

# Allow direct execution for a quick test.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
        echo "Usage: $0 <swift_file>" >&2
        exit 1
    fi
    extract_instruction_content "$1"
fi
