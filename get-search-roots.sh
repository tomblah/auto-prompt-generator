#!/bin/bash
# get-search-roots.sh
#
# This function returns a list of directories that are potential Swift package roots.
# It includes the main git repository root and any subdirectories that contain a Package.swift.
#
# Usage: get-search-roots <git_root>
#
# Output: a list of directories (one per line).
get-search-roots() {
    local git_root="$1"
    # Always include the git root.
    echo "$git_root"
    # Find all directories with Package.swift inside the git repo.
    find "$git_root" -type f -name "Package.swift" -exec dirname {} \; | sort -u
}

# Allow direct execution for testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
        echo "Usage: $0 <git_root>" >&2
        exit 1
    fi
    get-search-roots "$1"
fi
