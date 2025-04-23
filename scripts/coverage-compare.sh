#!/usr/bin/env bash
# Compare coverage on the current branch against `main`
# and leave the HTML report in .coverage/target/tarpaulin-report.html

set -euo pipefail

TARGET_DIR=".coverage/target"          # where Tarpaulin writes JSON + HTML
BASE_BRANCH="main"                     # baseline branch
HTML_FILE="$TARGET_DIR/tarpaulin-report.html"

branch=$(git rev-parse --abbrev-ref HEAD)
[[ $branch == "$BASE_BRANCH" ]] && {
  echo "Already on $BASE_BRANCH – nothing to compare."; exit 0; }

git stash -q --keep-index || true      # save un-committed edits

# 1. generate baseline on main
git checkout -q "$BASE_BRANCH"
mkdir -p "$TARGET_DIR"
cargo tarpaulin --workspace --out Json \
                --target-dir "$TARGET_DIR" \
                --output-dir "$TARGET_DIR"

# 2. switch back and restore
git checkout -q "$branch"
git stash pop -q || true

# 3. copy baseline into “previous run” cache
mkdir -p "$TARGET_DIR/tarpaulin"
cp "$TARGET_DIR/tarpaulin-report.json" \
   "$TARGET_DIR/tarpaulin/baseline.json"

# 4. run coverage on the feature branch
cargo tarpaulin --workspace --out Html \
                --target-dir "$TARGET_DIR" \
                --output-dir "$TARGET_DIR"

echo "Coverage report ready at $HTML_FILE"
echo "Open it in your browser to view the Change column."
