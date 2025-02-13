#!/bin/bash
# diff-with-branch.sh
#
# This function generates a diff for a given file between the current branch and
# the branch specified in the environment variable DIFF_WITH_BRANCH.
# It assumes that the file is tracked by Git. If the file is unmodified relative to
# the given branch, an empty string is returned.
#
# Usage: get_diff_with_branch <file_path>
get_diff_with_branch() {
    local file="$1"
    local branch="${DIFF_WITH_BRANCH:-main}"
    
    # Check if the file is tracked in Git.
    if ! git ls-files --error-unmatch "$file" >/dev/null 2>&1; then
        echo "[DEBUG] File '$file' is not tracked by Git." >&2
        return 0
    fi

    # Get the diff between the file in the current branch and the given branch.
    local git_diff
    git_diff=$(git diff "$branch" -- "$file")
    echo "[DEBUG] git diff for '$file' (against '$branch'):" >&2
    echo "[DEBUG] $git_diff" >&2
    echo "$git_diff"
}

# Allow direct testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
        echo "Usage: $0 <file_path>" >&2
        exit 1
    fi
    get_diff_with_branch "$1"
fi
