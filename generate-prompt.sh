#!/bin/bash
set -euo pipefail

##########################################
# generate-prompt.sh
#
# This script finds the unique Swift file that contains a
# TODO instruction (either “// TODO: - ” or “// TODO: ChatGPT: ”),
# processes it along with related type definitions in the repository,
# and then assembles a ChatGPT prompt that is copied to the clipboard.
#
# Usage:
#   generate-prompt.sh [--slim] [--singular] [--exclude <filename>] [--verbose] [--exclude <another_filename>] ...
#
# Options:
#   --slim         Only include the file that contains the TODO instruction
#                  and “model” files. In slim mode, files whose names contain
#                  keywords such as “ViewController”, “Manager”, “Presenter”,
#                  “Configurator”, “Router”, “DataSource”, “Delegate”, or “View”
#                  are excluded.
#   --singular     Only include the Swift file that contains the TODO instruction.
#   --exclude      Exclude any file whose basename matches the provided filename.
#   --verbose      Enable verbose console logging for debugging purposes.
#
# It sources the following components:
#   - find-prompt-instruction.sh       : Locates the unique Swift file with the TODO.
#   - extract-instruction-content.sh   : Extracts the TODO instruction content from the file.
#   - extract-types.sh                 : Extracts potential type names from a Swift file.
#   - find-definition-files.sh         : Finds Swift files containing definitions for the types.
#   - filter-files.sh                  : Filters the found files in slim mode.
#   - exclude-files.sh                 : Filters out files matching user-specified exclusions.
#   - assemble-prompt.sh               : Assembles the final prompt and copies it to the clipboard.
#   - get-git-root.sh                  : Determines the Git repository root.
#   - get-package-root.sh               : Determines the package root (if any) for a given file.
#   - filter-files-singular.sh         : (New) Returns only the file that contains the TODO.
##########################################

# Process optional parameters.
SLIM=false
SINGULAR=false
VERBOSE=false
EXCLUDES=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --slim)
            SLIM=true
            shift
            ;;
        --singular)
            SINGULAR=true
            shift
            ;;
        --exclude)
            if [ -n "${2:-}" ]; then
                EXCLUDES+=("$2")
                shift 2
            else
                echo "Usage: $0 [--slim] [--singular] [--exclude <filename>]" >&2
                exit 1
            fi
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        *)
            echo "Usage: $0 [--slim] [--singular] [--exclude <filename>]" >&2
            exit 1
            ;;
    esac
done

# Export VERBOSE so that helper scripts can use it for debugging output.
export VERBOSE

# Save the directory where you invoked the script.
CURRENT_DIR="$(pwd)"

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source external components from SCRIPT_DIR.
source "$SCRIPT_DIR/find-prompt-instruction.sh"
source "$SCRIPT_DIR/extract-instruction-content.sh"
source "$SCRIPT_DIR/assemble-prompt.sh"
source "$SCRIPT_DIR/get-git-root.sh"
source "$SCRIPT_DIR/get-package-root.sh"

if [ "$SINGULAR" = true ]; then
    source "$SCRIPT_DIR/filter-files-singular.sh"
else
    source "$SCRIPT_DIR/extract-types.sh"
    source "$SCRIPT_DIR/find-definition-files.sh"
    source "$SCRIPT_DIR/filter-files.sh"      # Slim mode filtering.
    source "$SCRIPT_DIR/exclude-files.sh"       # Exclusion filtering.
fi

echo "--------------------------------------------------"

# Change back to the directory where the command was invoked.
cd "$CURRENT_DIR"

# Determine the Git repository root.
GIT_ROOT=$(get-git-root) || exit 1
echo "Git root: $GIT_ROOT"

# Move to the repository root.
cd "$GIT_ROOT"

# Use the external component to locate the file with the TODO instruction.
FILE_PATH=$(find-prompt-instruction "$GIT_ROOT") || exit 1
echo "Found exactly one instruction in $FILE_PATH"

# --- Determine Package Scope ---
# (Source the package root helper above.)
# If the TODO file is in a package (i.e. an ancestor directory contains Package.swift),
# use that package as the search scope; otherwise, use the entire Git repository.
PACKAGE_ROOT=$(get-package-root "$FILE_PATH" || true)
if [ -n "$PACKAGE_ROOT" ]; then
    echo "Found package root: $PACKAGE_ROOT"
    SEARCH_ROOT="$PACKAGE_ROOT"
else
    SEARCH_ROOT="$GIT_ROOT"
fi
# --- End Package Scope ---

# Extract the instruction content from the file.
INSTRUCTION_CONTENT=$(extract-instruction-content "$FILE_PATH")

if [ "$SINGULAR" = true ]; then
    echo "Singular mode enabled: only including the TODO file"
    FOUND_FILES=$(filter_files_singular "$FILE_PATH")
else
    # Extract potential type names from the Swift file.
    TYPES_FILE=$(extract-types "$FILE_PATH")
    
    # Find Swift files containing definitions for the types.
    FOUND_FILES=$(find-definition-files "$TYPES_FILE" "$SEARCH_ROOT")
    
    # Ensure the chosen TODO file is included in the found files.
    echo "$FILE_PATH" >> "$FOUND_FILES"
    
    # If slim mode is enabled, filter the FOUND_FILES list.
    if [ "$SLIM" = true ]; then
         echo "Slim mode enabled: filtering files to include only the TODO file and model files..."
         FOUND_FILES=$(filter-files_for_slim_mode "$FILE_PATH" "$FOUND_FILES")
    fi
    
    # If any exclusions were specified, filter them out.
    if [ "${#EXCLUDES[@]}" -gt 0 ]; then
         echo "Excluding files matching: ${EXCLUDES[*]}"
         FOUND_FILES=$(filter_excluded_files "$FOUND_FILES" "${EXCLUDES[@]}")
    fi
fi

# Register a trap to clean up temporary files.
cleanup_temp_files() {
    [[ -n "${TYPES_FILE:-}" ]] && rm -f "$TYPES_FILE"
    [[ -n "${FOUND_FILES:-}" ]] && rm -f "$FOUND_FILES"
}
trap cleanup_temp_files EXIT

echo "--------------------------------------------------"
if [ "${SINGULAR}" = false ]; then
    echo "Types found:"
    cat "$TYPES_FILE"
    echo "--------------------------------------------------"
fi

echo "Files (final list):"
sort "$FOUND_FILES" | uniq | while read -r file_path; do
    basename "$file_path"
done

# Assemble the final clipboard content and copy it to the clipboard.
FINAL_CLIPBOARD_CONTENT=$(assemble-prompt "$FOUND_FILES" "$INSTRUCTION_CONTENT")

echo "--------------------------------------------------"
echo
echo "Success:"
echo
echo "$INSTRUCTION_CONTENT"
echo
echo "--------------------------------------------------"
