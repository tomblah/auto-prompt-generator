#!/bin/bash
# check-prompt-size.sh
#
# This helper defines a function to check the size of the prompt
# (in characters) and print a warning if it exceeds a specified threshold.
#
# Usage:
#   check_prompt_size "<prompt_text>"
#
# The threshold is set to 100,000 characters by default, but you can adjust it.
check_prompt_size() {
    local prompt_text="$1"
    local prompt_length
    # Use printf to output the text without adding a newline.
    prompt_length=$(printf "%s" "$prompt_text" | wc -m | tr -d ' ')
    local max_length=100000
    if [ "$prompt_length" -gt "$max_length" ]; then
        echo "Warning: The prompt is ${prompt_length} characters long. This may exceed what the AI can handle effectively." >&2
    fi
}
