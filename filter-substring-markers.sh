#!/bin/bash
# filter-substring-markers.sh
#
# This function processes a file for “substring markers.”
# Markers are defined as:
#   - An opening marker: a line that matches “// v” (optionally surrounded by whitespace)
#   - A closing marker: a line that matches “// ^”
#
# If the file does not contain both markers, the entire file is returned.
# Otherwise, only the text between each matching pair is output,
# and a placeholder comment (“// ... rest of file ...”) is inserted between regions
# (and at the very start if there’s content before the first marker).
#
# Usage: filter_substring_markers <file_path>
filter_substring_markers() {
    local file="$1"

    # If the file does not contain both markers, just output it in full.
    if ! grep -q '^[[:space:]]*//[[:space:]]*v[[:space:]]*$' "$file" || \
       ! grep -q '^[[:space:]]*//[[:space:]]*\^[[:space:]]*$' "$file"; then
        cat "$file"
        return 0
    fi

    awk '
    BEGIN {
        state = "OUTSIDE"
        placeholder = "// ... rest of file ..."
        printed_first_region = 0
    }
    {
        # Check for the opening marker (// v)
        if ($0 ~ /^[[:space:]]*\/\/[[:space:]]*v[[:space:]]*$/) {
            state = "INSIDE"
            # If this is not the very first marked region, print a placeholder line to indicate omitted text.
            if (printed_first_region)
                print placeholder
            else
                printed_first_region = 1
            next
        }
        # Check for the closing marker (// ^)
        if ($0 ~ /^[[:space:]]*\/\/[[:space:]]*\^[[:space:]]*$/) {
            state = "OUTSIDE"
            next
        }
        # Only print lines when inside a marked region.
        if (state == "INSIDE")
            print $0
    }
    ' "$file"
}
