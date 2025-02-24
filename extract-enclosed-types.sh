#!/bin/bash
# extract-enclosed-types.sh
#
# This helper extracts type names (e.g. classes, structs, enums) from the code region
# between the first occurrence of the "// TODO: - " marker and the matching closing brace.
#
# Usage:
#   extract_enclosed_types <swift_file>
#
# It outputs a sorted, unique list of type names found in that enclosed region.

extract_enclosed_types() {
    local file="$1"
    # Use awk to start scanning after the TODO marker and count braces.
    # When the brace count returns to 0, stop printing.
    awk '
    BEGIN { found=0; count=0 }
    {
      if (!found && $0 ~ /\/\/[[:space:]]*TODO: - /) {
         found=1; next
      }
      if (found) {
         count += gsub(/{/, "{")
         count -= gsub(/}/, "}")
         print $0
         if (count <= 0) { exit }
      }
    }
    ' "$file" |
    # Extract capitalized words as potential type names.
    grep -oE '\b[A-Z][A-Za-z0-9]+\b' | sort | uniq
}

# Allow direct testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
         echo "Usage: $0 <swift_file>" >&2
         exit 1
    fi
    extract_enclosed_types "$1"
fi
