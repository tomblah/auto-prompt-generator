#!/bin/bash
# remove-other-todo-markers.sh
#
# This function removes lines that match the special marker pattern
# (“// TODO: - ”) from the provided content.
#
# Usage:
#   remove_other_todo_markers "<file_content>"
#
# It prints the cleaned content.
remove_other_todo_markers() {
    local content="$1"
    # Remove any line that (after trimming) starts with "// TODO: - "
    echo "$content" | sed '/^[[:space:]]*\/\/[[:space:]]*TODO: - /d'
}

# Allow direct testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
        echo "Usage: $0 <file_content>" >&2
        exit 1
    fi
    remove_other_todo_markers "$1"
fi
