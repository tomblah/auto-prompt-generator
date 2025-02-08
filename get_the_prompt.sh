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
# Usage:
#   get-the-prompt.sh [--slim]
#
# Options:
#   --slim    Only include the file that contains the TODO instruction
#             and “model” files. In slim mode, files whose names contain
#             keywords such as “ViewController”, “Manager”, “Presenter”,
#             “Configurator”, “Router”, “DataSource”, “Delegate”, or “View”
#             are excluded.
#
# It sources the following components:
#   - find_prompt_instruction.sh       : Locates the unique Swift file with the TODO.
#   - extract_instruction_content.sh   : Extracts the TODO instruction content from the file.
#   - extract_types.sh                 : Extracts potential type names from a Swift file.
#   - find_definition_files.sh         : Finds Swift files containing definitions for the types.
#   - filter_files.sh                  : Filters the found files in slim mode.
#   - assemble_prompt.sh               : Assembles the final prompt and copies it to the clipboard.
#   - get_git_root.sh                  : Determines the Git repository root.
##########################################

# Process optional parameters.
SLIM=false
if [ "$#" -gt 0 ]; then
    if [ "$1" == "--slim" ]; then
        SLIM=true
    else
        echo "Usage: $0 [--slim]" >&2
        exit 1
    fi
fi

# Save the directory where you invoked the script.
CURRENT_DIR="$(pwd)"

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source external components from SCRIPT_DIR.
source "$SCRIPT_DIR/find_prompt_instruction.sh"
source "$SCRIPT_DIR/extract_instruction_content.sh"
source "$SCRIPT_DIR/extract_types.sh"
source "$SCRIPT_DIR/find_definition_files.sh"
source "$SCRIPT_DIR/filter_files.sh"   # New component for slim mode filtering.
source "$SCRIPT_DIR/assemble_prompt.sh"
source "$SCRIPT_DIR/get_git_root.sh"

echo "--------------------------------------------------"

# Change back to the directory where the command was invoked.
cd "$CURRENT_DIR"

# Determine the Git repository root.
GIT_ROOT=$(get_git_root) || exit 1
echo "Git root: $GIT_ROOT"

# Move to the repository root.
cd "$GIT_ROOT"

# Use the external component to locate the file with the TODO instruction.
FILE_PATH=$(find_prompt_instruction "$GIT_ROOT") || exit 1
echo "Found exactly one instruction in $FILE_PATH"

# Extract the instruction content from the file.
INSTRUCTION_CONTENT=$(extract_instruction_content "$FILE_PATH")

# Extract potential type names from the Swift file.
TYPES_FILE=$(extract_types "$FILE_PATH")

# Find Swift files containing definitions for the types.
FOUND_FILES=$(find_definition_files "$TYPES_FILE" "$GIT_ROOT")

# If slim mode is enabled, filter the FOUND_FILES list.
if [ "$SLIM" = true ]; then
    echo "Slim mode enabled: filtering files to include only the TODO file and model files..."
    FOUND_FILES=$(filter_files_for_slim_mode "$FILE_PATH" "$FOUND_FILES")
fi

# Register a trap to clean up temporary files.
cleanup_temp_files() {
    [[ -n "${TYPES_FILE:-}" ]] && rm -f "$TYPES_FILE"
    [[ -n "${FOUND_FILES:-}" ]] && rm -f "$FOUND_FILES"
}
trap cleanup_temp_files EXIT

echo "--------------------------------------------------"
echo "Types found:"
cat "$TYPES_FILE"
echo "--------------------------------------------------"

echo "Files:"
sort "$FOUND_FILES" | uniq | while read -r file_path; do
    basename "$file_path"
done

# Assemble the final clipboard content and copy it to the clipboard.
FINAL_CLIPBOARD_CONTENT=$(assemble_prompt "$FOUND_FILES" "$INSTRUCTION_CONTENT")

echo "--------------------------------------------------"
echo
echo "Success:"
echo
echo "$INSTRUCTION_CONTENT"
echo
echo "--------------------------------------------------"
