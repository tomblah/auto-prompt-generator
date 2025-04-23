#!/usr/bin/env bash
# code-review.sh
#
# Copy a self-contained “review bundle” to the clipboard:
#   • All commits on the current branch that aren’t on main
#     (oldest-first) – subject line & body.
#   • A file-by-file diff against origin/main for all tracked changes.
#   • Full contents of brand-new, untracked files.
#   • A trailing question that asks for a code review.
#
# Works quietly; exits non-zero on error.

set -euo pipefail

# You can change this if your primary branch is not called “main”.
MAIN_BRANCH=${MAIN_BRANCH:-origin/main}

echo "Collecting changes relative to $MAIN_BRANCH and copying to clipboard…"

##############################################################################
# 1 Figure out what’s different from main
##############################################################################
tracked=$(git diff --name-only "$MAIN_BRANCH"...HEAD)
untracked=$(git ls-files --others --exclude-standard)

# Abort if absolutely nothing has changed.
if [[ -z "$tracked" && -z "$untracked" ]]; then
  echo "No changes compared with $MAIN_BRANCH."
  exit 0
fi

##############################################################################
# 2 Grab commit messages that aren’t on main
##############################################################################
commit_log=$(
  git log --reverse --pretty=format:'[%h] %s%n%n%b%n' "$MAIN_BRANCH"..HEAD
  # (reverse for chronological order)
)

##############################################################################
# 3 Assemble clipboard payload
##############################################################################
{
  echo "### Commits since $MAIN_BRANCH"
  echo
  if [[ -n "$commit_log" ]]; then
    echo "$commit_log"
  else
    echo "_No new commits – working tree only_"
  fi

  ###########################################################################
  # Tracked file diffs
  ###########################################################################
  if [[ -n "$tracked" ]]; then
    echo
    echo "### File-by-file diff vs $MAIN_BRANCH"
    while IFS= read -r file; do
      echo
      echo "===== Diff for $file ====="
      git diff "$MAIN_BRANCH"...HEAD -- "$file"
    done <<< "$tracked"
  fi

  ###########################################################################
  # Untracked files
  ###########################################################################
  if [[ -n "$untracked" ]]; then
    echo
    echo "### Untracked files (not yet committed)"
    while IFS= read -r file; do
      echo
      echo "+++ $file (untracked)"
      cat -- "$file"
    done <<< "$untracked"
  fi

  ###########################################################################
  # Closing prompt
  ###########################################################################
  echo
  echo "---"
  echo "Can you conduct a code review of the above code and let me know"
  echo "if it's a well-constructed solution and highlight any issues?"
} | pbcopy

echo "Done – review bundle copied to clipboard."
