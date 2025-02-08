#!/bin/bash

echo "--------------------------------------------------"

cd "$(dirname "$0")"

# Find the root directory of the Git repository
GIT_ROOT=$(git rev-parse --show-toplevel)
TARGET_DIR="$GIT_ROOT/Packages/FullScreenMediaViewer"
echo "Target directory: $TARGET_DIR"

# Ensure the target directory exists
if [ ! -d "$TARGET_DIR" ]; then
    echo "Error: Target directory does not exist: $TARGET_DIR"
    exit 1
fi

# Search for Swift files containing the specific TODO comments, excluding MockData.swift
MATCHING_LINES=$(grep -rnE "// TODO: - |// TODO: ChatGPT: " --include "*.swift" --exclude "*MockData.swift" "$TARGET_DIR")
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
    echo "$MATCHING_LINES" | cut -d: -f3- | sed 's/^[[:space:]]*//'
    echo
    echo "--------------------------------------------------"
    exit 1
else
    FILE_PATH=$(echo "$MATCHING_LINES" | head -n 1 | cut -d: -f1)
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

> "$FOUND_FILES"

echo "$FILE_PATH" >> "$FOUND_FILES"

awk '{gsub(/[^a-zA-Z0-9]/, " "); print}' "$FILE_PATH" > "$TEMP_FILE_PREPROCESS"
awk '{$1=$1; print}' "$TEMP_FILE_PREPROCESS" > "$TEMP_FILE_STAGE0"
awk '!/^import /' "$TEMP_FILE_STAGE0" > "$TEMP_FILE_STAGE1"
awk '!/^\/\//' "$TEMP_FILE_STAGE1" > "$TEMP_FILE_STAGE2"

awk '
{
    for(i = 1; i <= NF; i++) {
        if ($i ~ /^[A-Z][A-Za-z0-9]+$/) {
            print $i
        } else if ($i ~ /\[[A-Z][A-Za-z0-9]+\]/) {
            gsub(/\[|\]/, "", $i);
            print $i
        }
    }
}' "$TEMP_FILE_STAGE2" | sort | uniq > "$TYPES_FILE"

echo "--------------------------------------------------"
echo "Types found:"
cat "$TYPES_FILE"
echo "--------------------------------------------------"

while read -r TYPE; do
    grep -rwlE --include="*.swift" --exclude="*MockData.swift" "\\b(class|struct|enum|protocol|typealias)\\s+$TYPE\\b" "$TARGET_DIR" >> "$FOUND_FILES"
done < "$TYPES_FILE"

# Exclude MockData.swift from the final file list
grep -v "MockData.swift" "$FOUND_FILES" | sort | uniq > "$FOUND_FILES.tmp"
mv "$FOUND_FILES.tmp" "$FOUND_FILES"

echo "Files:"
sort "$FOUND_FILES" | uniq | while read -r FILE_PATH; do
    basename "$FILE_PATH"
done

UNIQUE_FOUND_FILES=$(sort "$FOUND_FILES" | uniq)
CLIPBOARD_CONTENT=""

while read -r FILE_PATH; do
    FILE_BASENAME=$(basename "$FILE_PATH")
    FILE_CONTENT=$(cat "$FILE_PATH")
    CLIPBOARD_CONTENT+="The contents of $FILE_BASENAME is as follows:\n\n$FILE_CONTENT\n\n--------------------------------------------------\n"
done <<< "$UNIQUE_FOUND_FILES"

MODIFIED_CLIPBOARD_CONTENT=$(echo -e "$CLIPBOARD_CONTENT" | sed 's/\/\/ TODO: - /\/\/ TODO: ChatGPT: /g')

FINAL_CLIPBOARD_CONTENT="$MODIFIED_CLIPBOARD_CONTENT

Can you do the TODO: ChatGPT: in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by // TODO: ChatGPT:"

echo -e "$FINAL_CLIPBOARD_CONTENT" | pbcopy
echo "--------------------------------------------------"
echo
echo Success:
echo
echo "$INSTRUCTION_CONTENT"
echo
echo "--------------------------------------------------"
