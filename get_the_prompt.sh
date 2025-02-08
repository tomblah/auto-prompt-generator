#!/bin/bash
set -euo pipefail

##########################################
# get-the-prompt.sh
#
# This script finds the unique Swift file that contains a
# TODO instruction (either “// TODO: - ” or “// TODO: ChatGPT: ”),
# processes it along with related type definitions in the repository,
# and then assembles a ChatGPT prompt that is copied to the clipboard.
#
# It sources the find_prompt_instruction.sh component to
# isolate the logic for finding the TODO instruction.
##########################################

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the find_prompt_instruction component.
source "$SCRIPT_DIR/find_prompt_instruction.sh"

echo "--------------------------------------------------"

# Change to the directory of the script.
cd "$SCRIPT_DIR"

# Determine the root directory of the Git repository.
GIT_ROOT=$(git rev-parse --show-toplevel 2>/dev/null)
if [ -z "$GIT_ROOT" ]; then
    echo "Error: Not a git repository." >&2
    exit 1
fi
echo "Git root: $GIT_ROOT"

# Move to the repository root.
cd "$GIT_ROOT"

# Use the external component to locate the file with the TODO instruction.
FILE_PATH=$(find_prompt_instruction "$GIT_ROOT") || exit 1
echo "Found exactly one instruction in $FILE_PATH"

# Extract the instruction content from the file.
# (This extracts the first matching line from the file.)
INSTRUCTION_CONTENT=$(grep -E '// TODO: (ChatGPT: |- )' "$FILE_PATH" | head -n 1 | sed 's/^[[:space:]]*//')

# Temporary files for intermediate steps.
TEMP_FILE_PREPROCESS="/tmp/filtered_swift_file_preprocess.tmp"
TEMP_FILE_STAGE0="/tmp/filtered_swift_file_stage0.tmp"
TEMP_FILE_STAGE1="/tmp/filtered_swift_file_stage1.tmp"
TEMP_FILE_STAGE2="/tmp/filtered_swift_file_stage2.tmp"
TYPES_FILE="/tmp/swift_types.txt"
FOUND_FILES="/tmp/found_swift_files.tmp"

# Ensure FOUND_FILES is empty at the start.
> "$FOUND_FILES"

# Start by adding the file containing the instruction to FOUND_FILES.
echo "$FILE_PATH" >> "$FOUND_FILES"

# --- Preprocessing the Swift file ---

# Preprocess: Replace all non-alphanumeric characters with whitespace.
awk '{gsub(/[^a-zA-Z0-9]/, " "); print}' "$FILE_PATH" > "$TEMP_FILE_PREPROCESS"

# Stage 0: Trim leading spaces from each line.
awk '{$1=$1; print}' "$TEMP_FILE_PREPROCESS" > "$TEMP_FILE_STAGE0"

# Stage 1: Remove import lines.
awk '!/^import /' "$TEMP_FILE_STAGE0" > "$TEMP_FILE_STAGE1"

# Stage 2: Remove comment lines.
awk '!/^\/\//' "$TEMP_FILE_STAGE1" > "$TEMP_FILE_STAGE2"

# Stage 3: Extract potential type names (classes, structs, enums, etc.)
awk '
{
    for(i = 1; i <= NF; i++) {
        if ($i ~ /^[A-Z][A-Za-z0-9]+$/) {
            print $i
        } else if ($i ~ /\[[A-Z][A-Za-z0-9]+\]/) {
            gsub(/\[|\]/, "", $i)
            print $i
        }
    }
}' "$TEMP_FILE_STAGE2" | sort | uniq > "$TYPES_FILE"

echo "--------------------------------------------------"
echo "Types found:"
cat "$TYPES_FILE"
echo "--------------------------------------------------"

# --- Finding Definition Files for the Types ---

# For each type found, search for its definition in Swift files (class, struct, enum, etc.)
while read -r TYPE; do
    grep -rwlE --include="*.swift" "\\b(class|struct|enum|protocol|typealias)\\s+$TYPE\\b" "$GIT_ROOT" >> "$FOUND_FILES" || true
done < "$TYPES_FILE"

echo "Files:"
sort "$FOUND_FILES" | uniq | while read -r file_path; do
    basename "$file_path"
done

# --- Assembling the Final Clipboard Content ---

# Ensure we're working with a unique list of files.
UNIQUE_FOUND_FILES=$(sort "$FOUND_FILES" | uniq)

# Initialize an empty variable to accumulate the content.
CLIPBOARD_CONTENT=""

while read -r file_path; do
    FILE_BASENAME=$(basename "$file_path")
    FILE_CONTENT=$(cat "$file_path")
    CLIPBOARD_CONTENT+="The contents of $FILE_BASENAME is as follows:\n\n$FILE_CONTENT\n\n--------------------------------------------------\n"
done <<< "$UNIQUE_FOUND_FILES"

# Modify the clipboard content: Replace "// TODO: - " with "// TODO: ChatGPT: "
MODIFIED_CLIPBOARD_CONTENT=$(echo -e "$CLIPBOARD_CONTENT" | sed 's/\/\/ TODO: - /\/\/ TODO: ChatGPT: /g')

# Append the instruction content to the final clipboard content.
FINAL_CLIPBOARD_CONTENT="$MODIFIED_CLIPBOARD_CONTENT\n\n$INSTRUCTION_CONTENT"

# Copy the final content to the clipboard using pbcopy.
echo -e "$FINAL_CLIPBOARD_CONTENT" | pbcopy

echo "--------------------------------------------------"
echo
echo "Success:"
echo
echo "$INSTRUCTION_CONTENT"
echo
echo "--------------------------------------------------"
