#!/bin/bash
# filter-substring-markers.sh
#
# This function checks if a given file contains “substring markers.”
# The markers are defined as follows:
#   - An opening marker: a line that, when trimmed, exactly matches:
#         // v
#   - A closing marker: a line that, when trimmed, exactly matches:
#         // ^
#
# If these markers are found in the file, only the text between them is output.
# Any omitted regions (before the first block, between blocks, and after the last block)
# are replaced with a single placeholder (with an extra blank line above and below):
#
#         (blank line)
#         // ...
#         (blank line)
#
# If no markers are found, the entire file is output unchanged.
#
# Usage:
#   filter_substring_markers <file_path>
filter_substring_markers() {
    local file="$1"
    # If no opening marker exists (strictly matching), output the file unchanged.
    if ! grep -qE '^[[:space:]]*//[[:space:]]*v[[:space:]]*$' "$file"; then
        cat "$file"
        return 0
    fi

    awk '
    BEGIN {
        inBlock = 0;
        lastWasPlaceholder = 0;
    }
    # Function to print a placeholder (with extra blank lines above and below)
    # only if the previous printed line was not already a placeholder.
    function printPlaceholder() {
        if (lastWasPlaceholder == 0) {
            print "";
            print "// ...";
            print "";
            lastWasPlaceholder = 1;
        }
    }
    {
        # Check for the opening marker: when trimmed, the line must be exactly "// v"
        if ($0 ~ /^[[:space:]]*\/\/[[:space:]]*v[[:space:]]*$/) {
            printPlaceholder();
            inBlock = 1;
            next;
        }
        # Check for the closing marker: when trimmed, the line must be exactly "// ^"
        if ($0 ~ /^[[:space:]]*\/\/[[:space:]]*\^[[:space:]]*$/) {
            inBlock = 0;
            printPlaceholder();
            next;
        }
        # If inside a marked block, print the line.
        if (inBlock) {
            print $0;
            lastWasPlaceholder = 0;
        }
    }
    ' "$file"
}

# Allow direct execution for testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
         echo "Usage: $0 <file_path>" >&2
         exit 1
    fi
    filter_substring_markers "$1"
fi
