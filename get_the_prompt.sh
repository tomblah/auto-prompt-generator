#!/bin/bash

echo "--------------------------------------------------"

cd "$(dirname "$0")"

# Find the root directory of the Git repository
GIT_ROOT=$(git rev-parse --show-toplevel)
echo "Git root: $GIT_ROOT"

cd "$(git rev-parse --show-toplevel)"

# Search for Swift files containing the specific strings
MATCHING_LINES=$(grep -rnE "// TODO: - |// TODO: ChatGPT: " --include "*.swift" "$GIT_ROOT")
OCCURRENCE_COUNT=$(echo "$MATCHING_LINES" | grep -c .)

# Handle the cases based on the count of occurrences
if [ "$OCCURRENCE_COUNT" -eq 0 ]; then
    echo "--------------------------------------------------"
    echo
    echo "Error: No Swift files found containing '// TODO: - ' or '// TODO: ChatGPT: '"
    echo
    echo "--------------------------------------------------"
    exit 1
elif [ "$OCCURRENCE_COUNT" -gt 1 ]; then
    echo "--------------------------------------------------"
    echo
    echo "More than one instruction:"
    echo
    # Use cut to extract the content part of each matching line and sed to trim leading spaces
    echo "$MATCHING_LINES" | cut -d: -f3- | sed 's/^[[:space:]]*//'
    echo
    echo "--------------------------------------------------"
    exit 1
else
    # Extract the file path from the first matching line
    FILE_PATH=$(echo "$MATCHING_LINES" | head -n 1 | cut -d: -f1)
    # Save the instruction content for later use
    INSTRUCTION_CONTENT=$(echo "$MATCHING_LINES" | cut -d: -f3- | sed 's/^[[:space:]]*//')
    echo "Found exactly one instruction in $FILE_PATH"
fi

# Temporary files for intermediate steps and for storing found file paths
TEMP_FILE_PREPROCESS="/tmp/filtered_swift_file_preprocess.tmp"
TEMP_FILE_STAGE0="/tmp/filtered_swift_file_stage0.tmp"
TEMP_FILE_STAGE1="/tmp/filtered_swift_file_stage1.tmp"
TEMP_FILE_STAGE2="/tmp/filtered_swift_file_stage2.tmp"
TYPES_FILE="/tmp/swift_types.txt"
FOUND_FILES="/tmp/found_swift_files.tmp"

# Ensure FOUND_FILES is empty at the start
> "$FOUND_FILES"

echo "$FILE_PATH" >> "$FOUND_FILES"

# Preprocessing: Replace all non-alphanumeric characters with whitespaces
awk '{gsub(/[^a-zA-Z0-9]/, " "); print}' "$FILE_PATH" > "$TEMP_FILE_PREPROCESS"

# Stage 0: Trim leading spaces from each line
awk '{$1=$1; print}' "$TEMP_FILE_PREPROCESS" > "$TEMP_FILE_STAGE0"

# Stage 1: Filter out import lines from the result of Stage 0
awk '!/^import /' "$TEMP_FILE_STAGE0" > "$TEMP_FILE_STAGE1"

# Stage 2: Filter out comment lines from the result of Stage 1
awk '!/^\/\//' "$TEMP_FILE_STAGE1" > "$TEMP_FILE_STAGE2"

# Stage 3: Process the filtered file from Stage 2 to find and extract capitalized words (potential type names)
awk '
{
    # Look for words that start with a capital letter and are followed by one or more letters or numbers
    # Additionally, attempt to capture types that are part of array declarations
    for(i = 1; i <= NF; i++) {
        if ($i ~ /^[A-Z][A-Za-z0-9]+$/) {
            print $i
        }
        # Match and print types within square brackets (e.g., [TypeName])
        else if ($i ~ /\[[A-Z][A-Za-z0-9]+\]/) {
            gsub(/\[|\]/, "", $i); # Remove the square brackets
            print $i
        }
    }
}' "$TEMP_FILE_STAGE2" | sort | uniq > "$TYPES_FILE"
echo "--------------------------------------------------"

# Print a succinct list of the types found to the console
echo "Types found:"
cat "$TYPES_FILE"
echo "--------------------------------------------------"

# Cycle through each type in the list and search for its definition
while read -r TYPE; do
    # Use extended regex to search for class, struct, enum, protocol, and typealias definitions
    grep -rwlE --include="*.swift" "\\b(class|struct|enum|protocol|typealias)\\s+$TYPE\\b" "$GIT_ROOT" >> "$FOUND_FILES"
done < "$TYPES_FILE"

# Print the unique list of found files to the console, displaying only the base filenames
echo "Files:"
sort "$FOUND_FILES" | uniq | while read -r FILE_PATH; do
    basename "$FILE_PATH"
done


# Ensure we're working with unique list of files for this operation
UNIQUE_FOUND_FILES=$(sort "$FOUND_FILES" | uniq)

# Initialize an empty variable to accumulate the content
CLIPBOARD_CONTENT=""

# Read through each unique found file, display its contents, and accumulate the content
while read -r FILE_PATH; do
    FILE_BASENAME=$(basename "$FILE_PATH")
    FILE_CONTENT=$(cat "$FILE_PATH")
    
    # Accumulate the content for the clipboard
    CLIPBOARD_CONTENT+="The contents of $FILE_BASENAME is as follows:\n\n$FILE_CONTENT\n\n--------------------------------------------------\n"
done <<< "$UNIQUE_FOUND_FILES"

# Replace "// TODO: - "
MODIFIED_CLIPBOARD_CONTENT=$(echo -e "$CLIPBOARD_CONTENT" | sed 's/\/\/ TODO: - /\/\/ TODO: ChatGPT: /g')

# Append the specific string to the modified clipboard content
FINAL_CLIPBOARD_CONTENT="$MODIFIED_CLIPBOARD_CONTENT

Can you do the TODO: ChatGPT: in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by // TODO: ChatGPT:"

# Copy the final content to the clipboard
echo -e "$FINAL_CLIPBOARD_CONTENT" | pbcopy
echo "--------------------------------------------------"
echo
echo Success:
echo
echo "$INSTRUCTION_CONTENT"
echo
echo "--------------------------------------------------"
