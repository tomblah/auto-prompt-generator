#!/usr/bin/env bash
# is-this-right.sh — gather diffs vs HEAD (including untracked files) and copy to the clipboard, but stay quiet.

set -euo pipefail

echo "Collecting changes and copying to clipboard…"

# 1. All tracked files that differ from HEAD (modified, added, renamed, etc.)
tracked=$(git diff HEAD --name-only)

# 2. Any brand‑new, untracked files (ignoring those matched by .gitignore)
untracked=$(git ls-files --others --exclude-standard)

# Bail out early if absolutely nothing has changed
if [[ -z "$tracked" && -z "$untracked" ]]; then
  echo "No changes found."
  exit 0
fi

# Build the clipboard payload
{
  # ⇒ Tracked changes (diffs)
  if [[ -n "$tracked" ]]; then
    while IFS= read -r file; do
      echo
      echo "===== Diff for $file ====="
      git diff HEAD -- "$file"
    done <<< "$tracked"
  fi

  # ⇒ Untracked files (show full content)
  if [[ -n "$untracked" ]]; then
    echo
    echo "===== Untracked files (not yet committed) ====="
    while IFS= read -r file; do
      echo
      echo "+++ $file (untracked)"
      cat -- "$file"
    done <<< "$untracked"
  fi

  echo
  echo "---"
  echo "Is this right?"
} | pbcopy

echo "Done."
