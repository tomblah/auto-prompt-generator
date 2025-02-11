#!/bin/bash
set -euo pipefail

##########################################
# meta-context.sh
#
# This script collects the contents of all .sh and README* files,
# and optionally .bats files (if --include-tests or --tests-only is passed)
# in the repository (excluding itself and any files in the Legacy or MockFiles folders)
# and copies them to the clipboard.
#
# Usage:
#   ./meta-context.sh [--include-tests] [--tests-only]
#
# When the --include-tests option is used, .bats files will be included along with .sh and README* files.
# When the --tests-only option is used, only .bats files will be included and a different final message is appended.
#
# Before each file's content, a header is added in the following format:
#
#   The contents of <filename> is as follows:
#
# At the very end of the prompt, a custom message is appended:
#
#   (Normal run:)
#   "I'm improving the generate-prompt.sh script (see README above for more context). I'm trying to keep generate-prompt.sh as thin as possible, so try not to propose solutions that edit it unless where it makes obvious sense to, e.g. for parsing options. But if there is an easy solution to create another file, or edit another existing file, let's prefer that."
#
#   (--tests-only:)
#   "Can you look through these tests and add unit tests to cover the functionality we've added"
#
# The final prompt is then copied to the clipboard using pbcopy.
##########################################

# Parse command-line options
INCLUDE_TESTS=false
TESTS_ONLY=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --include-tests)
            INCLUDE_TESTS=true
            shift
            ;;
        --tests-only)
            TESTS_ONLY=true
            shift
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Ensure that --include-tests and --tests-only are not used together.
if $INCLUDE_TESTS && $TESTS_ONLY; then
  echo "Error: Cannot use --include-tests and --tests-only together." >&2
  exit 1
fi

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Optionally, determine the repository root (assumes you are in a Git repository).
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR")
cd "$REPO_ROOT"

# Build the find command based on the options provided.
if $TESTS_ONLY; then
    echo "Including only .bats test files in the context."
    files=$(find . -type f -iname "*.bats" \
            -not -name "meta-context.sh" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
elif $INCLUDE_TESTS; then
    echo "Including .bats files along with .sh and README* files in the context."
    files=$(find . -type f \( -iname "*.sh" -o -iname "README*" -o -iname "*.bats" \) \
            -not -name "meta-context.sh" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
else
    files=$(find . -type f \( -iname "*.sh" -o -iname "README*" \) \
            -not -name "meta-context.sh" \
            -not -path "*/Legacy/*" \
            -not -path "*/MockFiles/*")
fi

echo "--------------------------------------------------"
echo "Files to include in the meta-context prompt:"
for file in $files; do
    echo " - $file"
done
echo "--------------------------------------------------"

# Create a temporary file to accumulate the context.
temp_context=$(mktemp)

# Loop over each file and append a header and its content.
for file in $files; do
    {
      echo "--------------------------------------------------"
      echo "The contents of $(basename "$file") is as follows:"
      echo "--------------------------------------------------"
      cat "$file"
      echo -e "\n"
    } >> "$temp_context"
done

# Append the final custom message based on the option provided.
if $TESTS_ONLY; then
    {
      echo "--------------------------------------------------"
      echo -e "Can you look through these tests and add unit tests to cover the functionality we've added in.\n\nLet's lean towards appending to existing files where it makes sense to do so. And be sure to echo out the entire test file with the added test cases."
    } >> "$temp_context"
else
    {
      echo "--------------------------------------------------"
      echo -e "I'm improving the generate-prompt.sh script (see README above for more context). I'm trying to keep generate-prompt.sh as thin as possible, so try not to propose solutions that edit it unless where it makes obvious sense to, e.g. for parsing options. But if there is an easy solution to create another file, or edit another existing file, let's prefer that.\n\n"
    } >> "$temp_context"
fi

# Copy the final context to the clipboard using pbcopy (macOS).
# For Linux, you might use: xclip -selection clipboard or xsel --clipboard --input.
cat "$temp_context" | pbcopy

echo "--------------------------------------------------"
echo "Success: Meta context has been copied to the clipboard."
echo "--------------------------------------------------"

# Clean up the temporary file.
rm "$temp_context"
