#!/bin/bash
# extract-first-segment.sh
#
# This function extracts the first segment of a filename.
# A “segment” is defined as the leading capitalized word,
# i.e. from the beginning up until (but not including) the next capital letter.
#
# For example, for "HandleView.swift" it returns "Handle".
#
# Usage:
#   extract_first_segment <file_path>
extract_first_segment() {
    local file="$1"
    local base
    base=$(basename "$file")
    base="${base%.*}"  # Remove file extension
    # Try to match a capital letter followed by lowercase letters.
    if [[ $base =~ ^([A-Z][a-z]+) ]]; then
        echo "${BASH_REMATCH[1]}"
    else
        # If no match, fall back to the entire base name.
        echo "$base"
    fi
}

# Allow direct testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
         echo "Usage: $0 <file_path>" >&2
         exit 1
    fi
    extract_first_segment "$1"
fi
