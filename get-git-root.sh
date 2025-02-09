#!/bin/bash
# get-git-root.sh
#
# This function determines the Git repository root directory based on the current working directory.
# If the current directory is not within a Git repository, it outputs an error message and returns a non-zero status.
#
# Usage: get-git-root
#
# Outputs:
#   On success: prints the Git repository root directory.
#   On failure: prints an error message to stderr and returns a non-zero status.
get-git-root() {
    local git_root
    git_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
        echo "Error: Not a git repository." >&2
        return 1
    }
    echo "$git_root"
}

# Allow running this file directly for a quick test.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 0 ]; then
        echo "Usage: $0" >&2
        exit 1
    fi
    get-git-root
fi
