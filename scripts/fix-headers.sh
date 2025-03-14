#!/bin/bash
# Run this script from your project root

project_root=$(pwd)

# Process every .rs file in the project
find . -name "*.rs" | while read -r file; do
    # Remove a leading "./" if present to get the relative path.
    relpath="${file#./}"
    correct_header="// $relpath"

    # Get the first line of the file
    first_line=$(head -n 1 "$file")

    # Determine if the file already starts with a comment.
    if [[ "$first_line" =~ ^// ]]; then
        if [ "$first_line" != "$correct_header" ]; then
            echo "$file: updated header"
        else
            echo "$file: header is correct"
        fi
    else
        echo "$file: added header"
    fi

    # Create a temporary file for building the new content.
    temp=$(mktemp)

    # Write the correct header to the temp file.
    echo "$correct_header" > "$temp"
    # Write exactly one blank line after the header.
    echo "" >> "$temp"

    # If the file originally started with a comment header, skip its first line.
    # Otherwise, use the entire file.
    if [[ "$first_line" =~ ^// ]]; then
        rest=$(tail -n +2 "$file")
    else
        rest=$(cat "$file")
    fi

    # Remove any leading blank lines from the remaining content.
    # The sed command deletes all lines at the start that are empty until a non-empty line is found.
    cleaned_rest=$(echo "$rest" | sed '/./,$!d')

    # Append the cleaned remainder to the temp file.
    echo "$cleaned_rest" >> "$temp"

    # Overwrite the original file with the new content.
    mv "$temp" "$file"
done
