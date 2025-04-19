#!/usr/bin/env bash
# diff_each.sh — output per-file diffs for git changes
# Usage:
#   ./diff_each.sh [--staged|--unstaged|--all]
#
#   --staged   → show each staged file’s diff vs HEAD
#   --unstaged → show each unstaged file’s diff vs index
#   --all      → show each file changed vs HEAD (default)

set -euo pipefail

# Read mode argument (default to "all") and strip any leading dashes
mode="${1:-all}"
mode="${mode#--}"

declare diff_cmd files
case "$mode" in
  staged)
    diff_cmd=(git diff --cached --)
    files=$(git diff --cached --name-only)
    ;;
  unstaged)
    diff_cmd=(git diff --)
    files=$(git diff --name-only)
    ;;
  all)
    diff_cmd=(git diff HEAD --)
    files=$(git diff HEAD --name-only)
    ;;
  *)
    echo "Usage: $0 [--staged|--unstaged|--all]"
    exit 1
    ;;
esac

if [ -z "$files" ]; then
  echo "No changes found for mode '$mode'."
  exit 0
fi

# Output each file's diff under its own header
while IFS= read -r file; do
  echo
  echo "===== Diff for $file ====="
  "${diff_cmd[@]}" "$file"
done <<< "$files"
