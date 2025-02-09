#!/bin/bash
# get-search-roots.sh
#
# This function returns a list of directories that are potential Swift package roots.
# If the provided root is already a package (contains Package.swift), then only that
# directory is returned.
#
# Usage: get-search-roots <git_root_or_package_root>
#
# Output: a list of directories (one per line).
get-search-roots() {
    local root="$1"
    
    # If the root itself is a Swift package (contains Package.swift), return it.
    if [ -f "$root/Package.swift" ]; then
        echo "$root"
        return 0
    fi

    # Otherwise, include the root if it is not a .build directory.
    if [ "$(basename "$root")" != ".build" ]; then
        echo "$root"
    fi

    # Find any subdirectories that contain Package.swift, but exclude those inside .build folders.
    find "$root" -type f -name "Package.swift" -not -path "*/.build/*" -exec dirname {} \; | sort -u
}

# Allow direct execution for testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
        echo "Usage: $0 <git_root_or_package_root>" >&2
        exit 1
    fi
    get-search-roots "$1"
fi
