#!/bin/bash
# filter-substring-markers.sh
#
# This function processes a file for “substring markers.”
# Markers are defined as:
#   - An opening marker: a line that matches “// v” (optionally surrounded by whitespace)
#   - A closing marker: a line that matches “// ^”
#
# When both markers are found in proper order, only the content between them is output.
# Additionally, if there is any omitted content before the first marker, between marked regions,
# or after the last marker, a placeholder is inserted. The placeholder consists of a blank line,
# then the line "// ...", then another blank line.
#
# If the markers aren’t both found (or are not in proper order), the entire file is output.
#
# Usage: filter_substring_markers <file_path>
filter_substring_markers() {
    local file="$1"
    awk '
    {
        lines[NR] = $0;
    }
    END {
        first_open = 0;
        last_close = 0;
        # Identify the first opening marker and the last closing marker.
        for (i = 1; i <= NR; i++) {
            if (lines[i] ~ /^[[:space:]]*\/\/[[:space:]]*v[[:space:]]*$/ && first_open == 0) {
                first_open = i;
            }
            if (lines[i] ~ /^[[:space:]]*\/\/[[:space:]]*\^[[:space:]]*$/) {
                last_close = i;
            }
        }
        # If markers are not found properly, output the entire file.
        if (first_open == 0 || last_close == 0 || first_open >= last_close) {
            for (i = 1; i <= NR; i++) {
                print lines[i];
            }
            exit;
        }
        
        placeholder = "\n// ...\n";
        
        # If there is content before the first opening marker, print the placeholder.
        if (first_open > 1) {
            print placeholder;
        }
        
        in_region = 0;
        # Process lines between first_open and last_close.
        for (i = first_open; i <= last_close; i++) {
            # When encountering an opening marker, start printing (if not already printing).
            if (lines[i] ~ /^[[:space:]]*\/\/[[:space:]]*v[[:space:]]*$/) {
                in_region = 1;
                next;
            }
            # When encountering a closing marker, stop printing.
            if (lines[i] ~ /^[[:space:]]*\/\/[[:space:]]*\^[[:space:]]*$/) {
                in_region = 0;
                # Look ahead: if there is another opening marker later between i and last_close, insert a placeholder.
                found_next = 0;
                for (j = i+1; j <= last_close; j++) {
                    if (lines[j] ~ /^[[:space:]]*\/\/[[:space:]]*v[[:space:]]*$/) {
                        found_next = 1;
                        break;
                    }
                }
                if (found_next) {
                    print placeholder;
                }
                next;
            }
            if (in_region == 1) {
                print lines[i];
            }
        }
        # If there is content after the last closing marker, print the placeholder.
        if (last_close < NR) {
            print placeholder;
        }
    }
    ' "$file"
}
