#!/bin/bash
# get-package-root.sh
#
# This function determines the root of a Swift package by looking for a Package.swift
# in the current directory or one of its ancestors.
#
# Usage: get-package-root <file_path>
#
# On success: prints the package root directory.
# On failure: prints nothing (caller may then use the Git root).
get-package-root() {
    local file_path="$1"
    local dir
    # Start in the directory of the given file.
    dir=$(dirname "$file_path")
    # Walk upward until reaching the filesystem root.
    while [ "$dir" != "/" ]; do
        if [ -f "$dir/Package.swift" ]; then
            echo "$dir"
            return 0
        fi
        dir=$(dirname "$dir")
    done
    return 1
}

# Allow direct execution for testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
        echo "Usage: $0 <file_path>" >&2
        exit 1
    fi
    get-package-root "$1"
fi
