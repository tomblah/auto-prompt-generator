#!/bin/bash
set -euo pipefail

##########################################
# meta-context.sh
#
# This script collects the contents of all .sh, README* files,
# and optionally .bats files (if --include-tests is passed)
# in the repository (excluding itself and any files in the Legacy or MockFiles folders)
# and copies them to the clipboard.
#
# Usage:
#   ./meta-context.sh [--include-tests]
#
# When the --include-tests option is used, .bats files will also be included.
#
# Before each file's content, a header is added in the following format:
#
#   The contents of <filename> is as follows:
#
# At the very end of the prompt, a custom message is appended:
#
#   I'm improving the generate-prompt.sh script (see README above for more context)...
#
# The final prompt is then copied to the clipboard using pbcopy.
##########################################

# Parse command-line options
INCLUDE_TESTS=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --include-tests)
            INCLUDE_TESTS=true
            shift
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Optionally, determine the repository root (assumes you are in a Git repository).
REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || echo "$SCRIPT_DIR")
cd "$REPO_ROOT"

# Build the find command based on whether tests should be included.
if $INCLUDE_TESTS; then
    echo "Including .bats files in the context."
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

# Append the custom header message at the end without a final dashed line.
{
  echo "--------------------------------------------------"
  echo -e "I'm improving the generate-prompt.sh script (see README above for more context). I'm trying to keep generate-prompt.sh as thin as possible, so try not to propose solutions that edit it unless where it makes obvious sense to, e.g. for parsing options. But if there is an easy solution to create another file, or edit another existing file, let's prefer that.\n\n"
} >> "$temp_context"

# Copy the final context to the clipboard using pbcopy (macOS).
# For Linux, you might use: xclip -selection clipboard or xsel --clipboard --input.
cat "$temp_context" | pbcopy

echo "--------------------------------------------------"
echo "Success: Meta context has been copied to the clipboard."
echo "--------------------------------------------------"

# Clean up the temporary file.
rm "$temp_context"
