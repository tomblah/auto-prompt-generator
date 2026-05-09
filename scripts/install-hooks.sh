#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

source_dir="$repo_root/.githooks"
hooks_dir="$(git rev-parse --git-path hooks)"

mkdir -p "$hooks_dir"

for hook in "$source_dir"/*; do
  [[ -f "$hook" ]] || continue
  install -m 755 "$hook" "$hooks_dir/$(basename "$hook")"
done

echo "Installed Git hooks from .githooks into $hooks_dir"
