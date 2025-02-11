#!/bin/bash
# extract-enclosing-type.sh
#
# This helper defines a function to extract the enclosing type
# (class, struct, or enum) from a given Swift file. It scans until
# it reaches the TODO instruction and returns the last encountered type.
#
# If no enclosing type is found (i.e. if the TODO is outside any type),
# it falls back to using the file’s basename (without the .swift extension)
# as the type name.
#
# Usage (when sourcing):
#   extract_enclosing_type <swift_file>
#
# When executed directly, it performs a quick test.

extract_enclosing_type() {
    local swift_file="$1"
    local extracted_type

    extracted_type=$(awk '
       BEGIN { regex="(class|struct|enum)[[:space:]]+" }
       /\/\/ TODO: -/ { exit }
       {
           pos = match($0, regex)
           if (pos > 0) {
               # Get the substring immediately after the matched keyword.
               type_line = substr($0, RSTART+RLENGTH)
               # Split the remainder by any non-alphanumeric/underscore character.
               split(type_line, arr, /[^A-Za-z0-9_]/)
               if (arr[1] != "") { type = arr[1] }
           }
       }
       END { if (type != "") print type }
    ' "$swift_file")

    # Fallback: if no type was found, use the file's basename (without .swift)
    if [ -z "$extracted_type" ]; then
         extracted_type=$(basename "$swift_file" .swift)
    fi

    echo "$extracted_type"
}

# Allow direct execution for testing.
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    if [ $# -ne 1 ]; then
         echo "Usage: $0 <swift_file>" >&2
         exit 1
    fi
    extract_enclosing_type "$1"
fi
