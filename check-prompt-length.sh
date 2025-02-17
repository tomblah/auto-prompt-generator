#!/bin/bash
# check-prompt-length.sh
#
# This function checks the length of the prompt (in characters).
# If the length exceeds PROMPT_LENGTH_THRESHOLD (default: 600000),
# it prints a warning message.
#
# Usage: check_prompt_length "<prompt_content>"

check_prompt_length() {
    local prompt="$1"
    # Trim any extra spaces from the wc output using xargs.
    local length
    length=$(echo -n "$prompt" | wc -c | xargs)
    local threshold=${PROMPT_LENGTH_THRESHOLD:-600000}

    if [ "$length" -gt "$threshold" ]; then
        echo "Warning: The prompt is ${length} characters long. This may exceed what the AI can handle effectively." >&2
    fi
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    check_prompt_length "$1"
fi

