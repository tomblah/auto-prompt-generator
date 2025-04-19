#!/usr/bin/env bash
# is-this-right.sh — gather diffs vs HEAD, copy to pasteboard, but stay quiet

set -euo pipefail

echo "Collecting changes and copying to clipboard…"

# Find all changed files vs HEAD
files=$(git diff HEAD --name-only)

if [ -z "$files" ]; then
  echo "No changes found."
  exit 0
fi

# Build the full diff payload and pipe it into pbcopy
{
  while IFS= read -r file; do
    echo
    echo "===== Diff for $file ====="
    git diff HEAD -- "$file"
  done <<< "$files"

  echo
  echo "---"
  echo "Is this right?"
} | pbcopy

echo "Done."
